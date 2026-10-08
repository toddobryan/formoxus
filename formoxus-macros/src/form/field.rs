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
            (AttrType::TokenList | AttrType::Declarations, AttrSource::List(items)) => {
                quote! { #attrs::AttrValue::List(&[#(#items),*]) }
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
        AttrType::TokenList => {
            let list;
            bracketed!(list in body);
            let items = Punctuated::<CssClassName, Token![,]>::parse_terminated(&list)?;
            if attr == Attr::ClassPlus && items.is_empty() {
                return Err(list.error("`class_plus` with an empty list should be omitted"));
            }
            AttrSource::List(items.into_iter().map(|ccn| ccn.0).collect())
        }
        AttrType::Declarations => {
            let list;
            braced!(list in body);
            let declarations = style_declarations(&list)?;
            if attr == Attr::StylePlus && declarations.is_empty() {
                return Err(list.error("`style_plus` with an empty block should be omitted"));
            }
            AttrSource::List(declarations)
        }
        AttrType::Flag => unreachable!("handled above"),
    })
}

struct CssClassName(LitStr);

impl Parse for CssClassName {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        css_name(
            input,
            "class name",
            " Separate classes need a comma between them.",
        )
        .map(CssClassName)
    }
}

/// A class name or a style property: quoted and verbatim, or bare with `_`
/// turned into `-`. `what` names it in the errors, and `hyphen_tail` ends the
/// hyphen error, where a class list has a second reading to cover.
fn css_name(input: ParseStream<'_>, what: &str, hyphen_tail: &str) -> Result<LitStr> {
    if input.peek(LitStr) {
        input.parse()
    } else if input.peek(Ident::peek_any) {
        let token = input.call(Ident::parse_any)?;
        let as_str = bare_css(&token);
        if input.peek(Token![-]) {
            return Err(hyphenated(&token, &as_str, input, what, hyphen_tail));
        }
        Ok(LitStr::new(&as_str, token.span()))
    } else if input.peek(Token![-]) {
        let (name, tokens) = hyphen_run(String::new(), TokenStream2::new(), input);
        Err(syn::Error::new_spanned(
            tokens,
            format!("a {what} starting with `-` must be quoted: `\"{name}\"`"),
        ))
    } else {
        Err(input.error(format!("expected {what}, either bare or quoted")))
    }
}

/// The one rule for a bare name: `_` becomes `-`.
fn bare_css(ident: &Ident) -> String {
    ident.unraw().to_string().replace('_', "-")
}

/// The error for a bare name with a `-` in it, as in `text-center`.
///
/// Whitespace is not a token, so `[text -mt-4]` (two classes, comma missing)
/// arrives exactly like `[text-mt-4]`. Rather than guess, a class list's
/// message reads right either way: the fix for one name, then a reminder
/// about commas (its `tail`). The underscore spelling is offered only when it
/// is an identifier (`text-1.5` has no bare form).
fn hyphenated(
    first: &Ident,
    first_name: &str,
    input: ParseStream<'_>,
    what: &str,
    tail: &str,
) -> syn::Error {
    let (name, tokens) = hyphen_run(first_name.to_string(), first.to_token_stream(), input);
    let bare = name.replace('-', "_");
    let fix = if syn::parse_str::<Ident>(&bare).is_ok() {
        format!("write `{bare}`, or quote it: `\"{name}\"`")
    } else {
        format!("quote it: `\"{name}\"`")
    };
    syn::Error::new_spanned(
        tokens,
        format!("`-` cannot appear in a bare {what}: {fix}.{tail}"),
    )
}

/// Reads `-part-part…` onto `name`, where a part is an identifier or a
/// number (`mt`, `4`, `2xl`), stopping at anything else. A `-` straight after
/// a `-` is read too, so `--gap` comes out whole. Returns the hyphenated name
/// and the tokens read, so the error can span all of them.
fn hyphen_run(
    mut name: String,
    mut tokens: TokenStream2,
    input: ParseStream<'_>,
) -> (String, TokenStream2) {
    while let Ok(dash) = input.parse::<Token![-]>() {
        name.push('-');
        dash.to_tokens(&mut tokens);
        if input.peek(Ident::peek_any) {
            let Ok(part) = input.call(Ident::parse_any) else {
                break;
            };
            name.push_str(&bare_css(&part));
            part.to_tokens(&mut tokens);
        } else if input.peek(syn::LitInt) || input.peek(syn::LitFloat) {
            let Ok(part) = input.parse::<syn::Lit>() else {
                break;
            };
            name.push_str(&part.to_token_stream().to_string());
            part.to_tokens(&mut tokens);
        } else if !input.peek(Token![-]) {
            break;
        }
    }
    (name, tokens)
}

/// One `property: value` in a `style:` block. `written` is the property as
/// the author wrote it (`font_size`, `"--gap"`), for messages that show
/// their own code.
struct StyleStatement {
    written: String,
    key: LitStr,
    value: LitStr,
}

/// A `style:` block's declarations, each written out whole
/// (`"font-size: 20px"`) and spanned on its property, so that a style list
/// and a class list are the same `AttrSource::List` from here on.
///
/// A loop rather than `Punctuated`: the missing-colon message depends on
/// whether a declaration came before, and the duplicate check on which did.
fn style_declarations(input: ParseStream<'_>) -> Result<Vec<LitStr>> {
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    while !input.is_empty() {
        let StyleStatement {
            written,
            key,
            value,
        } = style_statement(input, !out.is_empty())?;
        // CSS lets the last one win; a `form!` key given twice is an error,
        // and a property reads the same way.
        if seen.contains(&key.value()) {
            return Err(syn::Error::new(
                key.span(),
                format!("`{written}` is given twice"),
            ));
        }
        seen.push(key.value());
        out.push(LitStr::new(
            &format!("{}: {}", key.value(), value.value()),
            key.span(),
        ));
        if input.is_empty() {
            break;
        }
        input.parse::<Token![,]>()?;
    }
    Ok(out)
}

fn style_statement(input: ParseStream<'_>, after_another: bool) -> Result<StyleStatement> {
    let fork = input.fork();
    let written = match fork.call(Ident::parse_any) {
        Ok(ident) => ident.to_string(),
        Err(_) => fork
            .parse::<LitStr>()
            .map(|s| format!("\"{}\"", s.value()))
            .unwrap_or_default(),
    };
    let key = css_name(input, "style property", "")?;
    // `font_family: Inter, serif` ends the declaration at the comma, so
    // `serif` arrives as a property with no colon. Say so, on `serif`.
    if !input.peek(Token![:]) {
        let hint = if after_another {
            format!(". If `{written}` belongs to the value before it, quote that whole value")
        } else {
            String::new()
        };
        return Err(syn::Error::new(
            key.span(),
            format!("expected `:` after `{written}`{hint}"),
        ));
    }
    input.parse::<Token![:]>()?;
    let value = style_value(input, &written)?;
    Ok(StyleStatement {
        written,
        key,
        value,
    })
}

/// A quoted string, verbatim, or a run of bare items up to the next `,`,
/// joined with single spaces.
fn style_value(input: ParseStream<'_>, written: &str) -> Result<LitStr> {
    let span = input.span();
    if input.peek(LitStr) {
        let value: LitStr = input.parse()?;
        if !end_of_value(input) {
            return Err(not_whole(input, written, true));
        }
        return Ok(value);
    }
    let mut items = Vec::new();
    while !end_of_value(input) {
        items.push(style_item(input, written)?);
    }
    if items.is_empty() {
        return Err(syn::Error::new(span, format!("`{written}` needs a value")));
    }
    Ok(LitStr::new(&items.join(" "), span))
}

fn end_of_value(input: ParseStream<'_>) -> bool {
    input.is_empty() || input.peek(Token![,])
}

/// One bare item of a style value: a name, a number (`20px`, `-1px`, `50%`),
/// or a final `!important`. Anything else is an error asking for quotes.
fn style_item(input: ParseStream<'_>, written: &str) -> Result<String> {
    if input.peek(Token![-]) && (input.peek2(syn::LitInt) || input.peek2(syn::LitFloat)) {
        input.parse::<Token![-]>()?;
        return Ok(format!("-{}", css_number(input)?));
    }
    if input.peek(syn::LitInt) || input.peek(syn::LitFloat) {
        return css_number(input);
    }
    if input.peek(Ident::peek_any) {
        let token = input.call(Ident::parse_any)?;
        let as_str = bare_css(&token);
        // Only a `-` before a NAME joins it: `auto -1px` is two items.
        if input.peek(Token![-]) && input.peek2(Ident::peek_any) {
            return Err(hyphenated(&token, &as_str, input, "style value", ""));
        }
        return Ok(as_str);
    }
    if input.peek(Token![!]) {
        let bang: Token![!] = input.parse()?;
        return match input.call(Ident::parse_any) {
            Ok(word) if word == "important" && end_of_value(input) => Ok("!important".into()),
            Ok(word) if word == "important" => {
                Err(input.error("`!important` must come last in a value"))
            }
            _ => Err(syn::Error::new(bang.span(), quote_it(written))),
        };
    }
    Err(not_whole(input, written, false))
}

/// `20px`, `1.5rem`, `0`, with a `%` written straight after (`50%`) glued
/// on. A unit starting with `e` (`2em`) never gets here: Rust's lexer reads
/// it as an exponent and fails before the macro runs.
fn css_number(input: ParseStream<'_>) -> Result<String> {
    let lit: syn::Lit = input.parse()?;
    let mut number = lit.to_token_stream().to_string();
    if input.parse::<Option<Token![%]>>()?.is_some() {
        number.push('%');
    }
    Ok(number)
}

/// The error at a token a style value cannot take bare, or at whatever
/// follows a quoted value (`after_quoted`).
fn not_whole(input: ParseStream<'_>, written: &str, after_quoted: bool) -> syn::Error {
    if input.peek(Token![;]) {
        input.error("separate declarations with `,`, not `;`")
    } else if after_quoted || input.peek(LitStr) {
        input.error(format!(
            "a quoted value must be the whole value: `{written}: \"…\"`"
        ))
    } else {
        input.error(quote_it(written))
    }
}

fn quote_it(written: &str) -> String {
    format!("quote the whole value: `{written}: \"…\"`")
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
/// table attribute (only `maxlength` and `minlength` can be written as an
/// identifier and still differ from their key), a near miss, or else the
/// quoting rule.
///
/// No list of legal keys: with the ARIA rows it is about ninety long, and the
/// mistake it was guarding against, a custom attribute written bare, is
/// better answered by the rule itself.
fn unknown_key(name: &str) -> String {
    if let Some(attr) = Attr::from_name(name).filter(|attr| attr.owner() != Owner::Formoxus) {
        return format!(
            "unknown key `{name}`: write `{}`",
            attr.variant_name().to_snake_case()
        );
    }
    // A near miss is most likely a typo, so the full rule, whose example would
    // quote the typo, shrinks to a reminder.
    if let Some(near) = legal_keys()
        .into_iter()
        .filter(|k| edit_distance(name, k) <= 2)
        .min_by_key(|k| edit_distance(name, k))
    {
        return format!(
            "unknown key `{name}`. Did you mean `{near}`? (A custom attribute must be quoted.)"
        );
    }
    format!(
        "unknown key `{name}`. A standard HTML attribute is written unquoted, \
         in snake_case (`max_length`, `aria_label`); any other must be quoted \
         (`\"{}\": …`)",
        name.replace('_', "-")
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
    use super::{AttrSource, legal_keys, parse_value};
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
        expect_that!(msg, contains_substring("unknown key `contrl`"));
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
                        formoxus_attrs::AttrType::TokenList => quote!(#ident: [x, "y"]),
                        formoxus_attrs::AttrType::Declarations => quote!(#ident: { x: y }),
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

    /// The only two attributes whose HTML name is an identifier other than
    /// their key, so the only two this branch can ever fire for.
    #[gtest]
    fn the_html_spelling_of_a_key_gets_the_form_spelling() {
        for (html, key) in [("maxlength", "max_length"), ("minlength", "min_length")] {
            let ident = syn::Ident::new(html, proc_macro2::Span::call_site());
            expect_that!(
                err_of(quote! { Source { notes => { #ident: 3 } } }),
                eq(&format!("unknown key `{html}`: write `{key}`"))
            );
        }
    }

    #[gtest]
    fn a_near_miss_suggests_the_key_it_missed() {
        expect_that!(
            err_of(quote! { Source { notes => { max_lenght: 3 } } }),
            eq("unknown key `max_lenght`. Did you mean `max_length`? \
                (A custom attribute must be quoted.)")
        );
    }

    /// No list of every key: the rule answers the likelier mistake, a custom
    /// attribute written bare, and its example is the author's own name,
    /// hyphenated the way HTML would write it.
    #[gtest]
    fn an_unknown_key_gets_the_quoting_rule() {
        let msg = err_of(quote! { Source { notes => { hx_get: "/x" } } });
        expect_that!(msg, contains_substring("any other must be quoted"));
        expect_that!(msg, contains_substring("`\"hx-get\": …`"));
        expect_that!(msg, not(contains_substring("Did you mean")));
        expect_that!(msg, not(contains_substring("aria_valuetext")));
    }

    // ── Class lists ──────────────────────────────────────────────────────

    /// What a list attribute's value parses to, or the error's message.
    fn list_value(
        attr: formoxus_attrs::Attr,
        value: &proc_macro2::TokenStream,
    ) -> std::result::Result<Vec<String>, String> {
        use syn::parse::Parser;
        let key = syn::Ident::new(
            &heck::ToSnakeCase::to_snake_case(attr.variant_name()),
            proc_macro2::Span::call_site(),
        );
        let parser = |input: syn::parse::ParseStream<'_>| parse_value(attr, &key, input);
        match parser.parse2(quote!(: #value)) {
            Ok(AttrSource::List(items)) => Ok(items.iter().map(syn::LitStr::value).collect()),
            Ok(other) => panic!("{attr:?} parsed as {other:?}"),
            Err(e) => Err(e.to_string()),
        }
    }

    fn class_names(value: &proc_macro2::TokenStream) -> std::result::Result<Vec<String>, String> {
        list_value(formoxus_attrs::Attr::ClassPlus, value)
    }

    fn style(value: &proc_macro2::TokenStream) -> std::result::Result<Vec<String>, String> {
        list_value(formoxus_attrs::Attr::Style, value)
    }

    /// One rule: bare names turn `_` into `-`; quoted names are verbatim.
    /// Keywords and raw identifiers are names like any other.
    #[gtest]
    fn a_class_list_takes_bare_and_quoted_names() {
        expect_that!(
            class_names(&quote!([dark, my_class, "real_underscore", static, r#type,])),
            ok(elements_are![
                eq("dark"),
                eq("my-class"),
                eq("real_underscore"),
                eq("static"),
                eq("type")
            ])
        );
    }

    #[gtest]
    fn a_hyphenated_bare_class_gets_both_spellings() {
        expect_that!(
            class_names(&quote!([text - center])),
            err(eq(
                "`-` cannot appear in a bare class name: write `text_center`, \
                    or quote it: `\"text-center\"`. Separate classes need a comma \
                    between them."
            ))
        );
    }

    /// Whitespace is not a token, so a missing comma before `-mt-4` looks
    /// exactly like one name. The comma reminder is what covers this reading.
    #[gtest]
    fn a_missing_comma_reads_like_one_hyphenated_name() {
        expect_that!(
            class_names(&quote!([text - mt - 4])),
            err(all![
                contains_substring("write `text_mt_4`"),
                contains_substring("Separate classes need a comma")
            ])
        );
    }

    /// `2xl` is one token, a number with the suffix `xl`.
    #[gtest]
    fn a_hyphen_run_takes_numbers() {
        expect_that!(
            class_names(&quote!([text - 2xl])),
            err(contains_substring(
                "write `text_2xl`, or quote it: `\"text-2xl\"`"
            ))
        );
    }

    /// The bare-name rule holds past the first `-` too, so the two suggested
    /// spellings name the same class.
    #[gtest]
    fn both_spellings_name_the_same_class() {
        expect_that!(
            class_names(&quote!([text - my_size])),
            err(contains_substring(
                "write `text_my_size`, or quote it: `\"text-my-size\"`"
            ))
        );
    }

    #[gtest]
    fn a_name_with_no_bare_spelling_is_only_offered_quoted() {
        expect_that!(
            class_names(&quote!([text - 1.5])),
            err(all![
                contains_substring("quote it: `\"text-1.5\"`"),
                not(contains_substring("write `"))
            ])
        );
    }

    /// Tailwind's negative utilities. Unambiguous, unlike a `-` after a name.
    #[gtest]
    fn a_class_starting_with_a_hyphen_must_be_quoted() {
        expect_that!(
            class_names(&quote!([text, -mt - 4])),
            err(eq(
                "a class name starting with `-` must be quoted: `\"-mt-4\"`"
            ))
        );
    }

    #[gtest]
    fn a_class_list_refuses_anything_else() {
        expect_that!(
            class_names(&quote!([1])),
            err(eq("expected class name, either bare or quoted"))
        );
    }

    /// `class: []` is legal (it strips formoxus's classes); the `_plus`
    /// forms with nothing in them do nothing, so they are refused.
    #[gtest]
    fn an_empty_plus_list_is_refused() {
        expect_that!(
            class_names(&quote!([])),
            err(contains_substring(
                "`class_plus` with an empty list should be omitted"
            ))
        );
        expect_that!(
            list_value(formoxus_attrs::Attr::StylePlus, &quote!({})),
            err(contains_substring(
                "`style_plus` with an empty block should be omitted"
            ))
        );
        expect_that!(
            list_value(formoxus_attrs::Attr::Class, &quote!([])),
            ok(len(eq(0)))
        );
    }

    // ── Style blocks ─────────────────────────────────────────────────────

    /// Every bare form at once. Each declaration is written out whole, so a
    /// style list and a class list are the same `AttrSource::List`.
    #[gtest]
    fn a_style_block_takes_bare_and_quoted_values() {
        expect_that!(
            style(&quote!({
                color: red,
                font_size: 20px,
                justify_content: space_between,
                position: static,
                margin: -1px 0,
                width: 50%,
                line_height: 1.5,
                border: 1px solid black,
                display: none !important,
                font_family: "'Inter', sans-serif",
                "--gap": "4px",
            })),
            ok(elements_are![
                eq("color: red"),
                eq("font-size: 20px"),
                eq("justify-content: space-between"),
                eq("position: static"),
                eq("margin: -1px 0"),
                eq("width: 50%"),
                eq("line-height: 1.5"),
                eq("border: 1px solid black"),
                eq("display: none !important"),
                eq("font-family: 'Inter', sans-serif"),
                eq("--gap: 4px")
            ])
        );
    }

    /// Only a `-` before a NAME joins it to the name before; before a number
    /// it starts a negative one.
    #[gtest]
    fn a_name_then_a_negative_number_is_two_items() {
        expect_that!(
            style(&quote!({ margin: auto -1px })),
            ok(elements_are![eq("margin: auto -1px")])
        );
    }

    #[gtest]
    fn a_hyphenated_bare_property_gets_both_spellings() {
        expect_that!(
            style(&quote!({ font-size: 20px })),
            err(eq(
                "`-` cannot appear in a bare style property: write `font_size`, \
                    or quote it: `\"font-size\"`."
            ))
        );
    }

    /// Custom properties and vendor prefixes, with `--` read whole.
    #[gtest]
    fn a_property_starting_with_a_hyphen_must_be_quoted() {
        expect_that!(
            style(&quote!({ --gap: 4px })),
            err(eq(
                "a style property starting with `-` must be quoted: `\"--gap\"`"
            ))
        );
        expect_that!(
            style(&quote!({ -webkit-appearance: none })),
            err(eq(
                "a style property starting with `-` must be quoted: `\"-webkit-appearance\"`"
            ))
        );
    }

    #[gtest]
    fn a_hyphenated_bare_value_gets_both_spellings() {
        expect_that!(
            style(&quote!({ justify_content: space-between })),
            err(eq(
                "`-` cannot appear in a bare style value: write `space_between`, \
                    or quote it: `\"space-between\"`."
            ))
        );
    }

    /// The likeliest way to hit a missing colon: a comma inside a value ends
    /// the declaration, so the rest reads as the next property.
    #[gtest]
    fn a_comma_inside_a_value_says_to_quote_it() {
        expect_that!(
            style(&quote!({ font_family: Inter, serif })),
            err(eq(
                "expected `:` after `serif`. If `serif` belongs to the value \
                    before it, quote that whole value"
            ))
        );
    }

    /// With no declaration before it, there is no value it could belong to.
    #[gtest]
    fn a_first_property_without_a_colon_gets_no_comma_hint() {
        expect_that!(
            style(&quote!({ color red })),
            err(eq("expected `:` after `color`"))
        );
    }

    #[gtest]
    fn a_property_given_twice_is_refused() {
        expect_that!(
            style(&quote!({ color: red, font_size: 2px, color: blue })),
            err(eq("`color` is given twice"))
        );
    }

    /// Two spellings of one property are still one property.
    #[gtest]
    fn a_property_given_twice_is_refused_across_spellings() {
        expect_that!(
            style(&quote!({ font_size: 2px, "font-size": 3px })),
            err(eq("`\"font-size\"` is given twice"))
        );
    }

    #[gtest]
    fn what_a_bare_value_cannot_hold_must_be_quoted() {
        // `#fff` built from a string: inside `quote!`, `#fff` interpolates.
        let hex: proc_macro2::TokenStream = "#fff".parse().unwrap();
        for value in [hex, quote!(rgb(0, 0, 0)), quote!(12px / 1.5)] {
            expect_that!(
                style(&quote!({ color: #value })),
                err(eq("quote the whole value: `color: \"…\"`")),
                "{value}"
            );
        }
    }

    #[gtest]
    fn a_quoted_value_must_be_the_whole_value() {
        for value in [quote!(Inter "x"), quote!("Inter" serif)] {
            expect_that!(
                style(&quote!({ font_family: #value })),
                err(eq(
                    "a quoted value must be the whole value: `font_family: \"…\"`"
                )),
                "{value}"
            );
        }
    }

    #[gtest]
    fn important_must_come_last() {
        expect_that!(
            style(&quote!({ color: red !important blue })),
            err(eq("`!important` must come last in a value"))
        );
        expect_that!(
            style(&quote!({ color: red !imp })),
            err(eq("quote the whole value: `color: \"…\"`"))
        );
    }

    /// CSS habit.
    #[gtest]
    fn a_semicolon_gets_told_to_be_a_comma() {
        for value in [
            quote!({ color: red; width: 2px }),
            quote!({ color: "red"; }),
        ] {
            expect_that!(
                style(&value),
                err(eq("separate declarations with `,`, not `;`")),
                "{value}"
            );
        }
    }

    #[gtest]
    fn a_property_needs_a_value() {
        expect_that!(
            style(&quote!({ color: , width: 2px })),
            err(eq("`color` needs a value"))
        );
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
