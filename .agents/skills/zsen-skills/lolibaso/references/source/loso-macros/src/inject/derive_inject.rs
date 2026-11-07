// Copyright (c) 2025
//
// SPDX-License-Identifier: MIT

use super::{
    codegen::{Construction, Dependencies, ExecutionMode, Input, Scope, generate_impl},
    parse::{Dependency, DependencyClassifier, FieldKind as InjectFieldKind, field_kind},
};
use quote::{ToTokens, quote};
use syn::{DeriveInput, Generics, Ident, parse_quote};

#[derive(Clone)]
pub struct InjectStruct {
    names: super::names::Names,
    name: Ident,
    struct_kind: StructKind,
    fields: Vec<InjectField>,
    generics: Generics,
}

type InjectField = Dependency<Option<Ident>, InjectFieldKind>;

fn parse_field(field: &syn::Field, classifier: &DependencyClassifier) -> syn::Result<InjectField> {
    Ok(Dependency {
        name: field.ident.clone(),
        ty: field.ty.clone(),
        kind: field_kind(&field.attrs, &field.ty, classifier)?,
    })
}

impl InjectStruct {
    pub fn parse(derive_input: DeriveInput) -> syn::Result<Self> {
        for attr in &derive_input.attrs {
            if attr.path().is_ident("inject") {
                return Err(syn::Error::new_spanned(
                    attr,
                    "`inject` options are only supported on fields",
                ));
            }
        }
        let names = super::names::Names::new(derive_input.to_token_stream());
        let classifier = DependencyClassifier::new(&derive_input.generics);
        let struct_kind;
        let struct_fields;
        match &derive_input.data {
            syn::Data::Struct(data_struct) => match &data_struct.fields {
                syn::Fields::Named(fields) => {
                    struct_kind = StructKind::Named;
                    struct_fields = fields
                        .named
                        .iter()
                        .map(|f| parse_field(f, &classifier))
                        .collect::<syn::Result<Vec<_>>>()?;
                }
                syn::Fields::Unnamed(fields) => {
                    struct_kind = StructKind::Unnamed;
                    struct_fields = fields
                        .unnamed
                        .iter()
                        .map(|f| parse_field(f, &classifier))
                        .collect::<syn::Result<Vec<_>>>()?;
                }
                syn::Fields::Unit => {
                    struct_kind = StructKind::Unit;
                    struct_fields = Vec::new();
                }
            },
            syn::Data::Enum(..) => {
                return Err(syn::Error::new_spanned(
                    derive_input,
                    "Enum data is not supported",
                ));
            }
            syn::Data::Union(..) => {
                return Err(syn::Error::new_spanned(
                    derive_input,
                    "Union is not supported",
                ));
            }
        }

        Ok(Self {
            names,
            name: derive_input.ident,
            struct_kind,
            fields: struct_fields,
            generics: derive_input.generics,
        })
    }

    fn generate(&self, scope: Scope, root: &syn::Path) -> proc_macro2::TokenStream {
        let name = &self.name;
        let ty_generics = self.generics.split_for_impl().1;
        let self_ty = parse_quote!(#name #ty_generics);
        let input = Input {
            root,
            names: &self.names,
            self_ty: &self_ty,
            generics: &self.generics,
            constructor_types: &[],
            dependencies: Dependencies::Fields(&self.fields),
            construction: match self.struct_kind {
                StructKind::Named => Construction::Named,
                StructKind::Unnamed => Construction::Tuple,
                StructKind::Unit => Construction::Unit,
            },
        };
        let sync = generate_impl(&input, ExecutionMode::Sync, scope);
        let asynchronous = generate_impl(&input, ExecutionMode::Async, scope);
        quote!(#sync #asynchronous)
    }

    pub fn generate_inject(self, root: &syn::Path) -> proc_macro2::TokenStream {
        self.generate(Scope::Transient, root)
    }

    pub fn generate_inject_singleton(self, root: &syn::Path) -> proc_macro2::TokenStream {
        self.generate(Scope::Singleton, root)
    }
}

#[derive(Clone)]
enum StructKind {
    Named,
    Unnamed,
    Unit,
}

#[cfg(test)]
mod tests {
    use super::super::parse::DependencyKind;
    use super::*;

    #[test]
    fn parses_field_shapes_and_dependency_kinds() {
        let model = InjectStruct::parse(parse_quote! {
            struct Graph<T> {
                recursive: Vec<T>,
                concrete: u32,
                #[inject(instance_clone,)]
                shared: String,
                #[inject(default)]
                fallback: T,
            }
        })
        .unwrap();
        assert!(matches!(model.struct_kind, StructKind::Named));
        let kinds: Vec<_> = model.fields.iter().map(|field| field.kind).collect();
        assert_eq!(
            kinds,
            [
                InjectFieldKind::Transient,
                InjectFieldKind::Instance,
                InjectFieldKind::InstanceClone,
                InjectFieldKind::Default
            ]
        );
        assert_eq!(model.fields[0].name.as_ref().unwrap(), "recursive");
        let tuple = InjectStruct::parse(parse_quote!(
            struct Pair<T>(T, #[inject(default)] bool);
        ))
        .unwrap();
        assert!(matches!(tuple.struct_kind, StructKind::Unnamed));
        assert!(tuple.fields.iter().all(|field| field.name.is_none()));
        let unit = InjectStruct::parse(parse_quote!(
            struct Unit;
        ))
        .unwrap();
        assert!(matches!(unit.struct_kind, StructKind::Unit));
        assert!(unit.fields.is_empty());
    }

    #[test]
    fn shared_classifier_preserves_nested_and_associated_type_detection() {
        let classifier = DependencyClassifier::new(&parse_quote!(<'a, T, const N: usize>));
        for ty in [
            parse_quote!(Vec<T>),
            parse_quote!(T::Item),
            parse_quote!(<T as Trait>::Item),
        ] {
            assert_eq!(
                classifier.classify(&ty, false, "generic clone").unwrap(),
                DependencyKind::Transient
            );
        }
        // Lifetimes and const parameters are not type parameters in the old checker.
        for ty in [
            parse_quote!(&'a str),
            parse_quote!([u8; N]),
            parse_quote!(String),
        ] {
            assert_eq!(
                classifier.classify(&ty, false, "generic clone").unwrap(),
                DependencyKind::Instance
            );
        }
    }

    #[test]
    fn rejects_invalid_field_configurations() {
        for (attrs, expected) in [
            (quote!(#[inject]), "expected `#[inject(default)]`"),
            (quote!(#[inject()]), "expected an injection option"),
            (
                quote!(#[inject = "default"]),
                "expected `#[inject(default)]`",
            ),
            (
                quote!(#[inject(unknown)]),
                "unsupported field injection attribute",
            ),
            (
                quote!(#[inject(foo::default)]),
                "unsupported field injection attribute",
            ),
            (
                quote!(#[inject(instance)]),
                "unsupported field injection attribute `instance`",
            ),
            (
                quote!(#[inject(default = true)]),
                "do not accept arguments or values",
            ),
            (
                quote!(#[inject(default())]),
                "do not accept arguments or values",
            ),
            (
                quote!(#[inject(default extra)]),
                "do not accept arguments or values",
            ),
            (
                quote!(#[inject(default, default)]),
                "duplicate injection option",
            ),
            (
                quote!(#[inject(default)] #[inject(default)]),
                "duplicate injection option",
            ),
            (
                quote!(#[inject(instance_clone, instance_clone)]),
                "duplicate injection option",
            ),
            (
                quote!(#[inject(default, instance_clone)]),
                "conflicting injection options",
            ),
            (
                quote!(#[inject(instance_clone)] #[inject(default)]),
                "conflicting injection options",
            ),
        ] {
            let input = syn::parse2(quote!(struct Test { #attrs value: u32 })).unwrap();
            let error = InjectStruct::parse(input)
                .err()
                .expect("invalid configuration");
            assert!(error.to_string().contains(expected), "{attrs}: {error}");
        }
        let error = InjectStruct::parse(parse_quote!(
            struct Test<T>(#[inject(instance_clone)] T);
        ))
        .err()
        .unwrap();
        assert!(
            error
                .to_string()
                .contains("only supported on concrete fields")
        );
    }

    #[test]
    fn rejects_wrong_targets_without_rejecting_unrelated_attributes() {
        for (input, expected) in [
            (
                quote!(
                    #[inject(default)]
                    struct Test;
                ),
                "only supported on fields",
            ),
            (
                quote!(
                    #[inject(crate = some::runtime)]
                    struct Test;
                ),
                "only supported on fields",
            ),
            (
                quote!(
                    enum Test {
                        One,
                    }
                ),
                "Enum data is not supported",
            ),
            (quote!(union Test { value: u32 }), "Union is not supported"),
        ] {
            let error = InjectStruct::parse(syn::parse2(input).unwrap())
                .err()
                .unwrap();
            assert!(error.to_string().contains(expected), "{error}");
        }
        InjectStruct::parse(parse_quote!(
            #[allow(dead_code)]
            struct Test(#[allow(unused)] u32);
        ))
        .unwrap();
    }
}
