use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;

pub fn resolve(component: &str, module: &str) -> syn::Result<syn::Path> {
    resolve_with(component, module, |name| {
        #[cfg(test)]
        if TEST_DEPENDENCIES.get() {
            return Some(FoundCrate::Name(name.replace('-', "_")));
        }
        crate_name(name).ok()
    })
}

fn resolve_with(
    component: &str,
    module: &str,
    mut lookup: impl FnMut(&str) -> Option<FoundCrate>,
) -> syn::Result<syn::Path> {
    if let Some(found) = lookup(component) {
        return root(found);
    }
    if let Some(found) = lookup("lolibaso") {
        let root = root(found)?;
        let module: syn::Path = syn::parse_str(module)?;
        return syn::parse2(quote!(#root::#module));
    }
    Err(syn::Error::new(
        proc_macro2::Span::call_site(),
        format!("cannot locate `{component}` or `lolibaso`; add one to Cargo.toml"),
    ))
}

fn root(found: FoundCrate) -> syn::Result<syn::Path> {
    match found {
        FoundCrate::Itself => Ok(syn::parse_quote!(crate)),
        FoundCrate::Name(name) => syn::parse_str(&format!("::{name}")),
    }
}

#[cfg(test)]
std::thread_local! {
    static TEST_DEPENDENCIES: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

// Macro unit tests run without runtime dependencies in loso-macros/Cargo.toml.
#[cfg(test)]
pub(crate) fn with_test_dependencies<T>(test: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) {
            TEST_DEPENDENCIES.set(self.0);
        }
    }
    let _restore = Restore(TEST_DEPENDENCIES.replace(true));
    test()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(path: syn::Path) -> String {
        quote!(#path).to_string()
    }

    #[test]
    fn resolves_dependency_matrix() {
        for (component, module) in [
            ("loso-entity", "entity"),
            ("loso-http", "http::error"),
            ("loso-inject", "inject"),
        ] {
            for alias in [component.replace('-', "_"), "renamed".into()] {
                let path = resolve_with(component, module, |name| {
                    Some(FoundCrate::Name(if name == component {
                        alias.clone()
                    } else {
                        "facade".into()
                    }))
                })
                .unwrap();
                assert_eq!(text(path), format!(":: {alias}"));
            }
            let path = resolve_with(component, module, |name| {
                (name == component).then_some(FoundCrate::Itself)
            })
            .unwrap();
            assert_eq!(text(path), "crate");
            for facade in ["lolibaso", "renamed_facade"] {
                let path = resolve_with(component, module, |name| {
                    (name == "lolibaso").then(|| FoundCrate::Name(facade.into()))
                })
                .unwrap();
                assert_eq!(
                    text(path),
                    format!(":: {facade} :: {}", module.replace("::", " :: "))
                );
            }
            let path = resolve_with(component, module, |name| {
                (name == "lolibaso").then_some(FoundCrate::Itself)
            })
            .unwrap();
            assert_eq!(
                text(path),
                format!("crate :: {}", module.replace("::", " :: "))
            );
        }
    }

    #[test]
    fn test_dependencies_are_scoped() {
        assert!(!TEST_DEPENDENCIES.get());
        with_test_dependencies(|| {
            assert_eq!(
                text(resolve("loso-entity", "entity").unwrap()),
                ":: loso_entity"
            );
        });
        assert!(!TEST_DEPENDENCIES.get());
    }

    #[test]
    fn missing_dependencies_request_a_cargo_dependency() {
        let error = resolve_with("loso-entity", "entity", |_| None)
            .err()
            .unwrap();
        assert!(error.to_string().contains("add one to Cargo.toml"));
    }
}
