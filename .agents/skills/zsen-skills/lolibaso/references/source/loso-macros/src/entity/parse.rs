use syn::{DeriveInput, Visibility};

use crate::entity::attrs::ContainerAttrs;
use crate::entity::field::{EntityField, FieldKind};

#[derive(Clone)]
pub struct EntityStruct {
    pub name: syn::Ident,
    pub crate_path: syn::Path,
    pub vis: Visibility,
    pub generics: syn::Generics,
    pub id_field: EntityField,
    pub fields: Vec<EntityField>,
    pub attrs: ContainerAttrs,
}

impl EntityStruct {
    pub fn patch_name(&self) -> syn::Ident {
        quote::format_ident!("{}Patch", self.name)
    }

    pub fn state_name(&self) -> syn::Ident {
        quote::format_ident!("{}State", self.name)
    }

    pub fn readonly_name(&self) -> syn::Ident {
        quote::format_ident!("{}Readonly", self.name)
    }
}

pub fn parse_entity(input: &DeriveInput) -> syn::Result<EntityStruct> {
    validate_generics(&input.generics)?;

    let name = input.ident.clone();
    let vis = match &input.vis {
        Visibility::Public(_) => input.vis.clone(),
        _ => syn::parse_quote!(pub),
    };
    let generics = input.generics.clone();

    let attrs = ContainerAttrs::parse_from_attrs(&input.attrs)?;

    let data_struct = match &input.data {
        syn::Data::Struct(s) => s,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Entity can only be derived for structs",
            ));
        }
    };

    let struct_fields = match &data_struct.fields {
        syn::Fields::Named(f) => &f.named,
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "Entity must be a struct with named fields",
            ));
        }
    };

    if struct_fields.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "Entity must have at least one field",
        ));
    }

    let mut parsed_fields = Vec::new();
    let mut explicit_id = false;
    for field in struct_fields.iter() {
        let entity_field = EntityField::from_syn_field(field)?;
        if entity_field.kind == FieldKind::Id {
            if explicit_id {
                return Err(syn::Error::new_spanned(
                    field,
                    "Entity must not have multiple fields marked with #[entity(id)]",
                ));
            }
            explicit_id = true;
        }
        parsed_fields.push(entity_field);
    }

    let id_field_index = parsed_fields
        .iter()
        .position(|f| f.kind == FieldKind::Id)
        .or_else(|| parsed_fields.iter().position(|f| f.name == "id"))
        .ok_or_else(|| {
            syn::Error::new_spanned(
                &input.ident,
                "Entity must have one field marked with #[entity(id)] or a field named 'id'",
            )
        })?;

    let id_field = parsed_fields.remove(id_field_index);

    let crate_path = crate::paths::resolve("loso-entity", "entity")?;

    Ok(EntityStruct {
        name,
        crate_path,
        vis,
        generics,
        id_field,
        fields: parsed_fields,
        attrs,
    })
}

fn validate_generics(generics: &syn::Generics) -> syn::Result<()> {
    if generics.params.is_empty() {
        Ok(())
    } else {
        Err(syn::Error::new_spanned(
            &generics.params,
            "#[derive(Entity)] does not support generic parameters",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_entity, validate_generics};

    #[test]
    fn selects_explicit_identity_before_name_fallback() {
        let entity = crate::paths::with_test_dependencies(|| {
            parse_entity(&syn::parse_quote! {
                struct Example {
                    id: String,
                    #[entity(id)]
                    #[entity(patch)]
                    key: u64,
                    #[entity(patch)]
                    value: String,
                }
            })
        })
        .unwrap();
        assert_eq!(entity.id_field.name, "key");
        assert!(entity.id_field.patch);
        assert!(!entity.attrs.patchable);
        assert!(entity.fields[1].patch);
        assert_eq!(
            entity
                .fields
                .iter()
                .map(|f| f.name.to_string())
                .collect::<Vec<_>>(),
            ["id", "value"]
        );

        let fallback = crate::paths::with_test_dependencies(|| {
            parse_entity(&syn::parse_quote! {
                struct Example { value: String, id: u64 }
            })
        })
        .unwrap();
        assert_eq!(fallback.id_field.name, "id");
        let crate_path = &fallback.crate_path;
        assert_eq!(quote::quote!(#crate_path).to_string(), ":: loso_entity");
    }

    #[test]
    fn rejects_invalid_entity_shapes_and_identity() {
        for (source, expected) in [
            ("enum Example { A }", "only be derived for structs"),
            ("union Example { id: u64 }", "only be derived for structs"),
            ("struct Example(u64);", "named fields"),
            ("struct Example;", "named fields"),
            ("struct Example {}", "at least one field"),
            ("struct Example { value: u64 }", "one field marked"),
            (
                "struct Example<T> { id: T }",
                "does not support generic parameters",
            ),
            (
                "struct Example { #[entity(id)] one: u64, #[entity(id)] two: u64 }",
                "multiple fields marked",
            ),
        ] {
            let input = syn::parse_str(source).unwrap();
            let error = parse_entity(&input).err().expect(source).to_string();
            assert!(error.contains(expected), "{source}: {error}");
        }
    }

    #[test]
    fn entity_generics_have_a_clear_diagnostic() {
        for generics in ["<T>", "<'a>", "<const N: usize>"] {
            let generics: syn::Generics = syn::parse_str(generics).unwrap();
            let error = validate_generics(&generics).unwrap_err();

            assert_eq!(
                error.to_string(),
                "#[derive(Entity)] does not support generic parameters"
            );
        }
    }
}
