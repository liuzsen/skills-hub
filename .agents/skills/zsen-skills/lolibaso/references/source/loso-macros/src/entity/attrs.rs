#[derive(Clone, Default)]
pub struct ContainerAttrs {
    pub patchable: bool,
    pub no_state: bool,
    pub common: Vec<syn::Attribute>,
    pub patch: Vec<syn::Attribute>,
    pub state: Vec<syn::Attribute>,
    pub readonly: Vec<syn::Attribute>,
}

impl ContainerAttrs {
    pub fn parse_from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut result = Self::default();

        for attr in attrs {
            let path = attr.path();
            if path.is_ident("doc") {
                continue;
            }

            if path.is_ident("entity") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("patchable") {
                        if result.patchable {
                            return Err(meta.error("duplicate `patchable` parameter"));
                        }
                        result.patchable = true;
                    } else if meta.path.is_ident("no_state") {
                        if result.no_state {
                            return Err(meta.error("duplicate `no_state` parameter"));
                        }
                        result.no_state = true;
                    } else if meta.path.is_ident("id") || meta.path.is_ident("patch") {
                        return Err(
                            meta.error("`id` and `patch` are only supported on Entity fields")
                        );
                    } else {
                        return Err(meta.error(
                            "unsupported Entity option; expected `patchable` or `no_state`",
                        ));
                    }
                    Ok(())
                })?;
            } else if path.is_ident("patch") {
                Self::parse_nested_attrs(attr, &mut result.patch)?;
            } else if path.is_ident("state") {
                Self::parse_nested_attrs(attr, &mut result.state)?;
            } else if path.is_ident("readonly") {
                Self::parse_nested_attrs(attr, &mut result.readonly)?;
            } else {
                result.common.push(attr.clone());
            }
        }

        Ok(result)
    }

    fn parse_nested_attrs(
        attr: &syn::Attribute,
        target: &mut Vec<syn::Attribute>,
    ) -> syn::Result<()> {
        let nested = attr.parse_args_with(
            syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
        )?;
        for meta in nested {
            if !meta.path().is_ident("doc") {
                target.push(syn::parse_quote!(#[#meta]));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ContainerAttrs;
    use quote::quote;

    fn parse(attributes: &str) -> syn::Result<ContainerAttrs> {
        let input: syn::DeriveInput =
            syn::parse_str(&format!("{attributes} struct Example {{ id: u64 }}")).unwrap();
        ContainerAttrs::parse_from_attrs(&input.attrs)
    }

    #[test]
    fn parses_entity_state_options_across_attributes() {
        let attrs = parse("#[entity()] #[entity(patchable,)]").unwrap();
        assert!(attrs.patchable);
        assert!(parse("#[entity(no_state)]").unwrap().no_state);
        // Wrappers without a generated target remain valid.
        assert!(
            parse("#[entity(no_state)] #[state(derive(Clone))] #[patch(derive(Clone))]").is_ok()
        );
        assert!(parse("#[state(derive(Clone))]").is_ok());
    }

    #[test]
    fn preserves_wrapper_meta_forms_order_and_doc_filtering() {
        let attrs = parse(
            r#"
            #[doc = "common doc"]
            #[allow(dead_code)]
            #[state(derive(Clone), repr(C),)]
            #[patch(derive(Debug, PartialEq))]
            #[readonly()]
            #[readonly(some::marker, some::option = "value", some::nested(key = "value"))]
            #[readonly(doc = "hidden", cfg_attr(any(), doc = "nested"))]
            #[readonly(derive(Debug))]
        "#,
        )
        .unwrap();
        let common = &attrs.common;
        let state = &attrs.state;
        let patch = &attrs.patch;
        let readonly = &attrs.readonly;
        assert_eq!(
            quote!(#(#common)*).to_string(),
            quote!(#[allow(dead_code)]).to_string()
        );
        assert_eq!(
            quote!(#(#state)*).to_string(),
            quote!(#[derive(Clone)] #[repr(C)]).to_string()
        );
        assert_eq!(
            quote!(#(#patch)*).to_string(),
            quote!(#[derive(Debug, PartialEq)]).to_string()
        );
        assert_eq!(
            quote!(#(#readonly)*).to_string(),
            quote!(
                #[some::marker]
                #[some::option = "value"]
                #[some::nested(key = "value")]
                #[cfg_attr(any(), doc = "nested")]
                #[derive(Debug)]
            )
            .to_string()
        );
        assert!(parse("#[readonly(derive(Debug))] #[readonly(derive(Debug))]").is_ok());
    }

    #[test]
    fn rejects_legacy_crate_overrides() {
        for value in ["crate::contracts", "\"crate::contracts\"", "::loso_entity"] {
            let error = parse(&format!("#[entity(crate = {value})]"))
                .err()
                .expect("legacy crate overrides must be rejected");
            assert_eq!(
                error.to_string(),
                "unsupported Entity option; expected `patchable` or `no_state`"
            );
        }
    }

    #[test]
    fn rejects_invalid_container_configuration() {
        for (attributes, expected) in [
            ("#[entity(unknown)]", "unsupported Entity option"),
            ("#[entity(id)]", "only supported on Entity fields"),
            ("#[entity(patch)]", "only supported on Entity fields"),
            ("#[entity]", "expected attribute arguments"),
            ("#[entity = true]", "expected parentheses"),
            ("#[entity(patchable = true)]", "expected `,`"),
            ("#[entity(no_state(extra))]", "expected `,`"),
            ("#[entity(crate = 7)]", "unsupported Entity option"),
            (
                "#[entity(crate = \"not a path\")]",
                "unsupported Entity option",
            ),
            (
                "#[entity(state = \"CustomState\")]",
                "unsupported Entity option",
            ),
            (
                "#[entity(state = CustomState, no_state)]",
                "unsupported Entity option",
            ),
            (
                "#[entity(no_state)] #[entity(state = CustomState)]",
                "unsupported Entity option",
            ),
            (
                "#[entity(crate = crate::one)] #[entity(crate = crate::two)]",
                "unsupported Entity option",
            ),
            (
                "#[entity(patchable)] #[entity(patchable)]",
                "duplicate `patchable`",
            ),
            (
                "#[entity(no_state)] #[entity(no_state)]",
                "duplicate `no_state`",
            ),
            (
                "#[entity(state = One)] #[entity(state = Two)]",
                "unsupported Entity option",
            ),
            ("#[entity(patchable,,)]", "unexpected token"),
            ("#[state]", "expected attribute arguments"),
            ("#[patch = true]", "expected parentheses"),
            ("#[readonly(derive(Debug), =)]", "expected identifier"),
            ("#[state(derive(Clone) derive(Debug))]", "expected `,`"),
        ] {
            let error = parse(attributes).err().expect(attributes).to_string();
            assert!(error.contains(expected), "{attributes}: {error}");
        }
    }
}
