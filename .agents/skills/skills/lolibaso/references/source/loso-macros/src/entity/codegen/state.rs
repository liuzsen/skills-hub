use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityStruct;

pub fn generate(entity: &EntityStruct) -> TokenStream {
    if entity.attrs.no_state {
        return quote! {};
    }

    let entity_name = &entity.name;
    let crate_path = &entity.crate_path;
    let state_name = entity.state_name();
    let vis = &entity.vis;
    let state_attrs: Vec<_> = entity
        .attrs
        .common
        .iter()
        .chain(&entity.attrs.state)
        .collect();
    let id_name = &entity.id_field.name;
    let id_type = &entity.id_field.ty;
    let names: Vec<_> = entity.fields.iter().map(|field| &field.name).collect();
    let types: Vec<_> = entity.fields.iter().map(|field| &field.ty).collect();

    quote! {
        #(#state_attrs)*
        #vis struct #state_name {
            #(pub #names: #types,)*
        }

        impl #crate_path::IntoParts for #entity_name {
            type State = #state_name;

            fn into_parts(self) -> (Self::Id, Self::State) {
                let Self {
                    #id_name,
                    #(#names,)*
                } = self;

                (#id_name, #state_name { #(#names,)* })
            }
        }

        impl #state_name {
            fn into_entity_unchecked(self, #id_name: #id_type) -> #entity_name {
                let Self {
                    #(#names,)*
                } = self;

                #entity_name {
                    #id_name,
                    #(#names,)*
                }
            }
        }

        const _: () = {
            fn assert_reconstitution_contract<T>()
            where
                T: #crate_path::Reconstitute<State = #state_name>,
            {}
            let _ = assert_reconstitution_contract::<#entity_name>;
        };
    }
}
