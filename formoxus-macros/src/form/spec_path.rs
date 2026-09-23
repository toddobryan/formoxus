//! `field.sub[].leaf` — the dotted path a `form!` entry is keyed by, and
//! the witness that proves one exists on the model.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Ident, Result, Token,
    parse::{Parse, ParseStream},
};

/// A dotted path, where any segment may be followed by `[]` to mean "each
/// element of this list" rather than the list itself.
///
/// `venues` addresses the `ListSet` (its legend); `venues[]` addresses every
/// row; `venues[].city` addresses the `city` field of every row. That third one
/// is the reason `[]` exists — a row's fields are otherwise unnameable, since a
/// row's real path contains a generated key (`venues.#0.city`) that no author
/// could write and that would pin one row anyway.
#[derive(Debug)]
pub(crate) struct SpecPath {
    pub(crate) segments: Vec<Segment>,
}

#[derive(Debug)]
pub(crate) struct Segment {
    pub(crate) ident: Ident,
    /// Was this segment written `name[]`?
    pub(crate) each: bool,
}

impl SpecPath {
    /// The map key: dotted, with `[]` kept as part of the segment it followed.
    ///
    /// `[]` survives into the key deliberately — `ListSet::apply_specs` is what
    /// resolves it, by substituting each row's actual segment. Stripping it here
    /// would lose the distinction between the list and its rows.
    pub(crate) fn key(&self) -> String {
        self.segments
            .iter()
            .map(|s| {
                if s.each {
                    format!("{}[]", s.ident)
                } else {
                    s.ident.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(".")
    }

    pub(crate) fn span(&self) -> proc_macro2::Span {
        // Never empty: the parse loop reads an ident before it can break.
        self.segments[0].ident.span()
    }
}

impl Parse for SpecPath {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut segments = Vec::new();
        loop {
            let ident: Ident = input.parse()?;
            let each = if input.peek(syn::token::Bracket) {
                let brackets;
                let span = syn::bracketed!(brackets in input);
                if !brackets.is_empty() {
                    // `venues[0]` looks plausible and isn't supported: a spec
                    // describes every row, not one of them. Say so rather than
                    // letting the index be silently dropped.
                    return Err(syn::Error::new(
                        span.span.join(),
                        "`[]` must be empty — a spec applies to every row, not one index",
                    ));
                }
                true
            } else {
                false
            };
            segments.push(Segment { ident, each });
            if input.peek(Token![.]) {
                input.parse::<Token![.]>()?;
            } else {
                break;
            }
        }
        Ok(SpecPath { segments })
    }
}

/// One statement per spec path, proving the path exists on the model.
///
/// Never executed — it exists so rustc checks the access. `[]` becomes a loop,
/// which is what lets a row's field be named at all: the element type is
/// inferred, so nothing has to spell it out.
pub(crate) fn probe(segments: &[Segment], base: TokenStream2, depth: usize) -> TokenStream2 {
    match segments.split_first() {
        None => quote! { let _ = &#base; },
        Some((seg, rest)) => {
            let id = &seg.ident;
            let next = quote! { #base.#id };
            if seg.each {
                let row = format_ident!("__row{depth}");
                let inner = probe(rest, quote!(#row), depth + 1);
                // `quote_spanned!`, not `quote!`: `.iter()` is the token that
                // fails when `[]` is put on something that is not a list, and
                // with a call-site span rustc underlines the whole `form!`
                // invocation and suggests nonsense. Borrowing the segment's own
                // span puts the caret on the author's `field[]`.
                let iter = quote_spanned! { seg.ident.span()=> #next.iter() };
                quote! { for #row in #iter { #inner } }
            } else {
                probe(rest, next, depth)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::Entry;
    use crate::form::tests::{err_of, fields, parse};
    use googletest::prelude::*;
    use quote::quote;

    // ── `[]` — each row of a list ────────────────────────────────────────

    #[gtest]
    fn a_bare_list_path_addresses_the_list_itself() {
        // No brackets: this is the `ListSet`, so it gets the legend. A widget
        // here is rejected at apply time, since a list has no single widget.
        let spec = parse(quote! { Quiz { answers => { label: "Answers" } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("answers"));
    }

    #[gtest]
    fn empty_brackets_address_every_row() {
        let spec = parse(quote! { Quiz { answers[] => { widget: textarea } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("answers[]"));
        expect_that!(fields(&spec)[0].1, eq("textarea"));
    }

    #[gtest]
    fn brackets_compose_with_a_field_below_them() {
        // The case that `[]` exists for: a row's own field. `venues.#0.city` is
        // the real member path, which no author can write and which would pin one
        // row if they could.
        let spec = parse(quote! { Trip { venues[].city => { label: "City" } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("venues[].city"));
    }

    #[gtest]
    fn brackets_may_appear_more_than_once() {
        // `Vec<Vec<T>>` is already a supported shape, so the path syntax should
        // not be the thing that can't express it.
        let spec = parse(quote! { Grid { rows[].cells[] => { widget: textarea } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("rows[].cells[]"));
    }

    #[gtest]
    fn an_indexed_bracket_is_rejected() {
        // `venues[0]` reads as if it would work; refusing it is better than
        // silently dropping the index and applying to every row.
        let msg = err_of(quote! { Quiz { answers[0] => { label: "First" } } });
        expect_that!(msg, contains_substring("every row"));
    }

    // ── The witness ──────────────────────────────────────────────────────
    //
    // `probe` emits a never-executed statement per spec path so that rustc
    // checks the access. These pin the SHAPE of what it emits; the compile-fail
    // goldens in `crates/formoxus/tests/ui/` pin that the check actually fires.

    /// The witness statement for the first field entry's path.
    fn witness_of(src: TokenStream2) -> String {
        let spec = parse(src).expect("should parse");
        let path = spec
            .entries
            .iter()
            .find_map(|e| match e {
                Entry::Field { path, .. } => Some(path),
                _ => None,
            })
            .expect("a field entry");
        probe(&path.segments, quote!(__s), 0).to_string()
    }

    #[gtest]
    fn a_plain_field_borrows_it() {
        expect_that!(
            witness_of(quote! { LoginForm { password => { widget: password } } }),
            eq("let _ = & __s . password ;")
        );
    }

    #[gtest]
    fn a_nested_path_walks_the_field_chain() {
        expect_that!(
            witness_of(quote! { Event { venue.city => { label: "City" } } }),
            eq("let _ = & __s . venue . city ;")
        );
    }

    #[gtest]
    fn a_row_selector_becomes_a_loop() {
        // The loop is what makes a row's type INFERRED — the macro never has to
        // name the element type, which it could not do anyway.
        expect_that!(
            witness_of(quote! { Quiz { answers[] => { widget: textarea } } }),
            eq("for __row0 in __s . answers . iter () { let _ = & __row0 ; }")
        );
    }

    #[gtest]
    fn a_row_field_is_checked_through_the_loop_binding() {
        expect_that!(
            witness_of(quote! { Trip { venues[].city => { label: "City" } } }),
            eq("for __row0 in __s . venues . iter () { let _ = & __row0 . city ; }")
        );
    }

    #[gtest]
    fn nested_row_selectors_nest_loops_with_distinct_bindings() {
        // Depth-indexed bindings rather than shadowing: `__row0`/`__row1` keep a
        // rustc error pointing at the loop that actually failed.
        expect_that!(
            witness_of(quote! { Grid { rows[].cells[] => { widget: textarea } } }),
            eq("for __row0 in __s . rows . iter () \
                 { for __row1 in __row0 . cells . iter () { let _ = & __row1 ; } }")
        );
    }

    #[gtest]
    fn a_field_below_a_nested_row_keeps_walking() {
        expect_that!(
            witness_of(quote! { Grid { rows[].cells[].text => { label: "Text" } } }),
            eq("for __row0 in __s . rows . iter () \
                 { for __row1 in __row0 . cells . iter () { let _ = & __row1 . text ; } }")
        );
    }

    #[gtest]
    fn the_expansion_carries_one_witness_per_field() {
        // Two fields, two statements, inside a single never-called fn.
        let spec = parse(quote! {
            ChangePasswordForm {
                title: "Change Password",
                current_password => { widget: password },
                new_password => { widget: password, label: "New password" },
            }
        })
        .unwrap();
        let out = spec.expand().to_string();
        expect_that!(
            out,
            contains_substring("fn __paths_exist (__s : & ChangePasswordForm)")
        );
        expect_that!(
            out,
            contains_substring("let _ = & __s . current_password ;")
        );
        expect_that!(out, contains_substring("let _ = & __s . new_password ;"));
        // And the builder chain is still there beside it.
        expect_that!(out, contains_substring("with_title"));
        expect_that!(out, contains_substring("with_custom_widget"));
    }

    #[gtest]
    fn a_spec_with_no_fields_has_an_empty_witness_body() {
        let out = parse(quote! { Article { title: "A" } })
            .unwrap()
            .expand()
            .to_string();
        expect_that!(
            out,
            contains_substring("fn __paths_exist (__s : & Article) { }")
        );
    }
}
