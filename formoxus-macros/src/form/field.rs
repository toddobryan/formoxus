//! `<path> => { … }` — one field's body.

use std::collections::HashSet;

use facet::Facet;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, Ident, LitStr, Result, Token, braced,
    parse::{Parse, ParseStream},
};

use super::{SpecPath, WidgetRef};

#[derive(Debug)]
pub(crate) struct FieldSpec {
    pub(crate) path: SpecPath,
    pub(crate) body: FieldBody,
}

/// One field's brace block. **These fields ARE the grammar** — the list of
/// legal keys is read back off this struct's [`Facet`] shape by [`legal_keys`],
/// so an error message cannot advertise a key that does not exist, and a new
/// key cannot be added without appearing in that message.
///
/// `#[facet(opaque)]` on every field because none of these types implement
/// `Facet` — `SHAPE` is wanted for the field NAMES, never to look inside a
/// value. Forgetting one is a loud trait-bound error, not a silent omission.
///
/// **Why `Expr` for the bounds and `LitStr` for the pattern.** A bound may be
/// any expression (`min: 13`, `min: MIN_AGE`, `min: 2 * N`), and holding it as
/// an `Expr` is also what lets the literal's own type pick the `Bound` variant
/// once it reaches `.into()`. A pattern cannot: the macro has to hand the
/// string to `regress` to check it compiles, and only a literal is readable at
/// macro time. It also lands in `Constraints::pattern`, which is a
/// `&'static str`.
#[derive(Debug, Default, Facet)]
pub(crate) struct FieldBody {
    #[facet(opaque)]
    pub(crate) widget: Option<WidgetRef>,
    #[facet(opaque)]
    pub(crate) label: Option<Expr>,
    #[facet(opaque)]
    pub(crate) min: Option<Expr>,
    #[facet(opaque)]
    pub(crate) max: Option<Expr>,
    #[facet(opaque)]
    pub(crate) min_length: Option<Expr>,
    #[facet(opaque)]
    pub(crate) max_length: Option<Expr>,
    #[facet(opaque)]
    pub(crate) pattern: Option<LitStr>,
}

/// Every key a field body accepts, in declaration order — read off
/// [`FieldBody`] itself, so the list and the struct cannot drift.
///
/// It does not make the `match` in `parse` exhaustive; a field with no arm
/// would simply never be set. `every_legal_key_parses` closes that, and takes
/// its input from here, so the test cannot drift either.
pub(crate) fn legal_keys() -> Vec<&'static str> {
    match FieldBody::SHAPE.ty {
        facet::Type::User(facet::UserType::Struct(s)) => s.fields.iter().map(|f| f.name).collect(),
        _ => unreachable!("FieldBody is a struct"),
    }
}

impl FieldBody {
    /// The `Constraints` literal this body declares, or `None` when it declares
    /// none — in which case no `.with_constraints` call is emitted at all and a
    /// form without constraints expands exactly as it did before they existed.
    ///
    /// One call taking the whole struct, not five setters: the brace block in
    /// the source and the struct literal in the expansion are the same shape,
    /// so a key fills the field of the same name and the macro never has to
    /// decide which setter a key maps to.
    pub(crate) fn constraints_tokens(&self) -> Option<TokenStream2> {
        // `(#e).into()` — parenthesized because `#e` may be any expression, and
        // the literal's own type is what picks `Bound::Int` over `Bound::Float`.
        let min = self
            .min
            .as_ref()
            .map(|e| quote! { min: Some((#e).into()), });
        let max = self
            .max
            .as_ref()
            .map(|e| quote! { max: Some((#e).into()), });
        let min_length = self
            .min_length
            .as_ref()
            .map(|e| quote! { min_length: Some(#e), });
        let max_length = self
            .max_length
            .as_ref()
            .map(|e| quote! { max_length: Some(#e), });
        let pattern = self.pattern.as_ref().map(|s| quote! { pattern: Some(#s), });

        if min.is_none()
            && max.is_none()
            && min_length.is_none()
            && max_length.is_none()
            && pattern.is_none()
        {
            return None;
        }
        Some(quote! {
            ::formoxus::fields::Constraints {
                #min #max #min_length #max_length #pattern
                ..::core::default::Default::default()
            }
        })
    }
}

impl Parse for FieldBody {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if !input.peek(syn::token::Brace) {
            return Err(input.error("expected a field specification surrounded by braces"));
        }
        let body;
        let braces = braced!(body in input);
        let mut fb = FieldBody::default();
        // `insert` returning false IS the duplicate check, so there is no
        // separate seen-flag per key to keep in step with the arms below.
        let mut seen: HashSet<String> = HashSet::new();
        while !body.is_empty() {
            let key: Ident = body.parse()?;
            let name = key.to_string();
            let _colon: Token![:] = body.parse()?;
            if !seen.insert(name.clone()) {
                return Err(syn::Error::new_spanned(
                    &key,
                    format!("`{name}` is given twice"),
                ));
            }
            match name.as_str() {
                "widget" => fb.widget = Some(body.parse()?),
                "label" => fb.label = Some(body.parse()?),
                "min" => fb.min = Some(body.parse()?),
                "max" => fb.max = Some(body.parse()?),
                "min_length" => fb.min_length = Some(body.parse()?),
                "max_length" => fb.max_length = Some(body.parse()?),
                "pattern" => fb.pattern = Some(body.parse()?),
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!(
                            "unknown key {other}, expected one of: {}",
                            legal_keys().join(", ")
                        ),
                    ));
                }
            }
            if body.peek(Token![,]) {
                body.parse::<Token![,]>()?;
            }
        }

        // `seen`, not a check per field: this asks "were there any keys?",
        // which is the actual question and cannot go stale when a key is added.
        if seen.is_empty() {
            return Err(syn::Error::new(braces.span.join(), "empty field body"));
        }
        Ok(fb)
    }
}

#[cfg(test)]
mod tests {
    use super::legal_keys;
    use crate::form::tests::{err_of, parse};
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

    /// Worded to match the form-level message, so the same mistake reads the
    /// same at both levels.
    #[gtest]
    fn a_duplicate_key_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes => { label: "a", label: "b" } } }),
            contains_substring("`label` is given twice")
        );
    }

    /// One duplicate check covers every key, so this is not `label`'s test
    /// repeated — it is the evidence that a new key needs no new check.
    #[gtest]
    fn the_duplicate_check_covers_a_constraint_key_too() {
        expect_that!(
            err_of(quote! { Source { notes => { max_length: 1, max_length: 2 } } }),
            contains_substring("`max_length` is given twice")
        );
    }

    #[gtest]
    fn a_field_body_must_be_braced() {
        expect_that!(
            err_of(quote! { Source { notes => textarea } }),
            contains_substring("braces")
        );
    }

    // ── The keys are the struct ──────────────────────────────────────────

    /// **The other half of `legal_keys`.** Reading names off `FieldBody`'s
    /// shape stops the error message advertising a key that does not exist; it
    /// does NOT stop a field being declared with no arm in `parse`, which would
    /// leave it silently `None`. This walks the same list the message does and
    /// proves each name is actually accepted.
    ///
    /// Adding a key with no sample here panics by name rather than passing
    /// quietly, which is the point.
    #[gtest]
    fn every_legal_key_parses() {
        for key in legal_keys() {
            let value = match key {
                "widget" => quote!(textarea),
                "label" => quote!("A label"),
                "min" | "max" | "min_length" | "max_length" => quote!(1),
                "pattern" => quote!("x"),
                other => panic!("no sample value for the new key `{other}` — add one here"),
            };
            let ident = syn::Ident::new(key, proc_macro2::Span::call_site());
            let parsed = parse(quote! { Source { notes => { #ident: #value } } });
            expect_that!(
                parsed.is_ok(),
                eq(true),
                "`{key}` should be an accepted key"
            );
        }
    }

    #[gtest]
    fn an_unknown_key_lists_every_legal_one() {
        let msg = err_of(quote! { Source { notes => { maxlen: 3 } } });
        for key in legal_keys() {
            expect_that!(
                msg,
                contains_substring(key),
                "the message should name `{key}`"
            );
        }
    }

    // ── Constraints reach the expansion ──────────────────────────────────

    #[gtest]
    fn constraint_keys_become_one_with_constraints_call() {
        let spec = parse(quote! {
            Source { notes => { min_length: 3, max_length: 500, pattern: "\\w+" } }
        })
        .unwrap();
        let tokens = spec.expand().to_string();

        // ONE call, holding a struct literal — not one call per key.
        expect_that!(tokens.matches("with_constraints").count(), eq(1));
        expect_that!(tokens, contains_substring("min_length : Some (3)"));
        expect_that!(tokens, contains_substring("max_length : Some (500)"));
        expect_that!(tokens, contains_substring("pattern : Some"));
    }

    /// A bound goes through `.into()`, which is what lets the literal's own
    /// type decide between `Bound::Int` and `Bound::Float` — the macro never
    /// inspects the token to choose.
    #[gtest]
    fn a_bound_is_converted_rather_than_classified() {
        let spec = parse(quote! { Source { age => { min: 13, max: 120.5 } } }).unwrap();
        let tokens = spec.expand().to_string();
        expect_that!(tokens, contains_substring("min : Some ((13) . into ())"));
        expect_that!(tokens, contains_substring("max : Some ((120.5) . into ())"));
    }

    /// An expression, not just a literal — the reason these are held as `Expr`.
    #[gtest]
    fn a_bound_may_be_any_expression() {
        let spec = parse(quote! { Source { age => { min: MIN_AGE, max: 2 * LIMIT } } }).unwrap();
        let tokens = spec.expand().to_string();
        expect_that!(tokens, contains_substring("MIN_AGE"));
        expect_that!(tokens, contains_substring("2 * LIMIT"));
    }

    /// A form that declares no constraint expands exactly as it did before
    /// they existed.
    #[gtest]
    fn a_body_without_constraints_emits_no_call() {
        let spec = parse(quote! { Source { notes => { label: "Notes" } } }).unwrap();
        expect_that!(
            spec.expand().to_string(),
            not(contains_substring("with_constraints"))
        );
    }

    /// `pattern` is a `LitStr`, not an `Expr`, because the macro has to read
    /// the string to check it compiles as a regex. A const would parse as an
    /// expression and defeat that, so it is rejected here instead.
    #[gtest]
    fn a_pattern_must_be_a_literal() {
        expect_that!(
            err_of(quote! { Source { zip => { pattern: ZIP_RE } } }),
            contains_substring("expected string literal")
        );
    }
}
