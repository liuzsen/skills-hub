// Copyright (c) 2025
//
// SPDX-License-Identifier: MIT

mod attribute_inject;
mod codegen;
mod derive_inject;
mod names;
mod parse;

use attribute_inject::InjectImplBlock;
use derive_inject::InjectStruct;
use proc_macro2::TokenStream;

pub(crate) fn expand_derive(input: syn::DeriveInput, singleton: bool) -> syn::Result<TokenStream> {
    let model = InjectStruct::parse(input)?;
    let root = runtime_root()?;
    Ok(if singleton {
        model.generate_inject_singleton(&root)
    } else {
        model.generate_inject(&root)
    })
}

pub(crate) fn expand_attribute(
    attr: TokenStream,
    input: syn::ItemImpl,
    singleton: bool,
) -> syn::Result<TokenStream> {
    if !attr.is_empty() {
        return Err(syn::Error::new_spanned(
            attr,
            "injection attributes do not accept arguments",
        ));
    }
    let model = InjectImplBlock::parse(input)?;
    let root = runtime_root()?;
    Ok(if singleton {
        model.generate_inject_singleton(&root)
    } else {
        model.generate_inject(&root)
    })
}

fn runtime_root() -> syn::Result<syn::Path> {
    #[cfg(test)]
    {
        Ok(syn::parse_quote!(::test_runtime))
    }
    #[cfg(not(test))]
    {
        crate::paths::resolve("loso-inject", "inject")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn rejects_top_level_arguments_in_both_attribute_modes() {
        for singleton in [false, true] {
            let input: syn::ItemImpl = parse_quote!(impl Test { fn new() -> Self { Self } });
            for attr in [quote!(unknown), quote!(crate = some::runtime)] {
                let error = expand_attribute(attr, input.clone(), singleton).unwrap_err();
                assert!(error.to_string().contains("do not accept arguments"));
            }
        }
    }

    #[test]
    fn generated_impl_counts_and_original_retention_are_unchanged() {
        for singleton in [false, true] {
            for asynchronous in [false, true] {
                let async_token = asynchronous.then(|| quote!(async));
                let input = syn::parse2(quote! {
                    impl Test {
                        const VALUE: u32 = 1;
                        type Extra = u32;
                        #[allow(dead_code)]
                        #async_token fn new(#[instance_clone] #[allow(unused)] value: u32) -> Self { Self }
                    }
                }).unwrap();
                let output = expand_attribute(TokenStream::new(), input, singleton).unwrap();
                let file: syn::File = syn::parse2(output).unwrap();
                assert_eq!(
                    file.items
                        .iter()
                        .filter(|item| matches!(item, syn::Item::Impl(_)))
                        .count(),
                    1
                );
                assert_eq!(
                    file.items
                        .iter()
                        .filter(|item| matches!(item, syn::Item::Const(_)))
                        .count(),
                    if asynchronous { 1 } else { 2 }
                );
                let syn::Item::Impl(original) = &file.items[0] else {
                    panic!("original impl first")
                };
                assert!(matches!(original.items[0], syn::ImplItem::Const(_)));
                assert!(matches!(original.items[1], syn::ImplItem::Type(_)));
                let syn::ImplItem::Fn(method) = &original.items[2] else {
                    panic!("constructor retained")
                };
                assert_eq!(method.attrs.len(), 1);
                assert!(method.attrs[0].path().is_ident("allow"));
                let syn::FnArg::Typed(parameter) = &method.sig.inputs[0] else {
                    panic!("typed parameter")
                };
                assert_eq!(parameter.attrs.len(), 1);
                assert!(parameter.attrs[0].path().is_ident("allow"));
            }
            let output = expand_derive(
                parse_quote!(
                    struct Test;
                ),
                singleton,
            )
            .unwrap();
            let file: syn::File = syn::parse2(output).unwrap();
            assert_eq!(file.items.len(), 2);
        }
    }
}
