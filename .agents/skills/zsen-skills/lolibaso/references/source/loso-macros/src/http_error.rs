use convert_case::{Case, Casing};
use quote::quote;
use std::collections::HashMap;
use syn::{Attribute, Data, DeriveInput, Fields, LitInt, LitStr, Variant};

pub(crate) fn expand(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "HttpErrorCode does not support generic enums",
        ));
    }

    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "HttpErrorCode can only be derived for enums",
        ));
    };
    if data.variants.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "HTTP error catalog must contain at least one variant",
        ));
    }
    let repr = parse_integer_repr(&input.attrs, &input.ident)?;

    let catalog = parse_catalog_attrs(&input.attrs)?;
    let mut codes = HashMap::<String, proc_macro2::Span>::new();
    let mut code_arms = Vec::with_capacity(data.variants.len());
    let mut status_arms = Vec::with_capacity(data.variants.len());

    for variant in &data.variants {
        let attrs = parse_variant_attrs(variant)?;
        let code = match attrs.code {
            Some(code) => code,
            None => {
                let value = rename(&variant.ident.to_string(), catalog.rename_all)?;
                LitStr::new(&value, variant.ident.span())
            }
        };
        if code.value().is_empty() {
            return Err(syn::Error::new_spanned(
                &code,
                "HTTP error code cannot be empty",
            ));
        }
        if codes.insert(code.value(), code.span()).is_some() {
            return Err(syn::Error::new_spanned(
                &code,
                format!(
                    "HTTP error code `{}` is assigned more than once",
                    code.value()
                ),
            ));
        }

        let status = attrs.status.or(catalog.status).unwrap_or(400);
        let name = &variant.ident;
        let pattern = match &variant.fields {
            Fields::Unit => quote!(Self::#name),
            Fields::Unnamed(_) => quote!(Self::#name(..)),
            Fields::Named(_) => quote!(Self::#name { .. }),
        };
        code_arms.push(quote!(#pattern => #code));
        status_arms.push(quote!(#pattern => #status));
    }

    let name = &input.ident;
    let crate_path = crate::paths::resolve("loso-http", "http::error")?;
    Ok(quote! {
        impl #crate_path::HttpErrorCode for #name {
            type Discriminant = #repr;

            fn code(&self) -> &'static str {
                match self { #(#code_arms,)* }
            }

            fn discriminant(&self) -> Self::Discriminant {
                // SAFETY: Self has an explicit integer repr matching Discriminant.
                // For fieldless enums this is the value's representation. For
                // enums with fields (including repr(C, integer)), the integer tag
                // is at offset zero and aligned for the declared integer type.
                unsafe { *::core::ptr::from_ref(self).cast::<#repr>() }
            }

            fn http_status(&self) -> u16 {
                match self { #(#status_arms,)* }
            }
        }
    })
}

#[derive(Default)]
struct CatalogAttrs {
    status: Option<u16>,
    rename_all: Option<RenameRule>,
}

#[derive(Default)]
struct VariantAttrs {
    code: Option<LitStr>,
    status: Option<u16>,
}

#[derive(Clone, Copy)]
enum RenameRule {
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    ScreamingSnake,
    Kebab,
    ScreamingKebab,
}

fn parse_integer_repr(attrs: &[Attribute], ident: &syn::Ident) -> syn::Result<syn::Ident> {
    let mut repr = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("repr")) {
        let items = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        )?;
        for item in items {
            let syn::Meta::Path(path) = item else {
                continue;
            };
            let Some(integer) = path.get_ident() else {
                continue;
            };
            if matches!(
                integer.to_string().as_str(),
                "u8" | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
            ) {
                if repr.is_some() {
                    return Err(syn::Error::new_spanned(
                        integer,
                        "multiple integer repr types",
                    ));
                }
                repr = Some(integer.clone());
            }
        }
    }
    repr.ok_or_else(|| {
        syn::Error::new_spanned(
            ident,
            "HttpErrorCode enums must explicitly declare an integer repr, such as #[repr(u64)]",
        )
    })
}

fn parse_catalog_attrs(attrs: &[Attribute]) -> syn::Result<CatalogAttrs> {
    let mut result = CatalogAttrs::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("http")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("status") {
                if result.status.is_some() {
                    return Err(meta.error("duplicate `status` parameter"));
                }
                result.status = Some(parse_status(meta.value()?.parse()?)?);
                return Ok(());
            }
            if meta.path.is_ident("rename_all") {
                if result.rename_all.is_some() {
                    return Err(meta.error("duplicate `rename_all` parameter"));
                }
                let value: LitStr = meta.value()?.parse()?;
                result.rename_all = Some(parse_rename_rule(&value)?);
                return Ok(());
            }
            Err(meta.error("unknown enum-level http parameter"))
        })?;
    }
    Ok(result)
}

fn parse_variant_attrs(variant: &Variant) -> syn::Result<VariantAttrs> {
    let mut result = VariantAttrs::default();
    for attr in variant
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("http"))
    {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("code") {
                if result.code.is_some() {
                    return Err(meta.error("duplicate `code` parameter"));
                }
                result.code = Some(meta.value()?.parse()?);
                return Ok(());
            }
            if meta.path.is_ident("status") {
                if result.status.is_some() {
                    return Err(meta.error("duplicate `status` parameter"));
                }
                result.status = Some(parse_status(meta.value()?.parse()?)?);
                return Ok(());
            }
            Err(meta.error("unknown http parameter"))
        })?;
    }
    Ok(result)
}

fn parse_status(value: LitInt) -> syn::Result<u16> {
    let status = value.base10_parse::<u16>()?;
    if !(100..=999).contains(&status) {
        return Err(syn::Error::new_spanned(
            value,
            format!("{status} is not a valid HTTP status code"),
        ));
    }
    Ok(status)
}

fn parse_rename_rule(value: &LitStr) -> syn::Result<RenameRule> {
    match value.value().as_str() {
        "lowercase" => Ok(RenameRule::Lower),
        "UPPERCASE" => Ok(RenameRule::Upper),
        "PascalCase" => Ok(RenameRule::Pascal),
        "camelCase" => Ok(RenameRule::Camel),
        "snake_case" => Ok(RenameRule::Snake),
        "SCREAMING_SNAKE_CASE" => Ok(RenameRule::ScreamingSnake),
        "kebab-case" => Ok(RenameRule::Kebab),
        "SCREAMING-KEBAB-CASE" => Ok(RenameRule::ScreamingKebab),
        _ => Err(syn::Error::new_spanned(
            value,
            format!(
                "unsupported rename_all rule `{}`; expected one of: `lowercase`, `UPPERCASE`, \
                 `PascalCase`, `camelCase`, `snake_case`, `SCREAMING_SNAKE_CASE`, `kebab-case`, \
                 `SCREAMING-KEBAB-CASE`",
                value.value()
            ),
        )),
    }
}

fn rename(value: &str, rule: Option<RenameRule>) -> syn::Result<String> {
    let Some(rule) = rule else {
        return Ok(value.to_owned());
    };
    let case = match rule {
        RenameRule::Lower => Case::Lower,
        RenameRule::Upper => Case::Upper,
        RenameRule::Pascal => Case::Pascal,
        RenameRule::Camel => Case::Camel,
        RenameRule::Snake => Case::Snake,
        RenameRule::ScreamingSnake => Case::UpperSnake,
        RenameRule::Kebab => Case::Kebab,
        RenameRule::ScreamingKebab => Case::UpperKebab,
    };
    Ok(value.to_case(case))
}

#[cfg(test)]
mod tests {
    use super::expand;

    #[test]
    fn detected_component_path_preserves_catalog_generation() {
        let output = crate::paths::with_test_dependencies(|| {
            expand(syn::parse_quote! {
                #[repr(u64)]
                #[http(rename_all = "snake_case")]
                enum Error {
                    #[http(status = 404)]
                    NotFound = 7,
                    Next,
                }
            })
        })
        .unwrap()
        .to_string();
        assert!(output.contains(":: loso_http :: HttpErrorCode"));
        assert!(output.contains("\"not_found\""));
        assert!(output.contains("404u16"));
        assert!(output.contains(":: core :: ptr :: from_ref"));
        syn::parse2::<syn::File>(output.parse().unwrap()).unwrap();
    }

    #[test]
    fn rejects_invalid_http_attributes() {
        for (attributes, expected) in [
            ("#[http(unknown)]", "unknown enum-level http parameter"),
            (
                "#[http(code = \"CODE\")]",
                "unknown enum-level http parameter",
            ),
            ("#[http(strict)]", "unknown enum-level http parameter"),
            (
                "#[http(status = 400)] #[http(status = 500)]",
                "duplicate `status`",
            ),
            ("#[http(status = \"400\")]", "expected integer literal"),
            ("#[http(status = 99)]", "not a valid HTTP status code"),
            ("#[http(status = 1000)]", "not a valid HTTP status code"),
            ("#[http]", "expected attribute arguments"),
            ("#[http = true]", "expected parentheses"),
            (
                "#[http(crate = crate::contracts)]",
                "unknown enum-level http parameter",
            ),
            (
                "#[http(crate = \"crate::contracts\")]",
                "unknown enum-level http parameter",
            ),
            (
                "#[http(rename_all = \"snake_case\")] #[http(rename_all = \"snake_case\")]",
                "duplicate `rename_all`",
            ),
            ("#[http(crate = 7)]", "unknown enum-level http parameter"),
            ("#[http(rename_all = 7)]", "expected string literal"),
            ("#[http(status = 400,,)]", "unexpected token"),
        ] {
            let input =
                syn::parse_str(&format!("#[repr(u64)] {attributes} enum Error {{ First }}"))
                    .unwrap();
            let error = expand(input).unwrap_err().to_string();
            assert!(error.contains(expected), "{attributes}: {error}");
        }
        for (attributes, expected) in [
            ("#[http(unknown)]", "unknown http parameter"),
            ("#[http(strict)]", "unknown http parameter"),
            (
                "#[http(crate = crate::contracts)]",
                "unknown http parameter",
            ),
            (
                "#[http(rename_all = \"snake_case\")]",
                "unknown http parameter",
            ),
            ("#[http]", "expected attribute arguments"),
            ("#[http = true]", "expected parentheses"),
            (
                "#[http(code = \"CODE\")] #[http(code = \"CODE\")]",
                "duplicate `code`",
            ),
            (
                "#[http(status = 400)] #[http(status = 400)]",
                "duplicate `status`",
            ),
            ("#[http(code = 7)]", "expected string literal"),
            ("#[http(status = \"400\")]", "expected integer literal"),
            ("#[http(status = 99)]", "not a valid HTTP status code"),
        ] {
            let input =
                syn::parse_str(&format!("#[repr(u64)] enum Error {{ {attributes} First }}"))
                    .unwrap();
            let error = expand(input).unwrap_err().to_string();
            assert!(error.contains(expected), "{attributes}: {error}");
        }
    }

    #[test]
    fn rejects_a_missing_repr() {
        let input = syn::parse_quote! { enum Error { First } };
        assert!(expand(input).unwrap_err().to_string().contains("repr(u64)"));
    }

    #[test]
    fn rejects_missing_or_ambiguous_integer_repr() {
        for (attrs, expected) in [
            ("#[repr(C)]", "integer repr"),
            ("#[repr(align(8))]", "integer repr"),
            ("#[repr(u8, i64)]", "multiple integer repr"),
            ("#[repr(u8)] #[repr(u16)]", "multiple integer repr"),
        ] {
            let input = syn::parse_str(&format!("{attrs} enum Error {{ First }}")).unwrap();
            let error = expand(input).unwrap_err().to_string();
            assert!(error.contains(expected), "{attrs}: {error}");
        }
    }

    #[test]
    fn rejects_an_empty_catalog() {
        let input = syn::parse_quote! { #[repr(u64)] enum Error {} };
        assert!(
            expand(input)
                .unwrap_err()
                .to_string()
                .contains("at least one")
        );
    }

    #[test]
    fn accepts_status_boundaries_at_both_levels() {
        for status in [100, 999] {
            for source in [
                format!("#[repr(u64)] #[http(status = {status})] enum Error {{ First }}"),
                format!("#[repr(u64)] enum Error {{ #[http(status = {status})] First }}"),
            ] {
                let output = crate::paths::with_test_dependencies(|| {
                    expand(syn::parse_str(&source).unwrap())
                })
                .unwrap()
                .to_string();
                assert!(output.contains(&format!("{status}u16")));
            }
        }
    }

    #[test]
    fn invalid_rename_all_lists_the_supported_rules() {
        let input = syn::parse_quote! {
            #[repr(u64)]
            #[http(rename_all = "SCREAMING_CAMEL_CASE")]
            enum Error { First }
        };

        let message = expand(input).unwrap_err().to_string();
        assert!(message.contains("unsupported rename_all rule `SCREAMING_CAMEL_CASE`"));
        for rule in [
            "lowercase",
            "UPPERCASE",
            "PascalCase",
            "camelCase",
            "snake_case",
            "SCREAMING_SNAKE_CASE",
            "kebab-case",
            "SCREAMING-KEBAB-CASE",
        ] {
            assert!(message.contains(rule), "diagnostic omitted `{rule}`");
        }
    }
}
