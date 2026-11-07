use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use std::collections::HashSet;

#[derive(Clone)]
pub(super) struct Names {
    reserved: HashSet<String>,
    next: usize,
}

impl Names {
    pub fn new(tokens: TokenStream) -> Self {
        fn collect(tokens: TokenStream, names: &mut HashSet<String>) {
            for token in tokens {
                match token {
                    TokenTree::Ident(ident) => {
                        names.insert(ident.to_string().trim_start_matches("r#").to_owned());
                    }
                    TokenTree::Group(group) => collect(group.stream(), names),
                    _ => {}
                }
            }
        }
        let mut reserved = HashSet::new();
        collect(tokens, &mut reserved);
        Self { reserved, next: 0 }
    }

    pub fn fresh(&mut self) -> Ident {
        loop {
            let name = format!("__loso_private_{}", self.next);
            self.next += 1;
            if self.reserved.insert(name.clone()) {
                return Ident::new(&name, Span::mixed_site());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    #[test]
    fn reserves_nested_raw_and_lifetime_tokens() {
        let mut names = Names::new(
            quote!(r#__loso_private_0 macro_call!('__loso_private_1 [__loso_private_2])),
        );
        assert_eq!(names.fresh(), "__loso_private_3");
        assert_eq!(names.fresh(), "__loso_private_4");
    }
}
