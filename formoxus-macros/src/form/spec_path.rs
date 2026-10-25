//! `field.sub[].maybe?.$Enum::Variant.leaf` — the dotted path a `form!` entry
//! is keyed by, and the witness that proves one exists on the model.

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Ident, Index, LitFloat, LitInt, Member, Result, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

/// A dotted path, where any field segment may be followed by `[]` to mean
/// "each element of this list" rather than the list itself, or `?` to mean
/// "what this `Option` holds", and a `$Enum::Variant` segment enters one
/// variant of the enum field before it.
///
/// `venues` addresses the `ListSet` (its legend); `venues[]` addresses every
/// row; `venues[].city` addresses the `city` field of every row. That third one
/// is the reason `[]` exists — a row's fields are otherwise unnameable, since a
/// row's real path contains a generated key (`venues.#0.city`) that no author
/// could write and that would pin one row anyway.
///
/// `shape.$Shape::Circle.radius` addresses `radius` while `Circle` is chosen.
/// The enum is named because the checks below are a `match`, and a pattern has
/// to name its enum; `form!` sees tokens, not types, so it cannot find the enum
/// from the field. The key keeps only `$Circle`, the runtime's own spelling.
///
/// `address?.city` and `nickname?` say the field is an `Option`. The `?` is
/// REQUIRED on one, at the end of a path too, so a path spells every container
/// in the field's type: `[]` a list, `?` an `Option`. The key drops it, since
/// the runtime walk adds no segment for an `Option`. See [`option_checks`] for
/// how a missing one is caught.
///
/// A path as parsed may begin or end with a variant, since a nested block's
/// prefix (`shape.$Shape::Circle => { … }`) and its entries
/// (`$Shape::Circle.radius => { … }` inside `shape => { … }`) are each half of
/// one. [`check_names_a_node`](Self::check_names_a_node) is the test for a
/// whole one.
#[derive(Clone, Debug)]
pub(crate) struct SpecPath {
    pub(crate) segments: Vec<Segment>,
}

#[derive(Clone, Debug)]
pub(crate) enum Segment {
    /// A field by name, or a tuple field by index (`0`), and the `?` and `[]`
    /// written after it, in order: `maybe?[]` is an `Option<Vec<T>>` and
    /// `rows[]?` a `Vec<Option<T>>`.
    Field {
        member: Member,
        suffixes: Vec<Suffix>,
    },
    /// `$Enum::Variant`. Always followed by a field: a variant has no node of
    /// its own, so a path cannot stop at one, and two in a row would skip the
    /// field that holds the second enum.
    Variant { path: syn::Path },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Suffix {
    /// `?`: what the `Option` holds.
    Some,
    /// `[]`: each row of the list.
    Each,
}

impl Segment {
    pub(crate) fn span(&self) -> Span {
        match self {
            Segment::Field {
                member: Member::Named(ident),
                ..
            } => ident.span(),
            Segment::Field {
                member: Member::Unnamed(index),
                ..
            } => index.span,
            // The variant's name, which is what an error about it is about.
            Segment::Variant { path } => variant_ident(path).span(),
        }
    }

    fn is_variant(&self) -> bool {
        matches!(self, Segment::Variant { .. })
    }
}

/// The variant a `$Enum::Variant` path names. Never empty: the parse refuses a
/// path without the enum in front.
fn variant_ident(path: &syn::Path) -> &Ident {
    &path
        .segments
        .last()
        .expect("a variant path has at least two segments")
        .ident
}

impl SpecPath {
    /// The map key: dotted, with `[]` kept as part of the segment it followed,
    /// and a variant as `$Variant`.
    ///
    /// `[]` survives into the key deliberately: the shape walk looks a row up by
    /// folding its `.#n` segment into `[]` (`build::spec_key`), so `venues[]`
    /// names every row. Stripping it here would lose the distinction between the
    /// list and its rows. The enum in front of a variant is dropped, since the
    /// runtime path never had it (`shape.$Circle.radius`).
    pub(crate) fn key(&self) -> String {
        self.segments
            .iter()
            .map(|s| match s {
                // `unraw`: an author writing `r#type` means the field `type`,
                // which is the name facet reports and the name the shape
                // walk looks a node up by. Leaving the
                // `r#` on would make the key match nothing.
                Segment::Field { member, suffixes } => {
                    let mut name = match member {
                        Member::Named(ident) => ident.unraw().to_string(),
                        Member::Unnamed(index) => index.index.to_string(),
                    };
                    for suffix in suffixes {
                        if *suffix == Suffix::Each {
                            name.push_str("[]");
                        }
                    }
                    name
                }
                Segment::Variant { path } => format!("${}", variant_ident(path).unraw()),
            })
            .collect::<Vec<_>>()
            .join(".")
    }

    pub(crate) fn span(&self) -> Span {
        // Never empty: the parse loop reads a segment before it can break.
        self.segments[0].span()
    }

    pub(crate) fn last_span(&self) -> Span {
        self.segments[self.segments.len() - 1].span()
    }

    pub(crate) fn starts_with_variant(&self) -> bool {
        self.segments[0].is_variant()
    }

    pub(crate) fn ends_with_variant(&self) -> bool {
        self.segments[self.segments.len() - 1].is_variant()
    }

    /// Refuse a path that names no node by itself: one that starts at a
    /// variant, where the model has no enum to enter, or stops at one.
    pub(crate) fn check_names_a_node(&self) -> Result<()> {
        self.check_starts_at_a_field()?;
        if self.ends_with_variant() {
            return Err(syn::Error::new(
                self.last_span(),
                format!(
                    "`{}` is a variant, which has no node of its own — name a field inside \
                     it, or put the setting on the enum field before it",
                    self.key()
                ),
            ));
        }
        Ok(())
    }

    /// Refuse a path that starts at a variant. A path starts at the model, and
    /// the variant to enter is a variant of one of its fields.
    pub(crate) fn check_starts_at_a_field(&self) -> Result<()> {
        if self.starts_with_variant() {
            return Err(syn::Error::new(
                self.span(),
                "a path starts at a field of the model — name the enum field first, as in \
                 `shape.$Shape::Circle.radius`",
            ));
        }
        Ok(())
    }

    /// `prefix` then `self`: an entry of a nested block, under the block's path.
    pub(crate) fn under(&self, prefix: &SpecPath) -> Result<SpecPath> {
        if prefix.ends_with_variant() && self.starts_with_variant() {
            return Err(two_variants(self.span()));
        }
        let mut segments = prefix.segments.clone();
        segments.extend(self.segments.iter().cloned());
        Ok(SpecPath { segments })
    }
}

fn two_variants(span: Span) -> syn::Error {
    syn::Error::new(
        span,
        "two variants in a row — a variant's field must come between, the one that holds the \
         second enum",
    )
}

/// One segment's name, normalized so both spellings of a keyword-named field
/// reach the same place.
///
/// `parse_any` is what accepts `type` at all: a plain `Ident` parse rejects
/// every Rust keyword, and `r#type` is a perfectly legal field name. Having
/// accepted it, the ident must be made RAW, because [`probe`] emits
/// `#base.#ident` and `__s.type` is a syntax error where `__s.r#type` is not —
/// accepting the bare spelling without this would trade a clear "expected
/// identifier" for a baffling one pointing into generated code.
///
/// A non-keyword is returned untouched, so nothing else in the expansion moves.
fn segment_ident(input: ParseStream<'_>) -> Result<Ident> {
    let ident = Ident::parse_any(input)?;
    let written = ident.to_string();
    if written.starts_with("r#") || syn::parse_str::<Ident>(&written).is_ok() {
        Ok(ident)
    } else {
        Ok(Ident::new_raw(&written, ident.span()))
    }
}

/// The `?` and `[]` after a field segment, in the order written.
fn suffixes(input: ParseStream<'_>) -> Result<Vec<Suffix>> {
    let mut suffixes = Vec::new();
    loop {
        if input.peek(Token![?]) {
            input.parse::<Token![?]>()?;
            suffixes.push(Suffix::Some);
        } else if input.peek(syn::token::Bracket) {
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
            suffixes.push(Suffix::Each);
        } else {
            return Ok(suffixes);
        }
    }
}

/// A tuple field's index. Unsuffixed, since `0u8` names no field.
fn index(digits: &str, span: Span) -> Result<Member> {
    match digits.parse::<u32>() {
        Ok(index) => Ok(Member::Unnamed(Index { index, span })),
        Err(_) => Err(syn::Error::new(
            span,
            "a tuple field is named by its plain index, as in `.0`",
        )),
    }
}

impl Parse for SpecPath {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut segments: Vec<Segment> = Vec::new();
        loop {
            if input.peek(Token![$]) {
                input.parse::<Token![$]>()?;
                let path = syn::Path::parse_mod_style(input)?;
                if path.segments.len() < 2 {
                    let variant = variant_ident(&path);
                    return Err(syn::Error::new_spanned(
                        &path,
                        format!(
                            "name the enum as well as the variant, as in `$MyEnum::{variant}` — \
                             `form!` sees the field's name, not its type, and the check that \
                             the variant exists has to name it"
                        ),
                    ));
                }
                if segments.last().is_some_and(Segment::is_variant) {
                    return Err(two_variants(variant_ident(&path).span()));
                }
                if input.peek(syn::token::Bracket) {
                    return Err(input.error(
                        "`[]` goes on a list field, not on a variant — a variant is not a list",
                    ));
                }
                if input.peek(Token![?]) {
                    return Err(input.error(
                        "`?` goes on an `Option` field, not on a variant — a variant is not an \
                         `Option`",
                    ));
                }
                segments.push(Segment::Variant { path });
            } else if input.peek(LitFloat) {
                // `pair.0.1` lexes its `0.1` as ONE float literal, the same
                // snag rustc's own field access works around: split it back
                // into the two indices it was written as.
                let lit: LitFloat = input.parse()?;
                let written = lit.to_string();
                let Some((first, second)) = written.split_once('.') else {
                    return Err(syn::Error::new(lit.span(), "expected a field or an index"));
                };
                segments.push(Segment::Field {
                    member: index(first, lit.span())?,
                    suffixes: Vec::new(),
                });
                segments.push(Segment::Field {
                    member: index(second, lit.span())?,
                    suffixes: suffixes(input)?,
                });
            } else if input.peek(LitInt) {
                let lit: LitInt = input.parse()?;
                let member = index(&lit.to_string(), lit.span())?;
                segments.push(Segment::Field {
                    member,
                    suffixes: suffixes(input)?,
                });
            } else {
                let ident = segment_ident(input)?;
                segments.push(Segment::Field {
                    member: Member::Named(ident),
                    suffixes: suffixes(input)?,
                });
            }
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
/// inferred, so nothing has to spell it out. A variant becomes a `match` that
/// binds the field after it, for the same reason: the field's type is
/// inferred from the pattern. `?` binds what the `Option` holds through
/// `field_kind::some`, which takes only an `Option`.
///
/// The `match` has a catch-all arm for the other variants, allowed to be
/// unreachable because for a one-variant enum it is.
#[expect(
    clippy::needless_pass_by_value,
    reason = "the recursion builds a fresh `quote!{…}` for `base` at every level, so a reference would need a `let` binding at each of the three call sites to borrow from"
)]
pub(crate) fn probe(segments: &[Segment], base: TokenStream2, depth: usize) -> TokenStream2 {
    match segments {
        [] => quote! { let _ = &#base; },
        [
            Segment::Variant { path },
            field @ Segment::Field { member, suffixes },
            rest @ ..,
        ] => {
            let bound = format_ident!("__v{depth}");
            let inner = probe_suffixes(
                quote!(#bound),
                true,
                suffixes,
                field.span(),
                rest,
                depth + 1,
            );
            quote! {
                match &#base {
                    #path { #member: #bound, .. } => { #inner }
                    #[allow(unreachable_patterns)]
                    _ => {}
                }
            }
        }
        [Segment::Variant { .. }, ..] => {
            unreachable!("a variant is followed by a field: the parse refuses anything else")
        }
        [field @ Segment::Field { member, suffixes }, rest @ ..] => probe_suffixes(
            quote! { #base.#member },
            false,
            suffixes,
            field.span(),
            rest,
            depth,
        ),
    }
}

/// The rest of `probe`, from a field that has been reached as `next`: through
/// each of its `?` and `[]` in turn, then on to the next segment. `next` is a
/// place, or with `is_ref` a binding that already refers to one.
fn probe_suffixes(
    next: TokenStream2,
    is_ref: bool,
    suffixes: &[Suffix],
    span: Span,
    rest: &[Segment],
    depth: usize,
) -> TokenStream2 {
    match suffixes.split_first() {
        None => probe(rest, next, depth),
        Some((Suffix::Each, more)) => {
            let row = format_ident!("__row{depth}");
            let inner = probe_suffixes(quote!(#row), true, more, span, rest, depth + 1);
            // `quote_spanned!`, not `quote!`: `.iter()` is the token that
            // fails when `[]` is put on something that is not a list, and
            // with a call-site span rustc underlines the whole `form!`
            // invocation and suggests nonsense. Borrowing the segment's own
            // span puts the caret on the author's `field[]`.
            let iter = quote_spanned! { span=> #next.iter() };
            quote! { for #row in #iter { #inner } }
        }
        Some((Suffix::Some, more)) => {
            let held = format_ident!("__o{depth}");
            let inner = probe_suffixes(quote!(#held), true, more, span, rest, depth + 1);
            let borrow = if is_ref {
                quote!(#next)
            } else {
                quote!(&#next)
            };
            let some = some_call(span);
            quote! { { let __p = #borrow; let #held = #some; #inner } }
        }
    }
}

/// The field a spec path names, as a place expression on `__m`.
///
/// For the projection closures `form!`'s constraint checks are built from
/// (`|__m: &Model| &<this>`); see `formoxus::field_kind` for why those are a
/// closure and not a witness like [`probe`]. `[]` goes through
/// `field_kind::row`, which stands for one element, so the element's own
/// fields can follow it. A variant is a `match` whose other arm is
/// `field_kind::other_variant`, which stands for the field in a variant that
/// isn't chosen. A `?` goes through `field_kind::some`, except the path's very
/// last one: the field keeps its `Option`, which is what the widget and
/// `required_true` checks ask about. Like `probe`, it is only ever
/// type-checked.
///
/// `[]` calls `.iter()` and `?` calls `some` with the segment's own span,
/// exactly as `probe` does, so a mistake fails here with the SAME error at the
/// same place, and rustc shows it once. Passing `&list` to `row` instead would
/// add a second, different error pointing at the whole `form!`.
pub(crate) fn project(segments: &[Segment]) -> TokenStream2 {
    walk(segments, false, |_, _| {})
}

/// Free `const _` assertions that every `Option` on the path was written with
/// its `?`.
///
/// The witness catches the rest: a `?` on what is not an `Option` is a type
/// error in `some`, and a missing `?` before `.field` or a variant is rustc's
/// own "no field on `Option`" or mismatched types. That leaves two places a
/// missing `?` type-checks anyway: the end of the path, where the field is
/// only borrowed, and before `[]`, where `Option` has an `.iter()` of its own
/// and would quietly make the "row" the whole list. One assertion for each,
/// asking the shape whether the place is still an `Option` once every written
/// `?` has been taken off.
pub(crate) fn option_checks(segments: &[Segment], model: &impl quote::ToTokens) -> TokenStream2 {
    let message = "this field is an `Option`, so a path writes `?` after it, as in `address?` \
                   or `address?.city`";
    let check = |place: &TokenStream2, span: Span| {
        quote_spanned! { span=>
            const _: () = ::core::assert!(
                !::formoxus::field_kind::is_optional(
                    ::formoxus::field_kind::shape_of(|__m: &#model| &#place)
                ),
                #message
            );
        }
    };
    let mut checks = Vec::new();
    let end = walk(segments, true, |place, span| {
        checks.push(check(place, span));
    });
    let last = segments.last().map_or_else(Span::call_site, Segment::span);
    checks.push(check(&end, last));
    quote! { #(#checks)* }
}

/// `some(__p)`, with `__p` the borrowed field, spanned on the field.
///
/// The argument is a binding and not `&field` itself so that ALL of it carries
/// the field's span: `&__s.name` mixes in call-site tokens, and rustc then
/// underlines the whole `form!` when `?` is on something that is not an
/// `Option`. `probe` and `walk` both borrow exactly one level, so the two
/// report the same error, which rustc shows once.
fn some_call(span: Span) -> TokenStream2 {
    let arg = quote_spanned! { span=> __p };
    quote_spanned! { span=> ::formoxus::field_kind::some(#arg) }
}

/// The place [`project`] and [`option_checks`] build: every segment applied to
/// `__m` in turn. `peel_last` takes off the path's final `?` too, which the
/// projection keeps; `before_rows` sees the place each `[]` is applied to.
fn walk(
    segments: &[Segment],
    peel_last: bool,
    mut before_rows: impl FnMut(&TokenStream2, Span),
) -> TokenStream2 {
    let mut place = quote!(__m);
    let mut segments = segments.iter().peekable();
    while let Some(seg) = segments.next() {
        let field = match seg {
            Segment::Field { member, .. } => {
                place = quote! { #place.#member };
                seg
            }
            Segment::Variant { path } => {
                let field = segments
                    .next()
                    .expect("a variant is followed by a field: the parse refuses anything else");
                let Segment::Field { member, .. } = field else {
                    unreachable!("two variants in a row: the parse refuses them")
                };
                place = quote! {
                    (*match &#place {
                        #path { #member: __v, .. } => __v,
                        #[allow(unreachable_patterns)]
                        _ => ::formoxus::field_kind::other_variant(),
                    })
                };
                field
            }
        };
        let Segment::Field { suffixes, .. } = field else {
            unreachable!("`field` is a field segment by construction")
        };
        let span = field.span();
        let last_segment = segments.peek().is_none();
        for (i, suffix) in suffixes.iter().enumerate() {
            match suffix {
                Suffix::Each => {
                    before_rows(&place, span);
                    let iter = quote_spanned! { span=> #place.iter() };
                    place = quote! { (*::formoxus::field_kind::row(#iter)) };
                }
                Suffix::Some if last_segment && i == suffixes.len() - 1 && !peel_last => {}
                Suffix::Some => {
                    let some = some_call(span);
                    place = quote! { (*{ let __p = &#place; #some }) };
                }
            }
        }
    }
    place
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
        // the real node path, which no author can write and which would pin one
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
        expect_that!(out, contains_substring("with_widget"));
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

    // ── Keyword-named fields ───────────────────────────────────────────

    /// `r#type` is a legal field name, so it has to be a legal path segment.
    /// The `r#` is a spelling, not part of the name: facet reports the field as
    /// `type`, and that is the key the shape walk looks a node up by.
    #[gtest]
    fn a_raw_identifier_keys_on_the_bare_name() {
        let spec = parse(quote! { Model { r#type => { label: "Kind" } } }).expect("parses");
        expect_that!(
            fields(&spec)
                .into_iter()
                .map(|(p, _, _)| p)
                .collect::<Vec<_>>(),
            elements_are![eq("type")]
        );
    }

    /// The bare spelling reaches the same field. A plain `Ident` parse rejects
    /// every Rust keyword, which is why this needs `parse_any`.
    #[gtest]
    fn the_bare_spelling_of_a_keyword_reaches_the_same_field() {
        let spec = parse(quote! { Model { type => { label: "Kind" } } }).expect("parses");
        expect_that!(
            fields(&spec)
                .into_iter()
                .map(|(p, _, _)| p)
                .collect::<Vec<_>>(),
            elements_are![eq("type")]
        );
    }

    #[gtest]
    fn a_keyword_segment_works_below_a_row_selector() {
        let spec = parse(quote! { Model { rows[].r#match => { label: "Arm" } } }).expect("parses");
        expect_that!(
            fields(&spec)
                .into_iter()
                .map(|(p, _, _)| p)
                .collect::<Vec<_>>(),
            elements_are![eq("rows[].match")]
        );
    }

    /// **The reason the parse normalizes to a RAW ident rather than keeping
    /// what was written.** The witness is field access, and `__s.type` is a
    /// syntax error — accepting the bare spelling without this would swap a
    /// clear parse error for rustc complaining about generated code.
    #[gtest]
    fn the_witness_emits_a_keyword_segment_raw_either_way() {
        expect_that!(
            witness_of(quote! { Model { r#type => { label: "Kind" } } }),
            contains_substring("__s . r#type")
        );
        expect_that!(
            witness_of(quote! { Model { type => { label: "Kind" } } }),
            contains_substring("__s . r#type")
        );
    }

    // ── The projection ───────────────────────────────────────────────────

    fn projection_of(src: TokenStream2) -> String {
        let spec = parse(src).expect("should parse");
        let path = spec
            .entries
            .iter()
            .find_map(|e| match e {
                Entry::Field { path, .. } => Some(path),
                _ => None,
            })
            .expect("a field entry");
        project(&path.segments).to_string()
    }

    /// A place, not a reference: `row` yields `&T`, so it is dereferenced to
    /// let the next field follow, and the closure's own `&` then makes the
    /// field's type `T` rather than `&T`.
    #[gtest]
    fn a_row_segment_projects_through_row() {
        expect_that!(
            projection_of(quote! { Trip { venues[].city => { label: "City" } } }),
            eq("(* :: formoxus :: field_kind :: row (__m . venues . iter ())) . city")
        );
    }

    #[gtest]
    fn a_nested_path_projects_as_plain_field_access() {
        expect_that!(
            projection_of(quote! { Trip { venue.city => { label: "City" } } }),
            eq("__m . venue . city")
        );
    }

    // ── `$Enum::Variant` ─────────────────────────────────────────────────

    /// The enum is written so the checks can name it; the key drops it,
    /// because the runtime path never had it.
    #[gtest]
    fn a_variant_segment_keys_on_the_variant_alone() {
        let spec = parse(quote! {
            Quiz { data.$QuestionData::TrueFalse.text => { label: "Prompt" } }
        })
        .unwrap();
        expect_that!(fields(&spec)[0].0, eq("data.$TrueFalse.text"));
    }

    #[gtest]
    fn a_qualified_enum_keys_the_same() {
        let spec = parse(quote! {
            Quiz { data.$crate::models::QuestionData::TrueFalse.text => { label: "Prompt" } }
        })
        .unwrap();
        expect_that!(fields(&spec)[0].0, eq("data.$TrueFalse.text"));
    }

    #[gtest]
    fn a_variant_composes_with_rows_on_either_side() {
        let spec = parse(quote! {
            Drawing { shapes[].$Shape::Poly.points[].x => { label: "X" } }
        })
        .unwrap();
        expect_that!(fields(&spec)[0].0, eq("shapes[].$Poly.points[].x"));
    }

    #[gtest]
    fn a_tuple_field_is_named_by_index() {
        let spec = parse(quote! { Meter { reading.$Reading::Pair.1 => { min: 3 } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("reading.$Pair.1"));
    }

    /// `.0.1` lexes as the float `0.1`, so the parse splits it back.
    #[gtest]
    fn two_indices_in_a_row_are_split_back_apart() {
        let spec = parse(quote! { M { a.$E::V.0.1 => { label: "x" } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("a.$V.0.1"));
    }

    /// What Todd first wrote, and what anyone will: the check is a `match`,
    /// which has to name the enum, and `form!` can't find it from the field.
    #[gtest]
    fn a_variant_without_its_enum_is_rejected() {
        let msg = err_of(quote! { Quiz { data.$TrueFalse.text => { label: "Prompt" } } });
        expect_that!(msg, contains_substring("`$MyEnum::TrueFalse`"));
    }

    #[gtest]
    fn two_variants_in_a_row_are_rejected() {
        let msg = err_of(quote! { M { a.$E::V.$F::W.x => { label: "x" } } });
        expect_that!(msg, contains_substring("two variants in a row"));
    }

    #[gtest]
    fn brackets_on_a_variant_are_rejected() {
        let msg = err_of(quote! { M { a.$E::V[].x => { label: "x" } } });
        expect_that!(msg, contains_substring("not on a variant"));
    }

    #[gtest]
    fn a_path_from_a_variant_is_rejected() {
        let msg = err_of(quote! { M { $E::V.x => { label: "x" } } });
        expect_that!(msg, contains_substring("starts at a field of the model"));
    }

    #[gtest]
    fn a_path_ending_at_a_variant_with_keys_is_rejected() {
        let msg = err_of(quote! { M { a.$E::V => { label: "x" } } });
        expect_that!(msg, contains_substring("no node of its own"));
    }

    /// A `match` that binds the field after the variant, so its type is
    /// inferred, with a catch-all that may be unreachable (one variant).
    #[gtest]
    fn a_variant_witness_is_a_match_binding_the_next_field() {
        expect_that!(
            witness_of(quote! { Quiz { data.$Q::TrueFalse.text => { label: "Prompt" } } }),
            eq(
                "match & __s . data { Q :: TrueFalse { text : __v0 , .. } => { let _ = & __v0 ; } \
                # [allow (unreachable_patterns)] _ => { } }"
            )
        );
    }

    #[gtest]
    fn a_row_inside_a_variant_loops_over_the_bound_field() {
        expect_that!(
            witness_of(quote! { D { s.$S::Poly.points[] => { label: "P" } } }),
            eq("match & __s . s { S :: Poly { points : __v0 , .. } => \
                { for __row1 in __v0 . iter () { let _ = & __row1 ; } } \
                # [allow (unreachable_patterns)] _ => { } }")
        );
    }

    /// A tuple field binds by index in a braced pattern, `{ 1: __v0, .. }`,
    /// so a tuple variant needs no count of the fields before it.
    #[gtest]
    fn a_tuple_field_binds_by_index() {
        expect_that!(
            witness_of(quote! { M { r.$R::Pair.1 => { min: 3 } } }),
            starts_with("match & __s . r { R :: Pair { 1 : __v0 , .. }")
        );
    }

    #[gtest]
    fn a_variant_projects_through_a_match() {
        expect_that!(
            projection_of(quote! { Quiz { data.$Q::TrueFalse.text => { label: "Prompt" } } }),
            eq(
                "(* match & __m . data { Q :: TrueFalse { text : __v , .. } => __v , \
                # [allow (unreachable_patterns)] _ => :: formoxus :: field_kind :: other_variant () , })"
            )
        );
    }

    // ── A nested block ───────────────────────────────────────────────────

    #[gtest]
    fn a_nested_block_flattens_to_one_entry_per_field() {
        let spec = parse(quote! {
            Quiz {
                data => {
                    label: "Question Type",
                    $Q::TrueFalse => {
                        text => { widget: textarea },
                        answer => { widget: select },
                    },
                },
            }
        })
        .unwrap();
        expect_that!(
            fields(&spec)
                .into_iter()
                .map(|(p, w, _)| format!("{p} {w}"))
                .collect::<Vec<_>>(),
            elements_are![
                eq("data "),
                eq("data.$TrueFalse.text textarea"),
                eq("data.$TrueFalse.answer select")
            ]
        );
    }

    /// The prefix's own entry exists only if it has keys of its own.
    #[gtest]
    fn a_prefix_with_only_nested_entries_contributes_none_itself() {
        let spec = parse(quote! {
            Event { venue => { city => { label: "Town" }, street => { label: "Road" } } }
        })
        .unwrap();
        expect_that!(
            fields(&spec)
                .into_iter()
                .map(|(p, _, _)| p)
                .collect::<Vec<_>>(),
            elements_are![eq("venue.city"), eq("venue.street")]
        );
    }

    /// A field named like a key nests by its `=>`; only `:` makes a key.
    #[gtest]
    fn a_nested_field_may_be_named_like_a_key() {
        let spec = parse(quote! { M { a => { widget => { label: "W" } } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("a.widget"));
    }

    #[gtest]
    fn a_path_written_flat_and_nested_is_a_repeat() {
        let msg = err_of(quote! {
            Quiz {
                data.$Q::TrueFalse.text => { label: "Prompt" },
                data => { $Q::TrueFalse => { text => { widget: textarea } } },
            }
        });
        expect_that!(
            msg,
            contains_substring("`data.$TrueFalse.text` is specified twice")
        );
    }

    #[gtest]
    fn a_nested_variant_under_a_variant_is_rejected() {
        let msg = err_of(quote! {
            M { a.$E::V => { $F::W.x => { label: "x" } } }
        });
        expect_that!(msg, contains_substring("two variants in a row"));
    }

    // ── `?` — what an `Option` holds ─────────────────────────────────────

    /// The runtime walk adds no segment for an `Option`, so neither does the
    /// key; `[]` stays, since it does change which node is named.
    #[gtest]
    fn a_question_mark_is_dropped_from_the_key() {
        let spec = parse(quote! {
            M {
                addr?.city => { label: "Town" },
                nickname? => { widget: textarea },
                maybe?[] => { widget: email },
                rows[]?.x => { label: "X" },
            }
        })
        .unwrap();
        expect_that!(
            fields(&spec)
                .into_iter()
                .map(|(p, _, _)| p)
                .collect::<Vec<_>>(),
            elements_are![
                eq("addr.city"),
                eq("nickname"),
                eq("maybe[]"),
                eq("rows[].x")
            ]
        );
    }

    #[gtest]
    fn a_question_mark_on_a_variant_is_rejected() {
        let msg = err_of(quote! { M { a.$E::V?.x => { label: "x" } } });
        expect_that!(msg, contains_substring("not on a variant"));
    }

    /// `some` takes only an `Option`, so this is also the check that the `?` is
    /// on one.
    #[gtest]
    fn a_question_mark_witness_binds_what_the_option_holds() {
        expect_that!(
            witness_of(quote! { M { addr?.city => { label: "Town" } } }),
            eq(
                "{ let __p = & __s . addr ; let __o0 = :: formoxus :: field_kind :: some (__p) ; \
                let _ = & __o0 . city ; }"
            )
        );
    }

    #[gtest]
    fn suffixes_apply_in_the_order_written() {
        expect_that!(
            witness_of(quote! { M { maybe?[] => { widget: email } } }),
            eq(
                "{ let __p = & __s . maybe ; let __o0 = :: formoxus :: field_kind :: some (__p) ; \
                for __row1 in __o0 . iter () { let _ = & __row1 ; } }"
            )
        );
    }

    #[gtest]
    fn an_option_before_a_variant_is_entered_first() {
        expect_that!(
            witness_of(quote! { M { data?.$E::V.x => { label: "x" } } }),
            starts_with(
                "{ let __p = & __s . data ; let __o0 = :: formoxus :: field_kind :: some (__p) ; \
                 match & __o0 { E :: V { x : __v1 , .. }"
            )
        );
    }

    /// The last `?` is not taken off: the widget and `required_true` checks
    /// ask whether the field is an `Option`.
    #[gtest]
    fn the_projection_keeps_the_fields_own_option() {
        expect_that!(
            projection_of(quote! { M { nickname? => { widget: textarea } } }),
            eq("__m . nickname")
        );
        expect_that!(
            projection_of(quote! { M { addr?.zip? => { label: "ZIP" } } }),
            eq("(* { let __p = & __m . addr ; :: formoxus :: field_kind :: some (__p) }) . zip")
        );
    }

    fn option_checks_of(src: TokenStream2) -> String {
        let spec = parse(src).expect("should parse");
        let path = spec
            .entries
            .iter()
            .find_map(|e| match e {
                Entry::Field { path, .. } => Some(path),
                _ => None,
            })
            .expect("a field entry");
        option_checks(&path.segments, &quote!(M)).to_string()
    }

    /// One check at the end, on the place with every `?` taken off, and one
    /// before each `[]`: the two places a missing `?` type-checks anyway.
    #[gtest]
    fn option_checks_cover_the_end_and_each_row_selector() {
        let out = option_checks_of(quote! { M { maybe?[] => { widget: email } } });
        expect_that!(out.matches("const _").count(), eq(2));
        expect_that!(
            out,
            contains_substring(
                "shape_of (| __m : & M | & (* { let __p = & __m . maybe ; :: formoxus :: \
                 field_kind :: some (__p) }))"
            )
        );
        expect_that!(
            out,
            contains_substring(
                "shape_of (| __m : & M | & (* :: formoxus :: field_kind :: row ((* { let __p = & \
                 __m . maybe ; :: formoxus :: field_kind :: some (__p) }) . iter ())))"
            )
        );
    }

    #[gtest]
    fn the_end_check_takes_the_last_question_mark_off() {
        expect_that!(
            option_checks_of(quote! { M { nickname? => { widget: textarea } } }),
            contains_substring(
                "shape_of (| __m : & M | & (* { let __p = & __m . nickname ; :: formoxus :: \
                 field_kind :: some (__p) }))"
            )
        );
    }

    /// Everything in the braces, the label too, is about the `Option` field,
    /// so a `?` prefix may carry keys of its own (unlike a variant).
    #[gtest]
    fn a_question_mark_prefix_carries_its_own_keys_and_nested_ones() {
        let spec = parse(quote! {
            M { addr? => { label: "Address", city => { label: "Town" } } }
        })
        .unwrap();
        expect_that!(
            fields(&spec)
                .into_iter()
                .map(|(p, _, l)| format!("{p} {l}"))
                .collect::<Vec<_>>(),
            elements_are![eq("addr \"Address\""), eq("addr.city \"Town\"")]
        );
    }

    #[gtest]
    fn a_nested_entry_may_end_in_a_question_mark() {
        let spec = parse(quote! { M { addr? => { zip? => { label: "ZIP" } } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("addr.zip"));
    }

    #[gtest]
    fn an_empty_nested_block_is_still_empty() {
        let msg = err_of(quote! { M { a => { b => {} } } });
        expect_that!(msg, contains_substring("empty field body"));
    }
}
