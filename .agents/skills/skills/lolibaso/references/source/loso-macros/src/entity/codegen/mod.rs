mod entity_impl;
mod entity_readonly;
mod entity_updater;
mod state;

use proc_macro2::TokenStream;
use quote::quote;

use crate::entity::parse::EntityStruct;

pub fn generate_entity_code(entity: &EntityStruct) -> TokenStream {
    let entity_impl = entity_impl::generate(entity);

    let entity_patch = entity_updater::generate(entity);

    let state = state::generate(entity);

    let entity_readonly = entity_readonly::generate(entity);

    quote! {
        #entity_impl

        #entity_patch

        #state

        #entity_readonly
    }
}
