use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityStruct;

pub fn generate(entity: &EntityStruct) -> TokenStream {
    let entity_name = &entity.name;
    let crate_path = &entity.crate_path;
    let generics = &entity.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let id_ty = &entity.id_field.ty;
    let id_name = &entity.id_field.name;

    let readonly_name = entity.readonly_name();

    quote! {
        impl #impl_generics #crate_path::Entity for #entity_name #ty_generics #where_clause {
            type Id = #id_ty;

            type Readonly<'a> = #readonly_name<'a>;

            fn readonly(&self) -> Self::Readonly<'_> {
                self.into()
            }

            fn id(&self) -> &Self::Id {
                &self.#id_name
            }
        }
    }
}
