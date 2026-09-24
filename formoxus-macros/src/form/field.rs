//! `<path> => { … }` — one field's body.

use syn::{
    Expr, Ident, Result, Token, braced,
    parse::{Parse, ParseStream},
};

use super::{SpecPath, WidgetRef};

#[derive(Debug)]
pub(crate) struct FieldSpec {
    pub(crate) path: SpecPath,
    pub(crate) label: Option<Expr>,
    pub(crate) widget: Option<WidgetRef>,
}

#[derive(Debug)]
pub(crate) struct FieldBody {
    pub(crate) widget: Option<WidgetRef>,
    pub(crate) label: Option<Expr>,
}

impl Parse for FieldBody {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if !input.peek(syn::token::Brace) {
            return Err(input.error("expected a field specification surrounded by braces"));
        }
        let body;
        let braces = braced!(body in input);
        let mut fb = FieldBody {
            widget: None,
            label: None,
        };
        while !body.is_empty() {
            let key: Ident = body.parse()?;
            let _colon: Token![:] = body.parse()?;
            match key.to_string().as_str() {
                "widget" if fb.widget.is_some() => {
                    return Err(syn::Error::new_spanned(&key, "duplicate widget key"));
                }
                "widget" => fb.widget = Some(body.parse()?),
                "label" if fb.label.is_some() => {
                    return Err(syn::Error::new_spanned(&key, "duplicate label"));
                }
                "label" => fb.label = Some(body.parse()?),
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!("unknown key {other}, expected widget or label"),
                    ));
                }
            }
            if body.peek(Token![,]) {
                body.parse::<Token![,]>()?;
            }
        }

        if fb.widget.is_none() && fb.label.is_none() {
            return Err(syn::Error::new(braces.span.join(), "empty field body"));
        }
        Ok(fb)
    }
}

#[cfg(test)]
mod tests {
    use crate::form::tests::err_of;
    use googletest::prelude::*;
    use quote::quote;

    // ── Field-body errors ─────────────────────────────────────────────────

    #[gtest]
    fn an_empty_field_body_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes => {} } }),
            contains_substring("empty field body")
        );
    }

    #[gtest]
    fn an_unknown_key_names_itself() {
        let msg = err_of(quote! { Source { notes => { contrl: textarea } } });
        expect_that!(msg, contains_substring("contrl"));
        expect_that!(msg, contains_substring("widget"));
    }

    #[gtest]
    fn a_duplicate_key_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes => { label: "a", label: "b" } } }),
            contains_substring("duplicate")
        );
    }

    #[gtest]
    fn a_field_body_must_be_braced() {
        expect_that!(
            err_of(quote! { Source { notes => textarea } }),
            contains_substring("braces")
        );
    }
}
