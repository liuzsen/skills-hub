use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityStruct;

pub fn generate(entity: &EntityStruct) -> TokenStream {
    if !entity.attrs.patchable {
        return quote! {};
    }

    let patch_name = entity.patch_name();
    let vis = &entity.vis;
    let generics = &entity.generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let patch_attrs: Vec<_> = entity
        .attrs
        .common
        .iter()
        .chain(&entity.attrs.patch)
        .collect();

    let patch_fields: Vec<_> = entity.fields.iter().filter(|f| f.patch).collect();
    let field_names: Vec<_> = patch_fields.iter().map(|f| &f.name).collect();
    let field_types: Vec<_> = patch_fields.iter().map(|f| &f.ty).collect();
    let id_name = &entity.id_field.name;
    let id_ty = &entity.id_field.ty;

    quote! {
        #(#patch_attrs)*
        #vis struct #patch_name #impl_generics #where_clause {
            pub #id_name: #id_ty,
            #(
                pub #field_names: ::core::option::Option<#field_types>,
            )*
        }
    }
}
