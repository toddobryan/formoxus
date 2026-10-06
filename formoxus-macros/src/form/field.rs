//! `<path> => { … }` — one field's body.

use indexmap::IndexMap;

use formoxus_attrs::{Attr, AttrType, FieldType, Owner};
use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, quote, quote_spanned};
use regress::Regex;
use syn::{
    Expr, Ident, LitStr, Result, Token, braced, bracketed,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
};

use super::{SpecPath, WidgetRef, suggest::edit_distance};

#[derive(Debug)]
pub(crate) struct FieldSpec {
    pub(crate) path: SpecPath,
    pub(crate) body: FieldBody,
}

#[derive(Debug, Default)]
pub(crate) struct FieldBody {
    pub(crate) widget: Option<WidgetRef>,
    pub(crate) label: Option<Expr>,
    pub(crate) attrs: ParsedAttrs,
}

#[derive(Debug, Default)]
pub(crate) struct ParsedAttrs(IndexMap<AttrId, ParsedAttr>);

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum AttrId {
    Std(Attr),
    NonStd(String),
}

#[derive(Debug)]
pub(crate) struct ParsedAttr {
    key_span: Span,
    source: AttrSource,
}

impl ParsedAttrs {
    /// The expression written for a table attribute, if the body has it and
    /// its value is one. What the bound and length cross-checks read.
    fn expr(&self, attr: Attr) -> Option<&Expr> {
        match &self.0.get(&AttrId::Std(attr))?.source {
            AttrSource::Expr(e) => Some(e),
            _ => None,
        }
    }
}

/// The field types an attribute applies to, as an error message names them:
/// `MinLength` → "a String field", `Min` → "a number field".
///
/// Generated from the row's `for:` rather than written per key, so a new row
/// gets a message for free. `Int | Float` is "number", the word authors use;
/// either alone is named for what it is.
fn field_types_phrase(attr: Attr) -> String {
    let int = attr.applies_to(FieldType::Int);
    let float = attr.applies_to(FieldType::Float);
    let words: Vec<&str> = [
        (attr.applies_to(FieldType::Text), "String"),
        (int && float, "number"),
        (int && !float, "integer"),
        (float && !int, "float"),
        (attr.applies_to(FieldType::Bool), "bool"),
    ]
    .into_iter()
    .filter_map(|(applies, word)| applies.then_some(word))
    .collect();
    let joined = match words.as_slice() {
        [] => unreachable!("`{attr:?}` applies to no field type"),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    };
    format!("a {joined} field")
}

impl ParsedAttr {
    /// The `(AttrKey, AttrValue)` tuple this attribute contributes to the
    /// field's map, spanned onto the author's key.
    ///
    /// A table attribute's value variant comes from its row's `attr_type()`;
    /// a quoted key's is always `AttrValue::String`. `Parse` only builds the
    /// source a row's type asks for, so any other pairing is a bug in this
    /// file, not something an author can write.
    /// Where a compile-time check about this attribute points: the value, so
    /// the caret lands on `500` in `max_length: 500`, or the key for a bare
    /// flag, which has no value.
    fn value_span(&self) -> Span {
        match &self.source {
            AttrSource::Expr(e) => e.span(),
            AttrSource::Regex(s) => s.span(),
            AttrSource::Flag => self.key_span,
            AttrSource::List(items) => items.first().map_or(self.key_span, LitStr::span),
        }
    }

    fn entry_tokens(&self, attr_id: &AttrId) -> TokenStream2 {
        let attrs = quote! { ::formoxus::attrs };
        let (key, attr_type) = match attr_id {
            AttrId::Std(attr) => {
                let variant = Ident::new(attr.variant_name(), self.key_span);
                (
                    quote! { #attrs::AttrKey::Std(#attrs::Attr::#variant) },
                    attr.attr_type(),
                )
            }
            AttrId::NonStd(name) => (quote! { #attrs::AttrKey::NonStd(#name) }, AttrType::String),
        };
        // `(#e).into()` — parenthesized because `#e` may be any expression.
        // For a bound, the literal's own type is what picks `Bound::Int` over
        // `Bound::Float`; for a string, it accepts `"off"` and `format!(…)`
        // alike.
        let value = match (attr_type, &self.source) {
            (AttrType::String, AttrSource::Expr(e)) => {
                quote! { #attrs::AttrValue::String((#e).into()) }
            }
            (AttrType::Int, AttrSource::Expr(e)) => quote! { #attrs::AttrValue::Int(#e) },
            (AttrType::Bound, AttrSource::Expr(e)) => {
                quote! { #attrs::AttrValue::Bound((#e).into()) }
            }
            (AttrType::Regex, AttrSource::Regex(s)) => quote! { #attrs::AttrValue::Regex(#s) },
            (AttrType::Flag, AttrSource::Flag) => quote! { #attrs::AttrValue::Flag },
            (AttrType::TokenList | AttrType::Declarations, AttrSource::List(_)) => {
                todo!("3d: `AttrValue` has no list variants yet")
            }
            (attr_type, source) => {
                unreachable!("`{attr_id:?}` is {attr_type:?} but was parsed as {source:?}")
            }
        };
        quote_spanned! { self.key_span=> (#key, #value) }
    }
}

#[derive(Debug)]
pub(crate) enum AttrSource {
    Expr(Box<Expr>),
    Regex(LitStr),
    Flag,
    List(Vec<LitStr>),
}

impl FieldBody {
    /// The `FieldAttrs` this body declares, or `None` when it declares
    /// none — in which case no `.with_attrs` call is emitted at all and a
    /// form without attributes expands exactly as it did before they existed.
    ///
    /// One call taking the whole map, not one setter per key: each key the
    /// body has becomes one `(AttrKey, AttrValue)` entry, in the order
    /// written, and a key it lacks contributes nothing, so the expansion holds
    /// exactly what was written.
    pub(crate) fn attrs_tokens(&self) -> Option<TokenStream2> {
        let entries: Vec<TokenStream2> = self
            .attrs
            .0
            .iter()
            .map(|(attr_id, parsed_attr)| parsed_attr.entry_tokens(attr_id))
            .collect();

        if entries.is_empty() {
            return None;
        }
        Some(quote! { ::formoxus::fields::FieldAttrs::from([ #(#entries), * ]) })
    }
}

/// Lints the bound checks allow, whatever type the author wrote the bound in.
///
/// They matter because each check carries the author's span, so a lint on it
/// lands in THEIR crate: `max: 100` as f64 is an `unnecessary_cast` of a
/// literal, an `f64` bound `as f64` a trivial cast, a `u64` `as i128` a
/// lossless one, a float `as i128` a truncation.
fn cast_lints() -> TokenStream2 {
    quote! {
        trivial_numeric_casts,
        clippy::unnecessary_cast,
        clippy::cast_precision_loss,
        clippy::cast_lossless,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    }
}

impl FieldBody {
    /// Free `const _` assertions that the field at `place` can take what this
    /// body declares: one per table attribute (two for `required_true`), the
    /// bound checks, `min <= max`
    /// and `min_length <= max_length`, and whether the widget can render it
    /// (see `WidgetRef::checks`).
    ///
    /// Free const items rather than anything in `__paths_exist`, because only
    /// a free const item is evaluated by `cargo check` or when nothing calls
    /// it. `formoxus::field_kind` has the details. Each assertion is spanned
    /// onto the author's value, so the caret lands on `500` in
    /// `max_length: 500` rather than on the whole `form!`.
    ///
    /// A const panic takes a fixed `&'static str`, so the messages cannot
    /// name the field or the values; the span has to do that.
    pub(crate) fn type_checks(&self, model: &impl ToTokens, place: &TokenStream2) -> TokenStream2 {
        let shape = quote! { ::formoxus::field_kind::shape_of(|__m: &#model| &#place) };
        let takes = |span: Span, test: TokenStream2, message: &str| {
            quote_spanned! { span=>
                const _: () = ::core::assert!(#test, #message);
            }
        };
        let cast_lints = cast_lints();

        let mut checks = Vec::new();
        // One check per table attribute, asking the table whether its row's
        // `for:` covers the field. A quoted key is not in the table, so there
        // is nothing to ask.
        for (attr_id, parsed_attr) in &self.attrs.0 {
            let AttrId::Std(attr) = attr_id else { continue };
            let variant = Ident::new(attr.variant_name(), Span::call_site());
            let test = quote! {
                ::formoxus::field_kind::applies(::formoxus::attrs::Attr::#variant, #shape)
            };
            let span = parsed_attr.value_span();
            if *attr == Attr::RequiredTrue {
                // Two asserts, because a const panic takes one fixed message
                // and the two mistakes want different ones.
                // `required_is_not_optional` is `true` for a non-bool, so each
                // mistake reports once.
                let msg = "`required_true` applies only to a bool field, where it means the \
                    value must be true; every other field is already required unless its type \
                    is an `Option`";
                checks.push(takes(span, test, msg));
                let msg = "`required_true` cannot apply to an `Option<bool>`: an optional field \
                    may be left unanswered, so it cannot also be required to be true";
                checks.push(takes(
                    span,
                    quote!(::formoxus::field_kind::required_is_not_optional(#shape)),
                    msg,
                ));
            } else {
                let msg = format!(
                    "`{}` applies only to {}",
                    attr.variant_name().to_snake_case(),
                    field_types_phrase(*attr),
                );
                checks.push(takes(span, test, &msg));
            }
        }
        // Does each bound fit the field's type? The bound goes in cast both
        // ways, since a const fn cannot be generic over "some number";
        // `field_kind` explains what each pair of casts answers.
        for (key, attr) in [("min", Attr::Min), ("max", Attr::Max)] {
            let Some(e) = self.attrs.expr(attr) else {
                continue;
            };
            for (test, problem) in [
                ("bound_in_range", "is outside the range of the field's type"),
                (
                    "bound_is_whole",
                    "must be a whole number, because the field is an integer",
                ),
                (
                    "bound_is_exact",
                    "is an integer too large to hold exactly as an f64",
                ),
            ] {
                let test = Ident::new(test, Span::call_site());
                let message = format!("`{key}` {problem}");
                checks.push(quote_spanned! { e.span()=>
                    #[allow(#cast_lints)]
                    const _: () = ::core::assert!(
                        ::formoxus::field_kind::#test(#shape, (#e) as f64, (#e) as i128),
                        #message
                    );
                });
            }
        }
        // `as f64` on both sides is what lets `min: 3, max: 120.5` compare at
        // all.
        if let (Some(min), Some(max)) = (self.attrs.expr(Attr::Min), self.attrs.expr(Attr::Max)) {
            checks.push(quote_spanned! { max.span()=>
                #[allow(#cast_lints, clippy::assertions_on_constants)]
                const _: () = ::core::assert!(
                    ((#min) as f64) <= ((#max) as f64),
                    "`min` must not exceed `max`"
                );
            });
        }
        if let (Some(min), Some(max)) = (
            self.attrs.expr(Attr::MinLength),
            self.attrs.expr(Attr::MaxLength),
        ) {
            checks.push(quote_spanned! { max.span()=>
                #[allow(clippy::assertions_on_constants)]
                const _: () = ::core::assert!(
                    (#min) <= (#max),
                    "`min_length` must not exceed `max_length`"
                );
            });
        }
        if let Some(widget) = &self.widget {
            checks.push(widget.checks(&shape));
        }
        quote! { #(#checks)* }
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
        while !body.is_empty() {
            fb.parse_entry(&body)?;
            if body.peek(Token![,]) {
                body.parse::<Token![,]>()?;
            }
        }

        // Asks "were there any keys?" directly, which cannot go stale when a
        // key is added.
        if fb.widget.is_none() && fb.label.is_none() && fb.attrs.0.is_empty() {
            return Err(syn::Error::new(braces.span.join(), "empty field body"));
        }
        Ok(fb)
    }
}

impl FieldBody {
    /// One `key: value` (or bare flag) entry: `widget`, `label`, a table
    /// attribute by its `snake_case` variant name, or a quoted key passed
    /// through as written.
    fn parse_entry(&mut self, body: ParseStream<'_>) -> Result<()> {
        if body.peek(LitStr) {
            let key: LitStr = body.parse()?;
            let id = AttrId::NonStd(quoted_key(&key)?);
            body.parse::<Token![:]>()?;
            let source = AttrSource::Expr(body.parse()?);
            return self.attrs.insert(id, key.span(), source);
        }

        // `parse_any`, so that `type:` reaches the "set by formoxus" message
        // instead of syn's "expected identifier".
        let key = Ident::parse_any(body)?;
        let name = key.to_string();
        match name.as_str() {
            "widget" | "label" => {
                body.parse::<Token![:]>()?;
                let given_twice =
                    || syn::Error::new_spanned(&key, format!("`{name}` is given twice"));
                if name == "widget" {
                    if self.widget.is_some() {
                        return Err(given_twice());
                    }
                    self.widget = Some(body.parse()?);
                } else {
                    if self.label.is_some() {
                        return Err(given_twice());
                    }
                    self.label = Some(body.parse()?);
                }
                Ok(())
            }
            _ => {
                let attr = table_key(&key)?;
                let source = parse_value(attr, &key, body)?;
                self.attrs.insert(AttrId::Std(attr), key.span(), source)
            }
        }
    }
}

/// The table attribute an identifier key names, or the error explaining why
/// it names none an author may write.
fn table_key(key: &Ident) -> Result<Attr> {
    let name = key.to_string();
    // The round trip is what makes the key exactly the `snake_case` variant
    // name: `MaxLength` comes back from `max_length`, but also from
    // `maxLength` or `Max_Length`, which are not keys.
    let attr = Attr::from_variant_name(&name.to_upper_camel_case())
        .filter(|attr| attr.variant_name().to_snake_case() == name);
    let Some(attr) = attr else {
        return Err(syn::Error::new_spanned(key, unknown_key(&name)));
    };
    if attr.owner() == Owner::Formoxus {
        return Err(syn::Error::new_spanned(key, owned(attr)));
    }
    Ok(attr)
}

/// A quoted key's attribute name, refused if the table knows it, because then
/// it has a key of its own, which is checked.
fn quoted_key(key: &LitStr) -> Result<String> {
    let name = key.value();
    match Attr::from_name(&name) {
        None => Ok(name),
        Some(attr) if attr.owner() == Owner::Formoxus => {
            Err(syn::Error::new_spanned(key, owned(attr)))
        }
        Some(attr) => Err(syn::Error::new_spanned(
            key,
            format!(
                "`\"{name}\"` is an attribute formoxus knows: write `{}:`, which is checked",
                attr.variant_name().to_snake_case()
            ),
        )),
    }
}

/// Parses the value an attribute's row asks for: an expression, a literal
/// pattern, nothing at all for a flag, or a bracketed list of strings.
///
/// **Why `Expr` for most and `LitStr` for a pattern.** A bound may be any
/// expression (`min: 13`, `min: MIN_AGE`, `min: 2 * N`), and holding it as an
/// `Expr` is also what lets the literal's own type pick the `Bound` variant
/// once it reaches `.into()`. A pattern cannot: the macro has to hand the
/// string to `regress` to check it compiles, and only a literal is readable at
/// macro time. It also lands in `AttrValue::Regex`, which holds a
/// `&'static str`.
fn parse_value(attr: Attr, key: &Ident, body: ParseStream<'_>) -> Result<AttrSource> {
    if attr.attr_type() == AttrType::Flag {
        if body.peek(Token![:]) {
            return Err(syn::Error::new_spanned(
                key,
                format!("`{key}` is a flag: write it alone, with no `:` or value"),
            ));
        }
        return Ok(AttrSource::Flag);
    }
    body.parse::<Token![:]>()?;
    Ok(match attr.attr_type() {
        AttrType::String | AttrType::Int | AttrType::Bound => AttrSource::Expr(body.parse()?),
        AttrType::Regex => AttrSource::Regex(checked_pattern(body.parse()?)?),
        AttrType::TokenList | AttrType::Declarations | AttrType::List => {
            let list;
            bracketed!(list in body);
            let items = Punctuated::<LitStr, Token![,]>::parse_terminated(&list)?;
            AttrSource::List(items.into_iter().collect())
        }
        AttrType::Flag => unreachable!("handled above"),
    })
}

/// Checked here, while the macro parses, rather than in the const witness
/// with the other constraints: compiling a regex allocates, and const
/// evaluation cannot.
///
/// Bare, then wrapped, both with `v`, as HTML's "compiled pattern regular
/// expression" does: `a)|(b` fails alone but compiles as `^(?:a)|(b)$`, and a
/// browser ignores it, so we must reject it too.
fn checked_pattern(pattern: LitStr) -> Result<LitStr> {
    let patt_str = pattern.value();
    if let Err(e) = Regex::with_flags(&patt_str, "v")
        .and_then(|_| Regex::with_flags(&format!("^(?:{patt_str})$"), "v"))
    {
        return Err(syn::Error::new_spanned(
            pattern,
            format!("`pattern` is not a valid regular expression: {e}"),
        ));
    }
    Ok(pattern)
}

/// The refusal for an attribute formoxus sets itself.
fn owned(attr: Attr) -> String {
    let hint = if attr == Attr::Required {
        " (it follows from the model's type); for a bool that must be ticked, write \
        `required_true`"
    } else {
        ""
    };
    format!(
        "`{}` is set by formoxus and cannot be given in `form!`{hint}",
        attr.name()
    )
}

/// Every identifier key a field body accepts: `widget`, `label`, then each
/// table attribute an author may set, in table order.
pub(crate) fn legal_keys() -> Vec<String> {
    ["widget".to_string(), "label".to_string()]
        .into_iter()
        .chain(
            Attr::ALL
                .iter()
                .filter(|attr| attr.owner() != Owner::Formoxus)
                .map(|attr| attr.variant_name().to_snake_case()),
        )
        .collect()
}

/// The message for an identifier key that is not one: the HTML spelling of a
/// table attribute, a near miss, or no idea, each followed by every legal key.
fn unknown_key(name: &str) -> String {
    let legal = legal_keys();
    let hint = match Attr::from_name(name).filter(|attr| attr.owner() != Owner::Formoxus) {
        Some(attr) => format!(" — write `{}`", attr.variant_name().to_snake_case()),
        None => legal
            .iter()
            .filter(|k| edit_distance(name, k) <= 2)
            .min_by_key(|k| edit_distance(name, k))
            .map(|near| format!(" — did you mean `{near}`?"))
            .unwrap_or_default(),
    };
    format!(
        "unknown key `{name}`{hint} Expected one of: {}",
        legal.join(", ")
    )
}

impl ParsedAttrs {
    /// Adds one attribute, refusing a key given twice and a replace/append
    /// pair (`class` with `class_plus`, `style` with `style_plus`).
    fn insert(&mut self, id: AttrId, key_span: Span, source: AttrSource) -> Result<()> {
        if let AttrId::Std(attr) = &id
            && let Some(other) = conflicting(*attr)
            && self.0.contains_key(&AttrId::Std(other))
        {
            let (replace, append) = if matches!(attr, Attr::Class | Attr::Style) {
                (*attr, other)
            } else {
                (other, *attr)
            };
            return Err(syn::Error::new(
                key_span,
                format!(
                    "`{}` and `{}` cannot both be given: `{0}` replaces formoxus's own, \
                    `{1}` adds to them",
                    replace.variant_name().to_snake_case(),
                    append.variant_name().to_snake_case(),
                ),
            ));
        }
        let written = match &id {
            AttrId::Std(attr) => attr.variant_name().to_snake_case(),
            AttrId::NonStd(name) => format!("\"{name}\""),
        };
        if self.0.insert(id, ParsedAttr { key_span, source }).is_some() {
            return Err(syn::Error::new(
                key_span,
                format!("`{written}` is given twice"),
            ));
        }
        Ok(())
    }
}

/// The attribute that may not appear alongside this one: each of `class` and
/// `style` either replaces formoxus's value or appends to it, never both.
fn conflicting(attr: Attr) -> Option<Attr> {
    match attr {
        Attr::Class => Some(Attr::ClassPlus),
        Attr::ClassPlus => Some(Attr::Class),
        Attr::Style => Some(Attr::StylePlus),
        Attr::StylePlus => Some(Attr::Style),
        _ => None,
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

    /// A label is any expression, not just a literal, so a shared const or a
    /// translation call works.
    #[gtest]
    fn a_label_may_be_any_expression() {
        let spec = parse(quote! { Source { notes => { label: NOTES_LABEL } } }).unwrap();
        expect_that!(
            spec.expand().to_string(),
            contains_substring(". with_label (\"notes\" , & NOTES_LABEL)")
        );
    }

    #[gtest]
    fn a_field_body_must_be_braced() {
        expect_that!(
            err_of(quote! { Source { notes => textarea } }),
            contains_substring("braces")
        );
    }

    // ── The keys are the table ───────────────────────────────────────────

    /// Not a drift guard — the keys come from the table, so they cannot
    /// drift. What this earns is the parsing by `attr_type()`: it proves each
    /// key accepts the shape of value someone will actually write. `pattern:
    /// "x"` passing is the evidence that `LitStr` was the right choice there,
    /// and a bare `required_true` that a flag takes no value.
    #[gtest]
    fn every_legal_key_parses() {
        for key in legal_keys() {
            let ident = syn::Ident::new(&key, proc_macro2::Span::call_site());
            let entry = match key.as_str() {
                "widget" => quote!(#ident: textarea),
                "label" => quote!(#ident: "A label"),
                _ => {
                    let attr = formoxus_attrs::Attr::ALL
                        .iter()
                        .find(|a| heck::ToSnakeCase::to_snake_case(a.variant_name()) == key)
                        .unwrap();
                    match attr.attr_type() {
                        formoxus_attrs::AttrType::Flag => quote!(#ident),
                        formoxus_attrs::AttrType::String | formoxus_attrs::AttrType::Regex => {
                            quote!(#ident: "x")
                        }
                        formoxus_attrs::AttrType::Int | formoxus_attrs::AttrType::Bound => {
                            quote!(#ident: 1)
                        }
                        formoxus_attrs::AttrType::TokenList
                        | formoxus_attrs::AttrType::Declarations
                        | formoxus_attrs::AttrType::List => quote!(#ident: ["x"]),
                    }
                }
            };
            let parsed = parse(quote! { Source { notes => { #entry } } });
            expect_that!(
                parsed.is_ok(),
                eq(true),
                "`{key}` should be an accepted key: {:?}",
                parsed.err()
            );
        }
    }

    #[gtest]
    fn an_unknown_key_lists_every_legal_one() {
        let msg = err_of(quote! { Source { notes => { maxlen: 3 } } });
        for key in legal_keys() {
            expect_that!(
                msg,
                contains_substring(key.as_str()),
                "the message should name `{key}`"
            );
        }
    }

    // ── Constraints reach the expansion ──────────────────────────────────

    #[gtest]
    fn attribute_keys_become_one_with_attrs_call() {
        let spec = parse(quote! {
            Source { notes => { min_length: 3, max_length: 500, pattern: "\\w+" } }
        })
        .unwrap();
        let tokens = spec.expand().to_string();

        // ONE call, holding a struct literal — not one call per key.
        expect_that!(tokens.matches("with_attrs").count(), eq(1));
        expect_that!(
            tokens,
            contains_substring(":: formoxus :: fields :: FieldAttrs :: from ([")
        );
        expect_that!(
            tokens,
            contains_substring(
                "(:: formoxus :: attrs :: AttrKey :: Std (:: formoxus :: attrs :: Attr :: MinLength) , :: formoxus :: attrs :: AttrValue :: Int (3))"
            )
        );
        expect_that!(
            tokens,
            contains_substring(
                "(:: formoxus :: attrs :: AttrKey :: Std (:: formoxus :: attrs :: Attr :: MaxLength) , :: formoxus :: attrs :: AttrValue :: Int (500))"
            )
        );
        expect_that!(tokens, contains_substring("AttrValue :: Regex"));
    }

    /// A bound goes through `.into()`, which is what lets the literal's own
    /// type decide between `Bound::Int` and `Bound::Float` — the macro never
    /// inspects the token to choose.
    #[gtest]
    fn a_bound_is_converted_rather_than_classified() {
        let spec = parse(quote! { Source { age => { min: 13, max: 120.5 } } }).unwrap();
        let tokens = spec.expand().to_string();
        expect_that!(
            tokens,
            contains_substring(
                "Attr :: Min) , :: formoxus :: attrs :: AttrValue :: Bound ((13) . into ())"
            )
        );
        expect_that!(
            tokens,
            contains_substring(
                "Attr :: Max) , :: formoxus :: attrs :: AttrValue :: Bound ((120.5) . into ())"
            )
        );
    }

    /// An expression, not just a literal — the reason these are held as `Expr`.
    #[gtest]
    fn a_bound_may_be_any_expression() {
        let spec = parse(quote! { Source { age => { min: MIN_AGE, max: 2 * LIMIT } } }).unwrap();
        let tokens = spec.expand().to_string();
        expect_that!(tokens, contains_substring("MIN_AGE"));
        expect_that!(tokens, contains_substring("2 * LIMIT"));
    }

    /// `required_true` lands in the same `FieldAttrs` as the other keys, as
    /// `Attr::RequiredTrue`, so it reaches `check` by the same route.
    #[gtest]
    fn required_true_becomes_a_constraint() {
        let spec = parse(quote! { Source { agreed => { required_true } } }).unwrap();
        let tokens = spec.expand().to_string();
        expect_that!(tokens.matches("with_attrs").count(), eq(1));
        expect_that!(
            tokens,
            contains_substring(
                "(:: formoxus :: attrs :: AttrKey :: Std (:: formoxus :: attrs :: Attr :: RequiredTrue) , :: formoxus :: attrs :: AttrValue :: Flag)"
            )
        );
    }

    /// A flag is written bare. Its presence is the whole value, so there is
    /// no `false` to write and nothing for `true` to add.
    #[gtest]
    fn a_flag_takes_no_value() {
        expect_that!(
            err_of(quote! { Source { agreed => { required_true: true } } }),
            contains_substring("`required_true` is a flag: write it alone")
        );
    }

    /// Presence comes from the model's type alone, so `required` is
    /// formoxus's. The old spelling of "must be ticked" was `required: true`,
    /// so the refusal points at the new one.
    #[gtest]
    fn presence_required_is_owned_and_points_at_required_true() {
        let msg = err_of(quote! { Source { agreed => { required: true } } });
        expect_that!(msg, contains_substring("`required` is set by formoxus"));
        expect_that!(msg, contains_substring("write `required_true`"));
    }

    /// A form that declares no constraint expands exactly as it did before
    /// they existed.
    #[gtest]
    fn a_body_without_constraints_emits_no_call() {
        let spec = parse(quote! { Source { notes => { label: "Notes" } } }).unwrap();
        expect_that!(
            spec.expand().to_string(),
            not(contains_substring("with_attrs"))
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

    // ── A pattern must compile ───────────────────────────────────────────

    #[gtest]
    fn a_pattern_that_does_not_compile_is_rejected() {
        let msg = err_of(quote! { Source { zip => { pattern: "(\\d{5}" } } });
        expect_that!(
            msg,
            contains_substring("`pattern` is not a valid regular expression")
        );
        // `regress`'s own reason is passed through.
        expect_that!(msg, contains_substring("Unbalanced parenthesis"));
    }

    #[gtest]
    fn a_pattern_that_compiles_is_accepted() {
        expect_that!(
            parse(quote! { Source { zip => { pattern: "\\d{5}(-\\d{4})?" } } }).is_ok(),
            eq(true)
        );
    }

    /// `a)|(b` compiles once wrapped as `^(?:a)|(b)$`, and so would pass
    /// `FormField::check`, but not alone. HTML compiles the bare pattern first
    /// and drops the constraint if that fails, so accepting it would leave the
    /// browser checking nothing while the server checks something.
    #[gtest]
    fn a_pattern_must_compile_before_it_is_wrapped() {
        expect_that!(
            err_of(quote! { Source { code => { pattern: "a)|(b" } } }),
            contains_substring("`pattern` is not a valid regular expression")
        );
    }

    /// Legal with no flags, illegal under `v`, which is the flag HTML compiles
    /// a `pattern` with — an unescaped `-` is a class-set syntax character.
    #[gtest]
    fn a_pattern_is_checked_with_the_v_flag() {
        expect_that!(
            err_of(quote! { Source { code => { pattern: "[a-z-]" } } }),
            contains_substring("Invalid class set character")
        );
    }

    /// Checked even when `pattern` is not the last key, so the check sits after
    /// the key loop rather than inside the `pattern` arm.
    #[gtest]
    fn a_bad_pattern_is_caught_among_other_keys() {
        expect_that!(
            err_of(quote! { Source { zip => { pattern: "[", max_length: 10 } } }),
            contains_substring("`pattern` is not a valid regular expression")
        );
    }
}
