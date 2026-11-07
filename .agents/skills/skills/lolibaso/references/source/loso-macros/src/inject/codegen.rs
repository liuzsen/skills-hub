// Copyright (c) 2025
//
// SPDX-License-Identifier: MIT

use super::parse::{Dependency, FieldKind};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericParam, Generics, Ident, Type, TypeParam, parse_quote};

#[derive(Clone, Copy)]
pub(super) enum ExecutionMode {
    Sync,
    Async,
}

#[derive(Clone, Copy)]
pub(super) enum Scope {
    Transient,
    Singleton,
}

pub(super) enum Construction<'a> {
    Named,
    Tuple,
    Unit,
    Constructor {
        name: &'a Ident,
        mode: ExecutionMode,
        fallible: bool,
    },
}

// Borrow the parser's two shapes without weakening required parameter names or
// introducing a second owned dependency model.
pub(super) enum Dependencies<'a> {
    Fields(&'a [Dependency<Option<Ident>, FieldKind>]),
    Parameters(&'a [Dependency<Ident>]),
}

impl Dependencies<'_> {
    fn iter(&self) -> impl Iterator<Item = (Option<&Ident>, &Type, FieldKind)> {
        let (fields, parameters): (&[_], &[_]) = match self {
            Self::Fields(fields) => (fields, &[]),
            Self::Parameters(parameters) => (&[], parameters),
        };
        fields
            .iter()
            .map(|field| (field.name.as_ref(), &field.ty, field.kind))
            .chain(
                parameters
                    .iter()
                    .map(|param| (Some(&param.name), &param.ty, FieldKind::from(param.kind))),
            )
    }
}

pub(super) struct Input<'a> {
    pub root: &'a syn::Path,
    pub names: &'a super::names::Names,
    pub self_ty: &'a Type,
    pub generics: &'a Generics,
    pub constructor_types: &'a [Ident],
    pub dependencies: Dependencies<'a>,
    pub construction: Construction<'a>,
}

pub(super) fn generate_impl(input: &Input<'_>, mode: ExecutionMode, scope: Scope) -> TokenStream {
    let root = input.root;
    let mut allocator = input.names.clone();
    let context_ty = allocator.fresh();
    let context_local = allocator.fresh();
    let result_local = allocator.fresh();
    let singleton = matches!(scope, Scope::Singleton);
    let (trait_name, async_token, await_token) = match mode {
        ExecutionMode::Sync => (quote!(Inject), quote!(), quote!()),
        ExecutionMode::Async => (quote!(InjectAsync), quote!(async), quote!(.await)),
    };
    let mut generics = input.generics.clone();
    let mut indices = Vec::new();
    let mut names = Vec::new();
    let mut exprs = Vec::new();
    let mut context = quote!(#context_ty);
    let mut labels = Vec::new();

    if matches!(mode, ExecutionMode::Async) {
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote!(#context_ty: ::core::marker::Send));
    }
    if singleton {
        let predicates = &mut generics.make_where_clause().predicates;
        predicates.push(parse_quote!(Self: ::core::clone::Clone + 'static));
        predicates.push(parse_quote!(#context_ty: #root::SingletonContainer));
    }

    for (name, ty, kind) in input.dependencies.iter() {
        labels.push(name.cloned());
        let name = allocator.fresh();
        names.push(name.clone());
        if kind.is_default() {
            exprs.push(quote!(let #name = ::core::default::Default::default();));
            continue;
        }

        let idx = allocator.fresh();
        generics
            .params
            .push(GenericParam::Type(TypeParam::from(idx.clone())));
        indices.push(idx.clone());
        let predicates = &mut generics.make_where_clause().predicates;
        match kind {
            FieldKind::Instance | FieldKind::InstanceClone if singleton => {
                predicates.push(parse_quote!(#context_ty: #root::hlist::Selector<#ty, #idx>));
                exprs
                    .push(quote!(let #name = ::core::clone::Clone::clone(<#context as #root::hlist::Selector<#ty, #idx>>::get(&#context_local));));
            }
            FieldKind::Instance => {
                // A concrete pluck can precede an awaited recursive injection.
                // Its associated remainder is not implied Send by the input.
                let remainder_bound = match mode {
                    ExecutionMode::Sync => quote!(),
                    ExecutionMode::Async => quote!(, Remainder: ::core::marker::Send),
                };
                predicates.push(
                    parse_quote!(#context: #root::hlist::Plucker<#ty, #idx #remainder_bound>),
                );
                context = quote!(<#context as #root::hlist::Plucker<#ty, #idx>>::Remainder);
                exprs.push(
                    quote!(let (#name, #context_local) = #root::hlist::Plucker::pluck(#context_local);),
                );
            }
            FieldKind::InstanceClone => {
                predicates.push(parse_quote!(#context: #root::hlist::Selector<#ty, #idx>));
                exprs
                    .push(quote!(let #name = ::core::clone::Clone::clone(<#context as #root::hlist::Selector<#ty, #idx>>::get(&#context_local));));
            }
            FieldKind::Transient => {
                if singleton {
                    predicates.push(parse_quote!(#ty: #root::#trait_name<#context_ty, #idx, OutContext = #context_ty>));
                } else {
                    predicates.push(parse_quote!(#ty: #root::#trait_name<#context, #idx>));
                    context = quote!(<#ty as #root::#trait_name<#context, #idx>>::OutContext);
                }
                let input_context = if singleton {
                    quote!(#context_ty)
                } else {
                    quote!(_)
                };
                exprs.push(quote!(let (#name, mut #context_local) = <#ty as #root::#trait_name<#input_context, #idx>>::inject(#context_local) #await_token ?;));
            }
            FieldKind::Default => {
                unreachable!("default dependencies handled before index allocation")
            }
        }
    }

    generics.params.push(parse_quote!(#context_ty));
    let constructor_types = input.constructor_types;
    let index_tuple: Type = if !indices.is_empty() {
        parse_quote!((#(#constructor_types,)* #(#indices),*))
    } else if !constructor_types.is_empty() {
        parse_quote!((#(#constructor_types),*))
    } else {
        parse_quote!(#root::hlist::indices::Here)
    };
    let init = match &input.construction {
        Construction::Named => quote!(Self { #(#labels: #names,)* }),
        Construction::Tuple => quote!(Self(#(#names,)*)),
        Construction::Unit => quote!(Self {}),
        Construction::Constructor {
            name,
            mode,
            fallible,
        } => {
            let constructor_await = matches!(mode, ExecutionMode::Async).then(|| quote!(.await));
            let question = fallible.then(|| quote!(?));
            quote!(Self::#name(#(#names),*) #constructor_await #question)
        }
    };
    let body = if singleton {
        quote! {
            match #root::SingletonContainer::get_singleton::<Self>(&#context_local) {
                ::core::option::Option::Some(#result_local) => ::core::result::Result::Ok((::core::clone::Clone::clone(#result_local), #context_local)),
                ::core::option::Option::None => {
                    #(#exprs)*
                    let #result_local = #init;
                    #root::SingletonContainer::add_singleton(&mut #context_local, ::core::clone::Clone::clone(&#result_local));
                    ::core::result::Result::Ok((#result_local, #context_local))
                }
            }
        }
    } else {
        quote! { #(#exprs)* ::core::result::Result::Ok((#init, #context_local)) }
    };
    let self_ty = input.self_ty;
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    quote! {
        #[allow(non_camel_case_types, unused_mut)]
        const _: () = {
            impl #impl_generics #root::#trait_name<#context_ty, #index_tuple> for #self_ty #where_clause {
                type OutContext = #context;
                #async_token fn inject(mut #context_local: #context_ty) -> #root::anyhow::Result<(Self, Self::OutContext)> {
                    #body
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::super::{expand_attribute, expand_derive};
    use super::*;
    use syn::{GenericArgument, Item, ItemImpl, PathArguments, TypeParamBound, WherePredicate};

    fn implementations(tokens: TokenStream) -> Vec<ItemImpl> {
        let file: syn::File = syn::parse2(tokens).unwrap();
        file.items
            .into_iter()
            .filter_map(|item| {
                let Item::Const(item) = item else {
                    return None;
                };
                let syn::Expr::Block(block) = *item.expr else {
                    panic!("const block")
                };
                block.block.stmts.into_iter().find_map(|stmt| {
                    if let syn::Stmt::Item(Item::Impl(item)) = stmt {
                        Some(item)
                    } else {
                        None
                    }
                })
            })
            .collect()
    }

    fn trait_bounds(item: &ItemImpl) -> impl Iterator<Item = &syn::TraitBound> {
        item.generics
            .where_clause
            .iter()
            .flat_map(|clause| &clause.predicates)
            .filter_map(|predicate| match predicate {
                WherePredicate::Type(ty) => Some(&ty.bounds),
                _ => None,
            })
            .flatten()
            .filter_map(|bound| match bound {
                TypeParamBound::Trait(bound) => Some(bound),
                _ => None,
            })
    }

    fn arguments(bound: &syn::TraitBound) -> impl Iterator<Item = &GenericArgument> {
        let PathArguments::AngleBracketed(arguments) =
            &bound.path.segments.last().unwrap().arguments
        else {
            panic!("trait arguments")
        };
        arguments.args.iter()
    }

    #[test]
    fn execution_and_scope_bounds_are_shared_but_not_strengthened_for_sync() {
        for singleton in [false, true] {
            let derive = expand_derive(
                parse_quote! {
                    struct Graph<T> { value: u32, dependency: T }
                },
                singleton,
            )
            .unwrap();
            let attribute = expand_attribute(
                TokenStream::new(),
                parse_quote! {
                    impl<T> Graph<T> { fn new(value: u32, dependency: T) -> Self { todo!() } }
                },
                singleton,
            )
            .unwrap();
            for output in [derive, attribute] {
                let impls = implementations(output);
                assert_eq!(impls.len(), 2);
                for (asynchronous, item) in impls.iter().enumerate() {
                    let is_async = asynchronous == 1;
                    let name = &item
                        .trait_
                        .as_ref()
                        .unwrap()
                        .1
                        .segments
                        .last()
                        .unwrap()
                        .ident;
                    assert_eq!(name, if is_async { "InjectAsync" } else { "Inject" });
                    assert_eq!(
                        trait_bounds(item)
                            .filter(|bound| bound.path.segments.last().unwrap().ident == "Send")
                            .count(),
                        usize::from(is_async)
                    );
                    let pluckers: Vec<_> = trait_bounds(item)
                        .filter(|bound| bound.path.segments.last().unwrap().ident == "Plucker")
                        .collect();
                    assert_eq!(pluckers.len(), usize::from(!singleton));
                    for bound in pluckers {
                        let remainder_send = arguments(bound).any(|arg| match arg {
                            GenericArgument::Constraint(constraint) if constraint.ident == "Remainder" => constraint.bounds.iter().any(|bound| matches!(bound, TypeParamBound::Trait(bound) if bound.path.segments.last().unwrap().ident == "Send")),
                            _ => false,
                        });
                        assert_eq!(remainder_send, is_async);
                    }
                    let recursive = trait_bounds(item)
                        .find(|bound| bound.path.segments.last().unwrap().ident == *name)
                        .unwrap();
                    let equal_context = arguments(recursive).any(|arg| matches!(arg, GenericArgument::AssocType(binding) if binding.ident == "OutContext" && matches!(&binding.ty, Type::Path(ty) if ty.path.segments.last().unwrap().ident == item.generics.type_params().last().unwrap().ident)));
                    assert_eq!(equal_context, singleton);
                    let out = item
                        .items
                        .iter()
                        .find_map(|item| match item {
                            syn::ImplItem::Type(ty) => Some(&ty.ty),
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(
                        matches!(out, Type::Path(ty) if ty.path.segments.last().unwrap().ident == item.generics.type_params().last().unwrap().ident),
                        singleton
                    );
                }
            }
        }
    }

    #[test]
    fn shapes_indices_and_constructor_generics_are_preserved() {
        for singleton in [false, true] {
            for input in [
                parse_quote!(
                    struct Named {
                        #[inject(default)]
                        value: bool,
                    }
                ),
                parse_quote!(
                    struct Tuple(#[inject(default)] bool);
                ),
                parse_quote!(
                    struct Unit;
                ),
            ] {
                for item in implementations(expand_derive(input, singleton).unwrap()) {
                    let segment = item.trait_.as_ref().unwrap().1.segments.last().unwrap();
                    let PathArguments::AngleBracketed(args) = &segment.arguments else {
                        panic!("indices")
                    };
                    assert!(
                        matches!(&args.args[1], GenericArgument::Type(Type::Path(ty)) if ty.path.segments.last().unwrap().ident == "Here")
                    );
                    assert_eq!(item.generics.params.len(), 1);
                }
            }
            for item in implementations(
                expand_derive(
                    parse_quote!(
                        struct One(u32);
                    ),
                    singleton,
                )
                .unwrap(),
            ) {
                let segment = item.trait_.as_ref().unwrap().1.segments.last().unwrap();
                let PathArguments::AngleBracketed(args) = &segment.arguments else {
                    panic!("indices")
                };
                assert!(matches!(
                    &args.args[1],
                    GenericArgument::Type(Type::Paren(_))
                ));
            }
        }
        for item in implementations(
            expand_attribute(
                TokenStream::new(),
                parse_quote! {
                    impl Node { fn new<U: Clone>(value: U) -> Self where U: Send { todo!() } }
                },
                false,
            )
            .unwrap(),
        ) {
            let segment = item.trait_.as_ref().unwrap().1.segments.last().unwrap();
            let PathArguments::AngleBracketed(args) = &segment.arguments else {
                panic!("indices")
            };
            let GenericArgument::Type(Type::Tuple(indices)) = &args.args[1] else {
                panic!("constructor prefix")
            };
            assert_eq!(indices.elems.len(), 2);
            assert!(matches!(&indices.elems[0], Type::Path(ty) if ty.path.is_ident("U")));
            assert_eq!(item.generics.where_clause.as_ref().unwrap().predicates.iter().filter(|predicate| matches!(predicate, WherePredicate::Type(ty) if matches!(&ty.bounded_ty, Type::Path(ty) if ty.path.is_ident("U")))).count(), 1);
        }
    }
}
