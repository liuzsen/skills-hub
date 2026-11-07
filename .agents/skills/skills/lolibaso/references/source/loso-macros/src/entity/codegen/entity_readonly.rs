use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityStruct;

pub fn generate(entity: &EntityStruct) -> TokenStream {
    let entity_name = &entity.name;
    let readonly_name = entity.readonly_name();
    let vis = &entity.vis;

    let readonly_attrs: Vec<_> = entity
        .attrs
        .common
        .iter()
        .chain(&entity.attrs.readonly)
        .collect();

    let id_field = &entity.id_field;
    let id_name = &id_field.name;
    let id_type = &id_field.ty;

    let field_names: Vec<_> = entity.fields.iter().map(|f| &f.name).collect();
    let field_types: Vec<_> = entity.fields.iter().map(|f| &f.ty).collect();

    quote! {
        #(#readonly_attrs)*
        #vis struct #readonly_name<'a> {
            pub #id_name: &'a #id_type,
            #(
                pub #field_names: &'a #field_types,
            )*
        }

        impl<'a> ::core::convert::From<&'a #entity_name> for #readonly_name<'a> {
            fn from(entity: &'a #entity_name) -> Self {
                Self {
                    #id_name: &entity.#id_name,
                    #(
                        #field_names: &entity.#field_names,
                    )*
                }
            }
        }
    }
}
