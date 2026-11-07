// Copyright (c) 2025
//
// SPDX-License-Identifier: MIT

use std::collections::HashSet;

use syn::{Attribute, GenericParam, Generics, Meta, Type, visit::Visit};

/// Names retain the caller's shape: optional for fields, required for parameters.
#[derive(Clone)]
pub(super) struct Dependency<N, K = DependencyKind> {
    pub name: N,
    pub ty: Type,
    pub kind: K,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DependencyKind {
    Instance,
    InstanceClone,
    Transient,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FieldKind {
    Default,
    Instance,
    InstanceClone,
    Transient,
}

impl From<DependencyKind> for FieldKind {
    fn from(kind: DependencyKind) -> Self {
        match kind {
            DependencyKind::Instance => Self::Instance,
            DependencyKind::InstanceClone => Self::InstanceClone,
            DependencyKind::Transient => Self::Transient,
        }
    }
}

impl FieldKind {
    pub fn is_default(&self) -> bool {
        matches!(self, Self::Default)
    }
}

pub(super) struct DependencyClassifier {
    generic_params: HashSet<String>,
}

impl DependencyClassifier {
    pub fn new(generics: &Generics) -> Self {
        Self {
            generic_params: generics
                .params
                .iter()
                .filter_map(|param| match param {
                    GenericParam::Type(param) => Some(param.ident.to_string()),
                    _ => None,
                })
                .collect(),
        }
    }

    pub fn classify(
        &self,
        ty: &Type,
        instance_clone: bool,
        generic_clone_error: &str,
    ) -> syn::Result<DependencyKind> {
        let mut checker = GenericParamChecker {
            generic_params: &self.generic_params,
            found_generic: false,
        };
        checker.visit_type(ty);
        match (checker.found_generic, instance_clone) {
            (true, true) => Err(syn::Error::new_spanned(ty, generic_clone_error)),
            (true, false) => Ok(DependencyKind::Transient),
            (false, true) => Ok(DependencyKind::InstanceClone),
            (false, false) => Ok(DependencyKind::Instance),
        }
    }
}

struct GenericParamChecker<'a> {
    generic_params: &'a HashSet<String>,
    found_generic: bool,
}

impl Visit<'_> for GenericParamChecker<'_> {
    fn visit_path(&mut self, path: &syn::Path) {
        if let Some(segment) = path.segments.first()
            && self.generic_params.contains(&segment.ident.to_string())
        {
            self.found_generic = true;
        }
        syn::visit::visit_path(self, path);
    }
}

pub(super) fn field_kind(
    attrs: &[Attribute],
    ty: &Type,
    classifier: &DependencyClassifier,
) -> syn::Result<FieldKind> {
    let mut option = None;
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("inject")) {
        let Meta::List(list) = &attr.meta else {
            return Err(syn::Error::new_spanned(
                attr,
                "expected `#[inject(default)]` or `#[inject(instance_clone)]`",
            ));
        };
        if list.tokens.is_empty() {
            return Err(syn::Error::new_spanned(
                attr,
                "expected an injection option",
            ));
        }
        attr.parse_nested_meta(|meta| {
            let kind = if meta.path.is_ident("default") {
                FieldKind::Default
            } else if meta.path.is_ident("instance_clone") {
                FieldKind::InstanceClone
            } else if meta.path.is_ident("instance") {
                return Err(meta.error("unsupported field injection attribute `instance`; supported attributes are `default` and `instance_clone`"));
            } else {
                return Err(meta.error("unsupported field injection attribute; supported attributes are `default` and `instance_clone`"));
            };
            if !meta.input.is_empty() && !meta.input.peek(syn::Token![,]) {
                return Err(meta.error("injection options do not accept arguments or values"));
            }
            if let Some(previous) = option {
                return Err(meta.error(if previous == kind {
                    "duplicate injection option"
                } else {
                    "conflicting injection options `default` and `instance_clone`"
                }));
            }
            option = Some(kind);
            Ok(())
        })?;
    }
    match option {
        Some(FieldKind::Default) => Ok(FieldKind::Default),
        other => classifier
            .classify(ty, other.is_some(), "`instance_clone` is only supported on concrete fields; generic fields support `default` or no injection attribute")
            .map(FieldKind::from),
    }
}

pub(super) fn parameter_clone(attrs: &[Attribute]) -> syn::Result<bool> {
    let mut instance_clone = false;
    for attr in attrs {
        if let Some(ident) = attr.path().get_ident()
            && (ident == "default" || ident == "instance")
        {
            return Err(syn::Error::new_spanned(
                attr,
                format!(
                    "unsupported custom constructor parameter attribute `{ident}`; only `instance_clone` is supported on concrete parameters"
                ),
            ));
        }
        if attr.path().is_ident("inject") {
            return Err(syn::Error::new_spanned(
                attr,
                "constructor parameters use bare `#[instance_clone]`, not `#[inject(...)]`",
            ));
        }
        if attr.path().is_ident("instance_clone") {
            if !matches!(attr.meta, Meta::Path(_)) {
                return Err(syn::Error::new_spanned(
                    attr,
                    "expected bare `#[instance_clone]`",
                ));
            }
            if instance_clone {
                return Err(syn::Error::new_spanned(
                    attr,
                    "duplicate `instance_clone` attribute",
                ));
            }
            instance_clone = true;
        }
    }
    Ok(instance_clone)
}
