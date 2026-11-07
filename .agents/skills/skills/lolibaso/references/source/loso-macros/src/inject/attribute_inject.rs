// Copyright (c) 2025
//
// SPDX-License-Identifier: MIT

use super::{
    codegen::{Construction, Dependencies, ExecutionMode, Input, Scope, generate_impl},
    parse::{Dependency, DependencyClassifier, parameter_clone},
};
use quote::{ToTokens, quote};
use syn::{GenericParam, Generics, Ident, ItemImpl, Type};

#[derive(Clone)]
pub struct InjectImplBlock {
    names: super::names::Names,
    self_ty: Type,
    type_generics: Generics,
    constructor: ConstructorFn,
    original_impl: ItemImpl,
}

#[derive(Clone)]
struct ConstructorFn {
    name: Ident,
    generics: Generics,
    params: Vec<ConstructorParam>,
    fallible: bool,
    asynchronous: bool,
}

type ConstructorParam = Dependency<Ident>;

fn returns_self_or_result(output: &syn::ReturnType) -> Option<bool> {
    let syn::ReturnType::Type(_, return_type) = output else {
        return None;
    };
    let Type::Path(type_path) = &**return_type else {
        return None;
    };

    if type_path.path.is_ident("Self") {
        return Some(false);
    }

    let return_type = type_path.path.segments.last()?;
    if return_type.ident.to_string().contains("Result") {
        Some(true)
    } else {
        None
    }
}

fn parse_param(
    param: &syn::FnArg,
    classifier: &DependencyClassifier,
) -> syn::Result<ConstructorParam> {
    let (param_name, param_ty, attrs) = match param {
        syn::FnArg::Typed(pat_type) => {
            let param_name = match &*pat_type.pat {
                syn::Pat::Ident(ident) => ident.ident.clone(),
                _ => {
                    return Err(syn::Error::new_spanned(
                        pat_type,
                        "Parameter must be a simple identifier",
                    ));
                }
            };
            (param_name, (*pat_type.ty).clone(), &pat_type.attrs)
        }
        syn::FnArg::Receiver(_) => {
            return Err(syn::Error::new_spanned(
                param,
                "Constructor cannot have self parameter",
            ));
        }
    };

    let instance_clone = parameter_clone(attrs)?;
    let kind = classifier.classify(
        &param_ty,
        instance_clone,
        "`instance_clone` is only supported on concrete constructor parameters; generic constructor parameters support no injection attributes",
    )?;

    Ok(Dependency {
        name: param_name,
        ty: param_ty,
        kind,
    })
}

impl InjectImplBlock {
    pub fn parse(item_impl: ItemImpl) -> syn::Result<Self> {
        if !matches!(&*item_impl.self_ty, syn::Type::Path(_)) {
            return Err(syn::Error::new_spanned(
                item_impl.self_ty,
                "Expected type path in impl block",
            ));
        }

        let names = super::names::Names::new(item_impl.to_token_stream());
        let self_ty = (*item_impl.self_ty).clone();

        let type_generics = item_impl.generics.clone();

        let mut methods = Vec::new();
        for item in &item_impl.items {
            if let syn::ImplItem::Fn(method) = item {
                methods.push(method);
            }
        }

        if methods.is_empty() {
            return Err(syn::Error::new_spanned(
                item_impl,
                "Impl block for injection must contain at least one method",
            ));
        }

        if methods.len() > 1 {
            return Err(syn::Error::new_spanned(
                &item_impl,
                format!(
                    "Impl block for injection must contain exactly one method (found {})",
                    methods.len()
                ),
            ));
        }

        let method = &methods[0];

        let fallible = returns_self_or_result(&method.sig.output).ok_or_else(|| {
            syn::Error::new_spanned(
                &method.sig.output,
                format!("Constructor must return `Self` or a type whose name contains `Result`, found: {}", quote::ToTokens::to_token_stream(&method.sig.output)),
            )
        })?;
        for param in &method.sig.generics.params {
            if !matches!(param, GenericParam::Type(_)) {
                return Err(syn::Error::new_spanned(
                    param,
                    "constructor lifetime and const generics are not supported by `inject`",
                ));
            }
        }

        let has_self = method
            .sig
            .inputs
            .iter()
            .any(|arg| matches!(arg, syn::FnArg::Receiver(_)));
        if has_self {
            return Err(syn::Error::new_spanned(
                &method.sig.inputs,
                "Constructor cannot have self parameter",
            ));
        }

        // Preserve classification by impl generics only; method generics still
        // participate in generated indices, not recursive dependency detection.
        let classifier = DependencyClassifier::new(&type_generics);
        let params = method
            .sig
            .inputs
            .iter()
            .map(|p| parse_param(p, &classifier))
            .collect::<syn::Result<Vec<_>>>()?;

        let mut clean_impl = item_impl.clone();

        for item in &mut clean_impl.items {
            if let syn::ImplItem::Fn(method) = item {
                for input in &mut method.sig.inputs {
                    if let syn::FnArg::Typed(pat_type) = input {
                        pat_type
                            .attrs
                            .retain(|attr| !attr.path().is_ident("instance_clone"));
                    }
                }
            }
        }

        Ok(Self {
            names,
            self_ty,
            type_generics,
            constructor: ConstructorFn {
                name: method.sig.ident.clone(),
                generics: method.sig.generics.clone(),
                params,
                fallible,
                asynchronous: method.sig.asyncness.is_some(),
            },
            original_impl: clean_impl,
        })
    }
}

impl InjectImplBlock {
    fn injection_generics(&self) -> (Generics, Vec<Ident>) {
        let mut generics = self.type_generics.clone();
        let mut constructor_types = Vec::new();

        for param in &self.constructor.generics.params {
            match param {
                GenericParam::Type(param) => {
                    constructor_types.push(param.ident.clone());
                    generics.params.push(GenericParam::Type(param.clone()));
                }
                // Already rejected while parsing the constructor.
                GenericParam::Lifetime(_) | GenericParam::Const(_) => {}
            }
        }
        if let Some(method_where) = &self.constructor.generics.where_clause {
            generics
                .make_where_clause()
                .predicates
                .extend(method_where.predicates.clone());
        }

        (generics, constructor_types)
    }

    fn generate(&self, scope: Scope, root: &syn::Path) -> proc_macro2::TokenStream {
        let (generics, constructor_types) = self.injection_generics();
        let mode = if self.constructor.asynchronous {
            ExecutionMode::Async
        } else {
            ExecutionMode::Sync
        };
        let input = Input {
            root,
            names: &self.names,
            self_ty: &self.self_ty,
            generics: &generics,
            constructor_types: &constructor_types,
            dependencies: Dependencies::Parameters(&self.constructor.params),
            construction: Construction::Constructor {
                name: &self.constructor.name,
                mode,
                fallible: self.constructor.fallible,
            },
        };
        let original_impl = &self.original_impl;
        let sync = (!self.constructor.asynchronous)
            .then(|| generate_impl(&input, ExecutionMode::Sync, scope));
        let asynchronous = generate_impl(&input, ExecutionMode::Async, scope);
        quote!(#original_impl #sync #asynchronous)
    }

    pub fn generate_inject(self, root: &syn::Path) -> proc_macro2::TokenStream {
        self.generate(Scope::Transient, root)
    }

    pub fn generate_inject_singleton(self, root: &syn::Path) -> proc_macro2::TokenStream {
        self.generate(Scope::Singleton, root)
    }
}

#[cfg(test)]
mod tests {
    use super::super::parse::DependencyKind as InjectParamKind;
    use super::*;
    use syn::parse_quote;

    #[test]
    fn preserves_result_name_recognition() {
        for (output, expected) in [
            (parse_quote!(-> Self), Some(false)),
            (parse_quote!(-> anyhow::Result<Self>), Some(true)),
            (parse_quote!(-> BuildResult<Self>), Some(true)),
            (parse_quote!(-> ResultLike), Some(true)),
            (parse_quote!(-> Option<Self>), None),
            (parse_quote!(-> Concrete), None),
            (parse_quote!(-> (Self,)), None),
            (syn::ReturnType::Default, None),
        ] {
            assert_eq!(returns_self_or_result(&output), expected);
        }
    }

    #[test]
    fn preserves_constructor_generics_classification_and_flags() {
        let model = InjectImplBlock::parse(parse_quote! {
            impl<T> Graph<T> {
                async fn new<U: Clone>(nested: Vec<T>, method_generic: U, #[instance_clone] shared: String) -> BuildResult<Self>
                where U: Send { todo!() }
            }
        }).unwrap();
        let constructor = &model.constructor;
        assert!(constructor.asynchronous && constructor.fallible);
        let kinds: Vec<_> = constructor.params.iter().map(|param| param.kind).collect();
        assert_eq!(
            kinds,
            [
                InjectParamKind::Transient,
                InjectParamKind::Instance,
                InjectParamKind::InstanceClone
            ]
        );
        assert_eq!(constructor.params[1].name, "method_generic");
        let (generics, constructor_types) = model.injection_generics();
        assert_eq!(constructor_types.len(), 1);
        assert_eq!(constructor_types[0], "U");
        assert_eq!(generics.params.len(), 2);
        assert_eq!(generics.where_clause.unwrap().predicates.len(), 1);
        let syn::ImplItem::Fn(method) = &model.original_impl.items[0] else {
            panic!("method")
        };
        let syn::FnArg::Typed(param) = &method.sig.inputs[2] else {
            panic!("typed parameter")
        };
        assert!(param.attrs.is_empty());
    }

    #[test]
    fn rejects_invalid_parameter_configurations() {
        for (attrs, expected) in [
            (
                quote!(#[default]),
                "unsupported custom constructor parameter attribute `default`",
            ),
            (
                quote!(#[instance]),
                "unsupported custom constructor parameter attribute `instance`",
            ),
            (
                quote!(#[instance_clone()]),
                "expected bare `#[instance_clone]`",
            ),
            (
                quote!(#[instance_clone = true]),
                "expected bare `#[instance_clone]`",
            ),
            (
                quote!(#[instance_clone] #[instance_clone]),
                "duplicate `instance_clone` attribute",
            ),
            (
                quote!(#[inject(instance_clone)]),
                "use bare `#[instance_clone]`",
            ),
        ] {
            let input =
                syn::parse2(quote!(impl Test { fn new(#attrs value: u32) -> Self { todo!() } }))
                    .unwrap();
            let error = InjectImplBlock::parse(input).err().unwrap();
            assert!(error.to_string().contains(expected), "{attrs}: {error}");
        }
        let error = InjectImplBlock::parse(parse_quote!(
            impl<T> Test<T> {
                fn new(#[instance_clone] value: T) -> Self {
                    todo!()
                }
            }
        ))
        .err()
        .unwrap();
        assert!(
            error
                .to_string()
                .contains("only supported on concrete constructor parameters")
        );
        InjectImplBlock::parse(
            parse_quote!(impl Test { fn new(#[allow(unused)] value: u32) -> Self { todo!() } }),
        )
        .unwrap();
    }

    #[test]
    fn rejects_invalid_constructor_shapes_during_parse() {
        for (input, expected) in [
            (
                quote!(impl [u8] { fn new() -> Self { todo!() } }),
                "Expected type path",
            ),
            (quote!(impl Test {}), "at least one method"),
            (
                quote!(impl Test { fn a() -> Self { todo!() } fn b() -> Self { todo!() } }),
                "exactly one method",
            ),
            (
                quote!(impl Test { fn new(&self) -> Self { todo!() } }),
                "cannot have self parameter",
            ),
            (
                quote!(impl Test { fn new((a, b): (u32, u32)) -> Self { todo!() } }),
                "simple identifier",
            ),
            (
                quote!(impl Test { fn new() -> Option<Self> { todo!() } }),
                "Constructor must return",
            ),
            (
                quote!(impl Test { fn new<'a>() -> Self { todo!() } }),
                "lifetime and const generics",
            ),
            (
                quote!(impl Test { fn new<const N: usize>() -> Self { todo!() } }),
                "lifetime and const generics",
            ),
        ] {
            let error = InjectImplBlock::parse(syn::parse2(input).unwrap())
                .err()
                .unwrap();
            assert!(error.to_string().contains(expected), "{error}");
        }
        // Non-method items and identifier patterns retain their existing acceptance.
        InjectImplBlock::parse(parse_quote!(impl Test { const VALUE: u32 = 1; fn new(mut value: u32) -> Self { todo!() } })).unwrap();
    }
}
