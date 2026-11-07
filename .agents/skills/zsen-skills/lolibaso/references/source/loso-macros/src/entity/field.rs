use syn::{Ident, Type};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Id,
    Normal,
}

#[derive(Clone)]
pub struct EntityField {
    pub name: Ident,
    pub ty: Type,
    pub kind: FieldKind,
    pub patch: bool,
}

impl EntityField {
    pub fn from_syn_field(field: &syn::Field) -> syn::Result<Self> {
        let name = field.ident.clone().ok_or_else(|| {
            syn::Error::new_spanned(field, "Entity must be a struct with named fields")
        })?;
        let mut is_id = false;
        let mut patch = false;

        for attr in &field.attrs {
            if attr.path().is_ident("entity") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("id") {
                        if is_id {
                            return Err(meta.error("duplicate `id` parameter"));
                        }
                        is_id = true;
                    } else if meta.path.is_ident("patch") {
                        if patch {
                            return Err(meta.error("duplicate `patch` parameter"));
                        }
                        patch = true;
                    } else if meta.path.is_ident("biz_id") {
                        return Err(meta.error("`#[entity(biz_id)]` was removed; implement `FindBy<Entity, Key>` on the repository"));
                    } else {
                        return Err(meta.error("unsupported Entity field option; expected `id` or `patch`"));
                    }
                    Ok(())
                })?;
            } else if attr.path().is_ident("state")
                || attr.path().is_ident("patch")
                || attr.path().is_ident("readonly")
            {
                return Err(syn::Error::new_spanned(
                    attr,
                    "Entity attribute wrappers are only supported on the struct",
                ));
            }
        }

        Ok(EntityField {
            name,
            ty: field.ty.clone(),
            kind: if is_id {
                FieldKind::Id
            } else {
                FieldKind::Normal
            },
            patch,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{EntityField, FieldKind};

    fn parse(attributes: &str) -> syn::Result<EntityField> {
        let input: syn::DeriveInput =
            syn::parse_str(&format!("struct Example {{ {attributes} key: u64 }}")).unwrap();
        let syn::Data::Struct(data) = input.data else {
            unreachable!()
        };
        EntityField::from_syn_field(data.fields.iter().next().unwrap())
    }

    #[test]
    fn parses_independent_field_markers() {
        for attributes in ["#[entity(id)] #[entity(patch)]", "#[entity(id, patch,)]"] {
            let field = parse(attributes).unwrap();
            assert_eq!(field.kind, FieldKind::Id);
            assert!(field.patch);
        }
        assert_eq!(parse("#[entity(id,)]").unwrap().kind, FieldKind::Id);
        assert!(parse("#[entity(patch,)]").unwrap().patch);
        assert_eq!(parse("#[entity()]").unwrap().kind, FieldKind::Normal);
        assert!(parse("#[some::other(anything)]").is_ok());
    }

    #[test]
    fn rejects_invalid_field_configuration() {
        for (attributes, expected) in [
            ("#[entity(unknown)]", "unsupported Entity field option"),
            (
                "#[entity(crate = crate::contracts)]",
                "unsupported Entity field option",
            ),
            ("#[entity(patchable)]", "unsupported Entity field option"),
            (
                "#[entity(state = State)]",
                "unsupported Entity field option",
            ),
            ("#[entity(no_state)]", "unsupported Entity field option"),
            ("#[entity]", "expected attribute arguments"),
            ("#[entity = true]", "expected parentheses"),
            ("#[entity(id = true)]", "expected `,`"),
            ("#[entity(patch(extra))]", "expected `,`"),
            ("#[entity(id,,)]", "unexpected token"),
            ("#[entity(id)] #[entity(id)]", "duplicate `id`"),
            ("#[entity(patch, patch)]", "duplicate `patch`"),
            ("#[entity(biz_id)]", "was removed"),
            ("#[entity(patch, biz_id)]", "was removed"),
            ("#[state(derive(Clone))]", "only supported on the struct"),
            ("#[patch(derive(Clone))]", "only supported on the struct"),
            ("#[readonly(derive(Clone))]", "only supported on the struct"),
        ] {
            let error = parse(attributes).err().expect(attributes).to_string();
            assert!(error.contains(expected), "{attributes}: {error}");
        }
    }
}
