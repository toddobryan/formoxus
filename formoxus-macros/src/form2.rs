use proc_macro2::TokenStream as TokenStream2;
use syn::{Expr, Ident, Path, Result, Token, braced, parenthesized, parse::{Parse, ParseStream}, punctuated::Punctuated};

pub fn impl_form2(input: TokenStream2) -> TokenStream2 {
    match syn::parse2::<FormSpecInput>(input) {
        Ok(spec) => spec.expand(),
        Err(err) => err.to_compile_error(),
    }
}

mod kw {
    syn::custom_keyword!(title);
    syn::custom_keyword!(validator);
}

#[derive(Debug)]
struct FormSpecInput {
    target: Path,
    entries: Vec<Entry>,
}

impl FormSpecInput {
    fn expand(&self) -> TokenStream2 {
        todo!()
    }
}

impl Parse for FormSpecInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let target: Path = input.parse()?;
        let body;
        braced!(body in input);
        let entries: Vec<Entry> = 
            Punctuated::<Entry, Token![,]>::parse_terminated(&body)?.into_iter().collect();
        Ok(FormSpecInput { target, entries })
    }
}

#[derive(Debug)]
enum Entry {
    Title(Expr),
    Validator(Expr),
    Field { path: SpecPath, body: FieldBody },
}

/// A dotted path, where any segment may be followed by `[]` to mean "each
/// element of this list" rather than the list itself.
///
/// `venues` addresses the `ListSet` (its legend); `venues[]` addresses every
/// row; `venues[].city` addresses the `city` field of every row. That third one
/// is the reason `[]` exists — a row's fields are otherwise unnameable, since a
/// row's real path contains a generated key (`venues.#0.city`) that no author
/// could write and that would pin one row anyway.
#[derive(Debug)]
struct SpecPath {
    segments: Vec<Segment>,
}

#[derive(Debug)]
struct Segment {
    ident: Ident,
    /// Was this segment written `name[]`?
    each: bool,
}

impl SpecPath {
    /// The map key: dotted, with `[]` kept as part of the segment it followed.
    ///
    /// `[]` survives into the key deliberately — `ListSet::apply_specs` is what
    /// resolves it, by substituting each row's actual segment. Stripping it here
    /// would lose the distinction between the list and its rows.
    fn key(&self) -> String {
        self.segments
            .iter()
            .map(|s| if s.each { format!("{}[]", s.ident) } else { s.ident.to_string() })
            .collect::<Vec<_>>()
            .join(".")
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

impl Entry {
    fn parse_title(input: ParseStream<'_>) -> Result<Self> {
        let _title: kw::title = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let expr: Expr = input.parse()?;
        Ok(Entry::Title(expr))
    }

    fn parse_validator(input: ParseStream<'_>) -> Result<Self> {
        let _validator: kw::validator = input.parse()?;
        let _color: Token![:] = input.parse()?;
        let expr: Expr = input.parse()?;
        Ok(Entry::Validator(expr))
    }

    fn parse_field(input: ParseStream<'_>) -> Result<Self> {
        let path: SpecPath = input.parse()?;
        if !input.peek(syn::token::FatArrow) {
            return Err(input.error("Expected => after a field name"));
        } else {
            input.parse::<syn::token::FatArrow>()?;
        }
        let body: FieldBody = input.parse()?;
        Ok(Entry::Field { path, body })
    }
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if input.peek(kw::title) {
            Entry::parse_title(input)
        } else if input.peek(kw::validator) {
            Entry::parse_validator(input)
        } else if input.peek(syn::Ident) {
            Entry::parse_field(input)
        } else {
            Err(input.error("expected a form attribute or a field specifier"))
        }
    }
}

#[derive(Debug)]
struct FieldBody {
    control: Option<ControlRef>,
    label: Option<Expr>,
}

impl Parse for FieldBody {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if !input.peek(syn::token::Brace) {
            return Err(input.error("expected a field specification surrounded by braces"))
        } else {
            let body;
            let braces = braced!(body in input);
            let mut fb = FieldBody { control: None, label: None };
            while !body.is_empty() {
                let key: Ident = body.parse()?;
                let _colon: Token![:] = body.parse()?;
                match key.to_string().as_str() {
                    "control" if fb.control.is_some() =>
                        return Err(syn::Error::new_spanned(&key, "duplicate control key")),
                    "control" => fb.control = Some(body.parse()?),
                    "label" if fb.label.is_some() =>
                        return Err(syn::Error::new_spanned(&key, "duplicate label")),
                    "label" => fb.label = Some(body.parse()?),
                    other => return Err(syn::Error::new_spanned(&key, format!("unknown key {other}, expected control or label"))),
                }
                if body.peek(Token![,]) {
                    body.parse::<Token![,]>()?;
                }
            }

            if fb.control.is_none() && fb.label.is_none() {
                return Err(syn::Error::new(braces.span.join(), "empty field body"));
            }
            Ok(fb)
        }
    }
}

#[derive(Debug)]
enum ControlRef {
    Wrapped {
        outer: Ident,
        inner: Ident,
    },
    Bare(Ident),
}

impl Parse for ControlRef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let outer: Ident = input.parse()?;
        if input.peek(syn::token::Paren) {
            let inner;
            parenthesized!(inner in input);
            Ok(Self::Wrapped { outer, inner: inner.parse()? })
        } else {
            Ok(Self::Bare(outer))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;
    use quote::quote;

    fn parse(tokens: TokenStream2) -> syn::Result<FormSpecInput> {
        syn::parse2::<FormSpecInput>(tokens)
    }

    /// Which kind each entry is, in source order — enough to pin the grammar
    /// without requiring `Debug` on everything below `Entry`.
    fn kinds(spec: &FormSpecInput) -> Vec<&'static str> {
        spec.entries
            .iter()
            .map(|e| match e {
                Entry::Title(_) => "title",
                Entry::Validator(_) => "validator",
                Entry::Field { .. } => "field",
            })
            .collect()
    }

    /// Field entries as `("dotted.path", "control", "label")`, with `""` for an
    /// absent control or label. The control is rendered the way `expand` will
    /// have to, so this also pins that `Input(Password)` keeps both idents.
    fn fields(spec: &FormSpecInput) -> Vec<(String, String, String)> {
        spec.entries
            .iter()
            .filter_map(|e| match e {
                Entry::Field { path, body } => Some((
                    path.key(),
                    match &body.control {
                        None => String::new(),
                        Some(ControlRef::Bare(v)) => v.to_string(),
                        Some(ControlRef::Wrapped { outer, inner }) => {
                            format!("{outer}({inner})")
                        }
                    },
                    match &body.label {
                        None => String::new(),
                        Some(expr) => quote!(#expr).to_string(),
                    },
                )),
                _ => None,
            })
            .collect()
    }

    // ── The two real forms ───────────────────────────────────────────────

    #[gtest]
    fn the_login_form_parses() {
        let spec = parse(quote! {
            LoginForm {
                title: "Sign In",
                password => { control: Input(Password) },
            }
        })
        .expect("LoginForm should parse");

        expect_that!(kinds(&spec), elements_are![eq(&"title"), eq(&"field")]);
        expect_that!(
            fields(&spec),
            elements_are![eq(&(
                "password".to_string(),
                "Input(Password)".to_string(),
                String::new()
            ))]
        );
    }

    #[gtest]
    fn the_change_password_form_parses() {
        // The whole surface at once: a title, a validator, three fields, two of
        // them carrying both keys. If entries were being dropped this would be
        // the test that noticed.
        let spec = parse(quote! {
            ChangePasswordForm {
                title: "Change Password",
                validator: check_new_and_confirm_match,
                current_password => { control: Input(Password) },
                new_password => { control: Input(Password), label: "New password" },
                confirm_new_password => { control: Input(Password), label: "Confirm new password" },
            }
        })
        .expect("ChangePasswordForm should parse");

        expect_that!(
            kinds(&spec),
            elements_are![
                eq(&"title"),
                eq(&"validator"),
                eq(&"field"),
                eq(&"field"),
                eq(&"field")
            ]
        );
        expect_that!(fields(&spec).len(), eq(3));
    }

    // ── Shapes ───────────────────────────────────────────────────────────

    #[gtest]
    fn a_bare_control_needs_no_parens() {
        let spec = parse(quote! { Source { notes => { control: Textarea } } }).unwrap();
        expect_that!(
            fields(&spec),
            elements_are![eq(&("notes".to_string(), "Textarea".to_string(), String::new()))]
        );
    }

    #[gtest]
    fn a_label_only_entry_is_legal() {
        // Nothing about a label requires a control — renaming a field is the
        // commonest customization there is.
        let spec = parse(quote! { Source { url => { label: "Homepage" } } }).unwrap();
        expect_that!(
            fields(&spec),
            elements_are![eq(&(
                "url".to_string(),
                String::new(),
                "\"Homepage\"".to_string()
            ))]
        );
    }

    #[gtest]
    fn a_nested_path_keeps_every_segment() {
        // `expand` needs both halves out of this: the dotted string for the map
        // key and the same idents for `&s.venue.city` in the witness.
        let spec = parse(quote! { Event { venue.city => { label: "City" } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("venue.city"));
    }

    #[gtest]
    fn the_target_may_be_a_qualified_path() {
        // A foreign model won't be in scope unqualified, which is the whole
        // second use case — so `Path`, not `Ident`.
        let spec = parse(quote! { models::Source { url => { label: "x" } } }).unwrap();
        let target = &spec.target;
        expect_that!(quote!(#target).to_string(), eq("models :: Source"));
    }

    #[gtest]
    fn trailing_commas_are_allowed_everywhere() {
        let spec = parse(quote! {
            Source {
                title: "Sources",
                notes => { control: Textarea, label: "Notes", },
            }
        })
        .expect("trailing commas in both positions should parse");
        expect_that!(kinds(&spec), elements_are![eq(&"title"), eq(&"field")]);
    }

    #[gtest]
    fn a_spec_may_be_empty() {
        // `form2! { T {} }` is the "formization with no customization" case —
        // every struct gets one, so the empty body has to be legal.
        let spec = parse(quote! { Source {} }).expect("an empty spec should parse");
        expect_that!(kinds(&spec), elements_are![]);
    }

    // ── `[]` — each row of a list ────────────────────────────────────────

    #[gtest]
    fn a_bare_list_path_addresses_the_list_itself() {
        // No brackets: this is the `ListSet`, so it gets the legend. A control
        // here is rejected at apply time, since a list has no single control.
        let spec = parse(quote! { Quiz { answers => { label: "Answers" } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("answers"));
    }

    #[gtest]
    fn empty_brackets_address_every_row() {
        let spec = parse(quote! { Quiz { answers[] => { control: Textarea } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("answers[]"));
        expect_that!(fields(&spec)[0].1, eq("Textarea"));
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
        let spec = parse(quote! { Grid { rows[].cells[] => { control: Textarea } } }).unwrap();
        expect_that!(fields(&spec)[0].0, eq("rows[].cells[]"));
    }

    #[gtest]
    fn an_indexed_bracket_is_rejected() {
        // `venues[0]` reads as if it would work; refusing it is better than
        // silently dropping the index and applying to every row.
        let msg = err_of(quote! { Quiz { answers[0] => { label: "First" } } });
        expect_that!(msg, contains_substring("every row"));
    }

    // ── Errors ───────────────────────────────────────────────────────────

    fn err_of(tokens: TokenStream2) -> String {
        parse(tokens).expect_err("should not parse").to_string()
    }

    #[gtest]
    fn an_empty_field_body_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes => {} } }),
            contains_substring("empty field body")
        );
    }

    #[gtest]
    fn an_unknown_key_names_itself() {
        let msg = err_of(quote! { Source { notes => { contrl: Textarea } } });
        expect_that!(msg, contains_substring("contrl"));
        expect_that!(msg, contains_substring("control"));
    }

    #[gtest]
    fn a_duplicate_key_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes => { label: "a", label: "b" } } }),
            contains_substring("duplicate")
        );
    }

    #[gtest]
    fn a_missing_fat_arrow_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes { control: Textarea } } }),
            contains_substring("=>")
        );
    }

    #[gtest]
    fn a_field_body_must_be_braced() {
        expect_that!(
            err_of(quote! { Source { notes => Textarea } }),
            contains_substring("braces")
        );
    }

    #[gtest]
    fn a_nonsense_entry_is_rejected() {
        expect_that!(err_of(quote! { Source { 42 } }).len(), gt(0));
    }
}
