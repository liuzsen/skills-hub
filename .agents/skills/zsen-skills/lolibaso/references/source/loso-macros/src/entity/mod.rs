mod attrs;
mod codegen;
mod field;
mod parse;

pub use parse::parse_entity;

use proc_macro2::TokenStream;
use syn::DeriveInput;

use crate::entity::codegen::generate_entity_code;

pub fn expand(input: DeriveInput) -> syn::Result<TokenStream> {
    let entity_struct = parse_entity(&input)?;
    Ok(generate_entity_code(&entity_struct))
}

#[cfg(test)]
mod tests {
    #[test]
    fn preserves_public_generated_types_and_shared_readonly_fields() {
        for visibility in ["", "pub(crate)", "pub(super)", "pub"] {
            let input = syn::parse_str(&format!(
                "#[entity(patchable)] {visibility} struct Example {{ id: u64, value: String, optional: Option<String> }}"
            )).unwrap();
            let file: syn::File =
                syn::parse2(crate::paths::with_test_dependencies(|| super::expand(input)).unwrap())
                    .unwrap();
            let structs: Vec<_> = file
                .items
                .iter()
                .filter_map(|item| match item {
                    syn::Item::Struct(item) => Some(item),
                    _ => None,
                })
                .collect();
            assert_eq!(structs.len(), 3);
            for item in &structs {
                assert!(
                    matches!(item.vis, syn::Visibility::Public(_)),
                    "{visibility}: {}",
                    item.ident
                );
                assert!(
                    item.fields
                        .iter()
                        .all(|field| matches!(field.vis, syn::Visibility::Public(_)))
                );
            }
            let readonly = structs
                .iter()
                .find(|item| item.ident == "ExampleReadonly")
                .unwrap();
            let fields: Vec<_> = readonly
                .fields
                .iter()
                .map(|field| {
                    let syn::Type::Reference(reference) = &field.ty else {
                        panic!("expected shared reference")
                    };
                    assert!(reference.mutability.is_none());
                    assert_eq!(reference.lifetime.as_ref().unwrap().ident, "a");
                    let ty = &reference.elem;
                    (
                        field.ident.as_ref().unwrap().to_string(),
                        quote::quote!(#ty).to_string(),
                    )
                })
                .collect();
            assert_eq!(
                fields,
                [
                    ("id".into(), "u64".into()),
                    ("value".into(), "String".into()),
                    ("optional".into(), "Option < String >".into()),
                ]
            );
        }
    }

    #[test]
    fn no_state_omits_generated_state_and_owned_contracts() {
        let output = crate::paths::with_test_dependencies(|| {
            super::expand(syn::parse_quote! {
                #[entity(no_state)]
                struct Example { id: u64, value: String }
            })
        })
        .unwrap()
        .to_string();
        assert!(!output.contains("ExampleState"));
        assert!(!output.contains("IntoParts"));
        assert!(!output.contains("Reconstitute"));
        syn::parse2::<syn::File>(output.parse().unwrap()).unwrap();
    }

    #[test]
    fn automatic_component_path_is_used_for_all_contracts_without_docs() {
        let output = crate::paths::with_test_dependencies(|| {
            super::expand(syn::parse_quote! {
                #[doc = "not propagated"]
                #[entity(patchable)]
                #[state(doc = "not propagated either")]
                struct Example { id: u64, value: String }
            })
        })
        .unwrap()
        .to_string();
        assert!(output.contains(":: loso_entity :: Entity"));
        assert!(output.contains(":: loso_entity :: Reconstitute"));
        assert!(output.contains(":: loso_entity :: IntoParts"));
        assert!(!output.contains("lolibaso"));
        assert!(!output.contains("doc"));
        syn::parse2::<syn::File>(output.parse().unwrap()).unwrap();
    }
}
