#[cfg(feature = "_paths")]
mod paths;

// 从 feature 代码块自动生成公共依赖的 cfg，新增 feature 时只需追加代码块。
macro_rules! feature_macros {
    ($($feature:literal => { $($item:item)* })*) => {
        #[cfg(any($(feature = $feature),*))]
        use proc_macro::TokenStream;

        $(
            $(
                #[cfg(feature = $feature)]
                $item
            )*
        )*
    };
}

feature_macros! {
    "entity" => {
        mod entity;

        #[proc_macro_derive(Entity, attributes(entity, patch, state, readonly))]
        pub fn derive_entity(input: TokenStream) -> TokenStream {
            let input = syn::parse_macro_input!(input as syn::DeriveInput);
            entity::expand(input)
                .unwrap_or_else(syn::Error::into_compile_error)
                .into()
        }
    }
    "inject" => {
        mod inject;

        #[proc_macro_derive(Inject, attributes(inject))]
        pub fn derive_inject(input: TokenStream) -> TokenStream {
            let input = syn::parse_macro_input!(input as syn::DeriveInput);
            inject::expand_derive(input, false)
                .unwrap_or_else(syn::Error::into_compile_error)
                .into()
        }

        #[proc_macro_derive(InjectSingleton, attributes(inject))]
        pub fn derive_inject_singleton(input: TokenStream) -> TokenStream {
            let input = syn::parse_macro_input!(input as syn::DeriveInput);
            inject::expand_derive(input, true)
                .unwrap_or_else(syn::Error::into_compile_error)
                .into()
        }

        #[proc_macro_attribute]
        pub fn inject(attr: TokenStream, item: TokenStream) -> TokenStream {
            let item = syn::parse_macro_input!(item as syn::ItemImpl);
            inject::expand_attribute(attr.into(), item, false)
                .unwrap_or_else(syn::Error::into_compile_error)
                .into()
        }

        #[proc_macro_attribute]
        pub fn inject_singleton(attr: TokenStream, item: TokenStream) -> TokenStream {
            let item = syn::parse_macro_input!(item as syn::ItemImpl);
            inject::expand_attribute(attr.into(), item, true)
                .unwrap_or_else(syn::Error::into_compile_error)
                .into()
        }
    }
    "http" => {
        mod http_error;

        #[proc_macro_derive(HttpErrorCode, attributes(http))]
        pub fn derive_http_error_code(input: TokenStream) -> TokenStream {
            let input = syn::parse_macro_input!(input as syn::DeriveInput);
            http_error::expand(input)
                .unwrap_or_else(syn::Error::into_compile_error)
                .into()
        }
    }
}
