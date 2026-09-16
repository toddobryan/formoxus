use std::collections::HashMap;

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::{Expr, Ident, Path, Result, Token, braced, ext::IdentExt, parenthesized, parse::{Parse, ParseStream}, punctuated::Punctuated};

pub fn impl_form2(input: TokenStream2) -> TokenStream2 {
    match syn::parse2::<FormSpecInput>(input) {
        Ok(spec) => spec.expand(),
        Err(err) => err.to_compile_error(),
    }
}

mod kw {
    syn::custom_keyword!(title);
    syn::custom_keyword!(validator);
    syn::custom_keyword!(buttons);
}

#[derive(Debug)]
struct FormSpecInput {
    target: Path,
    entries: Vec<Entry>,
}

#[derive(Debug)]
struct FormSpecMeta {
    model_type: Path,
    title: Option<Expr>,
    validator: Option<Expr>,
    buttons: Vec<ButtonInfo>,
    field_specs: Vec<FieldSpec>,
}

impl FormSpecMeta {
    fn new(model_type: Path) -> Self {
        Self { model_type, title: None, validator: None, buttons: Vec::new(), field_specs: Vec::new(), }
    }
}

impl FormSpecInput {
    fn expand(self) -> TokenStream2 {
        let mut fsm: FormSpecMeta = FormSpecMeta::new(self.target);
        for e in self.entries {
            match e {
                Entry::Title(expr) => fsm.title = Some(expr),
                Entry::Validator(expr) => fsm.validator = Some(expr),
                Entry::Field { path, body } => fsm.field_specs.push(
                    FieldSpec { path, label: body.label, control: body.control }
                ),
                Entry::Buttons(buttons) => fsm.buttons = buttons,
            }
        };

        let model_type: Path = fsm.model_type;
        let title: Option<TokenStream2> = fsm.title.map(|t| {
            quote! {
                .with_title(&#t)
            }
        });
        let validator: Option<TokenStream2> = fsm.validator.map(|v| {
            quote! {
                .with_validator(#v)
            }
        });
        // Omitted entirely when there are none, so a spec without buttons
        // expands to exactly what it did before they existed.
        let buttons: Option<TokenStream2> = (!fsm.buttons.is_empty()).then(|| {
            let specs = fsm.buttons.iter().map(ButtonInfo::spec_tokens);
            quote! { .with_buttons(::std::vec![ #(#specs),* ]) }
        });
        let fields: Vec<TokenStream2> = fsm.field_specs.iter().map(|f| {
            let key = f.path.key();
            let label = f.label.as_ref().map(|l| quote! { .with_label(#key, &#l) });
            let control = f.control.as_ref().map(|c| {
                let c = c.path();
                quote! { .with_custom_control(#key, #c) }
            });
            quote! { #label #control }
        }).collect();
        let witnesses: Vec<TokenStream2> = fsm
            .field_specs
            .iter()
            .map(|f| probe(&f.path.segments, quote!(__s), 0))
            .collect();

        quote! {
            {
                #[allow(unused)]
                fn __paths_exist(__s: &#model_type) {
                    #(#witnesses)*
                }

                ::formoxus::reflect::form::FormSpec::<#model_type>::new()
                #title
                #validator
                #buttons
                #(#fields)*         
            }
        }
    }
}

/// One statement per spec path, proving the path exists on the model.
///
/// Never executed — it exists so rustc checks the access. `[]` becomes a loop,
/// which is what lets a row's field be named at all: the element type is
/// inferred, so nothing has to spell it out.
fn probe(segments: &[Segment], base: TokenStream2, depth: usize) -> TokenStream2 {
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
                // with a call-site span rustc underlines the whole `form2!`
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

impl Parse for FormSpecInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let target: Path = input.parse()?;
        let body;
        braced!(body in input);
        let entries: Vec<Entry> = 
            Punctuated::<Entry, Token![,]>::parse_terminated(&body)?.into_iter().collect();
        let mut seen: HashMap<String, ()> = HashMap::new();
        let (mut had_title, mut had_validator, mut had_buttons) = (false, false, false);
        for e in &entries {
            match e {
                Entry::Title(_) if had_title =>
                    return Err(body.error("`title` is given twice")),
                Entry::Title(_) => had_title = true,
                Entry::Validator(_) if had_validator =>
                    return Err(body.error("`validator` is given twice")),
                Entry::Validator(_) => had_validator = true,
                Entry::Buttons(_) if had_buttons =>
                    return Err(body.error("`buttons` is given twice")),
                Entry::Buttons(_) => had_buttons = true,
                Entry::Field { path, .. } => {
                    let key = path.key();
                    if seen.insert(key.clone(), ()).is_some() {
                        return Err(syn::Error::new(
                            path.span(),
                            format!("`{key}` is specified twice — merge the two bodies into one"),
                        ));
                    }
                }
            }
        }
        Ok(FormSpecInput { target, entries })
    }
}

#[derive(Debug)]
enum Entry {
    Title(Expr),
    Validator(Expr),
    Field { path: SpecPath, body: FieldBody },
    Buttons(Vec<ButtonInfo>),
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

    /// `buttons: { save: { type: submit, text: "Save" }, … }`
    ///
    /// **Static knowledge only** — which buttons exist, in what order, how each
    /// one renders, and whether it validates first. What a button *does* is
    /// supplied at the render call site instead; `BUTTONS_PLAN.md` §2 records
    /// why that split is forced rather than chosen.
    ///
    /// Source order is display order, so this collects a `Vec` and not a map —
    /// the author's order is the only ordering information there is.
    fn parse_buttons(input: ParseStream<'_>) -> Result<Self> {
        let _buttons: kw::buttons = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        if !input.peek(syn::token::Brace) {
            return Err(input.error("expected a buttons block surrounded by braces"));
        }
        let body;
        let braces = braced!(body in input);

        let mut buttons: Vec<ButtonInfo> = Vec::new();
        while !body.is_empty() {
            let name: Ident = body.parse()?;
            let _colon: Token![:] = body.parse()?;
            // Each name becomes one slot in the generated handler struct, so a
            // repeat is a duplicate field rather than anything an author could
            // have meant by it.
            if buttons.iter().any(|b| b.name == name) {
                return Err(syn::Error::new_spanned(
                    &name,
                    format!("`{name}` is specified twice — merge the two bodies into one"),
                ));
            }
            buttons.push(ButtonInfo::parse_body(name, &body)?);
            if body.peek(Token![,]) {
                body.parse::<Token![,]>()?;
            }
        }

        if buttons.is_empty() {
            return Err(syn::Error::new(braces.span.join(), "empty buttons block"));
        }
        Ok(Entry::Buttons(buttons))
    }
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        if attribute(input, kw::title) {
            Entry::parse_title(input)
        } else if attribute(input, kw::validator) {
            Entry::parse_validator(input)
        } else if attribute(input, kw::buttons) {
            Entry::parse_buttons(input)
        } else if input.peek(syn::Ident) {
            Entry::parse_field(input)
        } else {
            Err(input.error("expected a form attribute or a field specifier"))
        }
    }
}

/// Is this entry the attribute `kw`, rather than a field whose name happens to
/// be that word?
///
/// A `custom_keyword!` is still an `Ident`, so `title` on its own cannot tell
/// the two apart — and a model with a `title` field is not a corner case, it is
/// most of them. The second token settles it: an attribute is `name:` and a
/// field spec is `name =>`, so a following `:` means attribute and anything
/// else means field.
///
/// **Testing for `:` rather than against `=>`.** The `=>` does not have to be
/// the second token — `title.text => …` and `titles[] => …` are both legal
/// paths rooted at a keyword, with `.` and `[` sitting where the `=>` would be.
/// Only `:` is guaranteed to be in this position, and only for an attribute.
fn attribute<K: syn::parse::Peek>(input: ParseStream<'_>, kw: K) -> bool
where
    K::Token: syn::token::Token,
{
    input.peek(kw) && input.peek2(Token![:])
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

    fn span(&self) -> proc_macro2::Span {
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

#[derive(Debug)]
struct FieldSpec {
    path: SpecPath,
    label: Option<Expr>,
    control: Option<ControlRef>,
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

/// The author-facing control vocabulary, and the `ControlType` each name means.
///
/// **Flat and lowercase, deliberately.** `ControlType::Input(InputType::Password)`
/// is the shape of formoxus's own enum — grouping the `<input type=X>` family
/// under one variant is a dispatch convenience, not a concept an author has. HTML
/// spells it `type="password"` and so does this. That makes this table the stable
/// surface: the enum below it can be regrouped, renamed, or split without
/// touching a single `form2!` call.
///
/// Names follow HTML where HTML has one (`tel`, not `telephone`; `datetime_local`
/// for `datetime-local`, since a hyphen cannot be an ident) and formoxus where it
/// does not (`integer`/`float`, which both become `type="number"`; `textarea`,
/// `select`).
///
/// Every name here is accepted whether or not `ScalarInput` can render it yet.
/// Gating on that was considered and REJECTED: the macro crate cannot see
/// `ScalarInput`'s match arms, so an "implemented" list would be a hand-kept copy
/// of a match in another crate — a worse sync hazard than the one this table
/// already has — and its first false positive would be `password`. An unwired
/// control still panics at render, naming both the control and the value kind.
macro_rules! controls {
    (
        $( $name:ident => $variant:ident $( ( $input:ident ) )? ),* $(,)?
    ) => {
        /// The tokens for a known control name, or `None` if it is not one.
        fn control_tokens(name: &Ident) -> Option<TokenStream2> {
            match name.to_string().as_str() {
                $( stringify!($name) => Some(quote! {
                    ::formoxus::reflect::widgets::ControlType::$variant
                    $( ( ::formoxus::reflect::widgets::InputType::$input ) )?
                }), )*
                _ => None,
            }
        }

        /// Every accepted name, for the "unknown control" message. Generated from
        /// the same table as the match, so the two cannot disagree.
        const CONTROL_NAMES: &[&str] = &[ $( stringify!($name) ),* ];
    };
}

controls! {
    // <input type=…>
    text           => Input(Text),
    password       => Input(Password),
    hidden         => Input(Hidden),
    // Selectable, but not the DEFAULT for a numeric field: `type="number"` hands
    // back `""` for anything the browser dislikes, so a half-typed value
    // vanishes. A numeric renders as text unless someone asks for this.
    number         => Input(Number),
    email          => Input(Email),
    tel            => Input(Telephone),
    url            => Input(Url),
    search         => Input(Search),
    color          => Input(Color),
    date           => Input(Date),
    time           => Input(Time),
    datetime_local => Input(DatetimeLocal),
    month          => Input(Month),
    week           => Input(Week),

    // Everything that is not an <input>.
    textarea          => Textarea,
    select            => Select,
    select_multiple   => SelectMultiple,
    checkbox          => Checkbox,
    checkbox_multiple => CheckboxMultiple,
    radio_group       => RadioGroup,
    file              => File,
}

mod control_kw {
    syn::custom_keyword!(custom);
}

#[derive(Debug)]
enum ControlRef {
    /// One of the names in [`controls!`], already validated.
    Named(Ident),
    /// `custom(MarkdownWidget)` — a `Path`, not an `Ident`, so that
    /// `custom(widgets::MarkdownWidget)` works without importing the widget.
    Custom(Path),
}

impl ControlRef {
    /// The `ControlType` expression this names, fully qualified.
    ///
    /// Infallible: `parse` rejected anything not in the table, so the lookup here
    /// cannot miss.
    fn path(&self) -> TokenStream2 {
        match self {
            Self::Named(name) => control_tokens(name)
                .expect("parse rejects names that are not in the table"),
            // A NON-CAPTURING closure, which coerces to `fn(ControlProps) ->
            // Element`. The widget goes inside `rsx!` rather than being called,
            // so it gets a component scope of its own and may use hooks.
            //
            // The name is carried separately because `Debug` on a fn pointer
            // prints an address, and panic messages and test assertions want
            // "MarkdownWidget".
            Self::Custom(widget) => {
                let name = last_segment_string(widget);
                quote! {
                    ::formoxus::reflect::widgets::ControlType::Custom {
                        name: #name,
                        render: |__p| ::dioxus::prelude::rsx! {
                            #widget { values: __p.values, props: __p.props }
                        },
                    }
                }
            }
        }
    }
}

/// The last segment of a path, as a string — `"MarkdownWidget"` for
/// `widgets::MarkdownWidget`.
fn last_segment_string(path: &Path) -> String {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

impl Parse for ControlRef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(control_kw::custom) {
            let kw: control_kw::custom = input.parse()?;
            if !input.peek(syn::token::Paren) {
                return Err(syn::Error::new(
                    kw.span,
                    "`custom` needs the widget it renders — write `custom(MyWidget)`",
                ));
            }
            let inner;
            parenthesized!(inner in input);
            let widget: Path = inner.parse()?;
            if !inner.is_empty() {
                return Err(inner.error("`custom` takes one widget and nothing else"));
            }
            return Ok(Self::Custom(widget));
        }

        let name: Ident = input.parse()?;
        if control_tokens(&name).is_some() {
            Ok(Self::Named(name))
        } else {
            Err(syn::Error::new(name.span(), unknown_control(&name)))
        }
    }
}

/// The message for a name that is not in the table.
///
/// Three cases, in order of how likely the author is to have meant them: a name
/// that IS in the table under a different case (`Password`, `DatetimeLocal` — the
/// old `ControlType`-shaped spelling), a near miss, and no idea.
fn unknown_control(name: &Ident) -> String {
    let written = name.to_string();
    let lowered = to_snake(&written);
    if lowered != written && CONTROL_NAMES.contains(&lowered.as_str()) {
        return format!(
            "unknown control `{written}` — control names are lowercase, write `{lowered}`"
        );
    }
    match CONTROL_NAMES
        .iter()
        .filter(|n| edit_distance(&lowered, n) <= 2)
        .min_by_key(|n| edit_distance(&lowered, n))
    {
        Some(near) => format!("unknown control `{written}` — did you mean `{near}`?"),
        None => format!(
            "unknown control `{written}` — expected one of {}, or `custom(MyWidget)`",
            CONTROL_NAMES.join(", ")
        ),
    }
}

/// `DatetimeLocal` -> `datetime_local`. Only good enough to recognise the
/// `ControlType`/`InputType` spellings an author might copy from the enum.
fn to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for (i, c) in s.char_indices() {
        if c.is_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Levenshtein distance, for "did you mean". Small inputs, so the simple
/// two-row version is plenty.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for (i, ca) in a.chars().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == *cb { 0 } else { 1 };
            cur[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ButtonInfo {
    pub name: syn::Ident,
    pub ty: ButtonType,
    pub text: Option<String>,
    pub invocation: Option<Invocation>,
}

impl ButtonInfo {
    /// The `ButtonSpec` this declares, fully qualified.
    ///
    /// The name goes across as a string because it is a map key at render, not
    /// an identifier — the reflection path has no per-form type to hang a field
    /// on, which is the whole reason handlers are reconciled at runtime.
    fn spec_tokens(&self) -> TokenStream2 {
        let name = self.name.to_string();
        let ty = self.ty.tokens();
        let text = self.text.as_ref().map(|t| quote! { .with_text(#t) });
        let invocation = self.invocation.as_ref().map(|i| {
            let i = i.tokens();
            quote! { .with_invocation(#i) }
        });
        quote! {
            ::formoxus::reflect::buttons::ButtonSpec::new(#name, #ty) #text #invocation
        }
    }

    /// The braced body after `name:`. `type` is the only required key; `text`
    /// falls back to the name run through the form's label casing, and
    /// `invocation` to whatever the type implies.
    fn parse_body(name: Ident, input: ParseStream<'_>) -> Result<Self> {
        if !input.peek(syn::token::Brace) {
            return Err(input.error("expected a button body surrounded by braces"));
        }
        let body;
        let braces = braced!(body in input);

        let mut ty: Option<ButtonType> = None;
        let mut text: Option<String> = None;
        let mut invocation: Option<Invocation> = None;
        while !body.is_empty() {
            // `parse_any`, not the plain `Ident` parse: `type` is a reserved
            // word, so the ordinary parse rejects the one key that is required.
            // HTML spells the attribute `type` and so does this — `kind` would
            // be a second name for a thing the author already knows.
            let key = Ident::parse_any(&body)?;
            let _colon: Token![:] = body.parse()?;
            match key.to_string().as_str() {
                "type" if ty.is_some() =>
                    return Err(syn::Error::new_spanned(&key, "duplicate type")),
                "type" => ty = Some(body.parse()?),
                "text" if text.is_some() =>
                    return Err(syn::Error::new_spanned(&key, "duplicate text")),
                // A literal, not an `Expr`: the text is baked into the spec, and
                // `ButtonInfo::text` on the derive path is a `String` too.
                "text" => text = Some(body.parse::<syn::LitStr>()?.value()),
                "invocation" if invocation.is_some() =>
                    return Err(syn::Error::new_spanned(&key, "duplicate invocation")),
                "invocation" => invocation = Some(body.parse()?),
                other => return Err(syn::Error::new_spanned(
                    &key,
                    format!("unknown key {other}, expected type, text, or invocation"),
                )),
            }
            if body.peek(Token![,]) {
                body.parse::<Token![,]>()?;
            }
        }

        let Some(ty) = ty else {
            // Nothing sensible to default to: every other key describes a
            // button that already exists, and the type is what decides whether
            // it submits, resets, or merely runs a handler.
            return Err(syn::Error::new(
                braces.span.join(),
                format!(
                    "`{name}` needs a `type` — one of {}",
                    BUTTON_TYPE_NAMES.join(", ")
                ),
            ));
        };
        Ok(ButtonInfo { name, ty, text, invocation })
    }
}

/// When the button's handler runs, overriding what its type implies.
///
/// Spelled out rather than borrowed from the derive path's
/// `handler: validated | unchecked`: `validated` names the *handler*, and what
/// is actually being chosen is whether the model has to pass validation before
/// the handler is reached at all.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Invocation {
    IfModelValidates,
    Unconditional,
}

impl Invocation {
    fn tokens(&self) -> TokenStream2 {
        let variant = match self {
            Invocation::IfModelValidates => quote!(IfModelValidates),
            Invocation::Unconditional => quote!(Unconditional),
        };
        quote! { ::formoxus::reflect::buttons::Invocation::#variant }
    }
}

impl Parse for Invocation {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        match name.to_string().as_str() {
            "if_model_validates" => Ok(Invocation::IfModelValidates),
            "unconditional" => Ok(Invocation::Unconditional),
            other => Err(syn::Error::new(
                name.span(),
                format!(
                    "unknown invocation `{other}` — expected if_model_validates or unconditional"
                ),
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ButtonType {
    Destructive,
    Reset,
    Cancel,
    Button,
    Submit,
}

/// The author-facing button-type vocabulary, same shape and same reasoning as
/// [`controls!`]: one table generates both the parse and the list the error
/// message reads from, so the two cannot drift apart.
///
/// Lowercase for the same reason control names are — three of these five are
/// literally the HTML `type` attribute, and the author is writing HTML's word.
macro_rules! button_types {
    ( $( $name:ident => $variant:ident ),* $(,)? ) => {
        impl Parse for ButtonType {
            fn parse(input: ParseStream<'_>) -> Result<Self> {
                let name: Ident = input.parse()?;
                match name.to_string().as_str() {
                    $( stringify!($name) => Ok(ButtonType::$variant), )*
                    _ => Err(syn::Error::new(name.span(), unknown_button_type(&name))),
                }
            }
        }

        impl ButtonType {
            /// The `ButtonType` variant this names, fully qualified. Generated
            /// from the same table as the parse, so a new type cannot be
            /// accepted and then fail to expand.
            fn tokens(&self) -> TokenStream2 {
                match self {
                    $( ButtonType::$variant => quote! {
                        ::formoxus::reflect::buttons::ButtonType::$variant
                    }, )*
                }
            }
        }

        const BUTTON_TYPE_NAMES: &[&str] = &[ $( stringify!($name) ),* ];
    };
}

button_types! {
    submit      => Submit,
    reset       => Reset,
    cancel      => Cancel,
    destructive => Destructive,
    button      => Button,
}

/// The message for a button type that is not in the table — the same three
/// cases, in the same order, as [`unknown_control`].
fn unknown_button_type(name: &Ident) -> String {
    let written = name.to_string();
    let lowered = to_snake(&written);
    if lowered != written && BUTTON_TYPE_NAMES.contains(&lowered.as_str()) {
        return format!(
            "unknown button type `{written}` — type names are lowercase, write `{lowered}`"
        );
    }
    match BUTTON_TYPE_NAMES
        .iter()
        .filter(|n| edit_distance(&lowered, n) <= 2)
        .min_by_key(|n| edit_distance(&lowered, n))
    {
        Some(near) => format!("unknown button type `{written}` — did you mean `{near}`?"),
        None => format!(
            "unknown button type `{written}` — expected one of {}",
            BUTTON_TYPE_NAMES.join(", ")
        ),
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
                Entry::Buttons(_) => "buttons",
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
                        Some(ControlRef::Named(n)) => n.to_string(),
                        Some(ControlRef::Custom(w)) => {
                            format!("custom({})", quote!(#w))
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

    /// Buttons as `("name", "Type", "text")`, in source order, with `""` for
    /// absent text. Flattened across entries, though the parser allows only one
    /// `buttons` block.
    fn buttons(spec: &FormSpecInput) -> Vec<(String, String, String)> {
        spec.entries
            .iter()
            .filter_map(|e| match e {
                Entry::Buttons(bs) => Some(bs),
                _ => None,
            })
            .flatten()
            .map(|b| {
                (
                    b.name.to_string(),
                    format!("{:?}", b.ty),
                    b.text.clone().unwrap_or_default(),
                )
            })
            .collect()
    }

    // ── The two real forms ───────────────────────────────────────────────

    #[gtest]
    fn the_login_form_parses() {
        let spec = parse(quote! {
            LoginForm {
                title: "Sign In",
                password => { control: password },
            }
        })
        .expect("LoginForm should parse");

        expect_that!(kinds(&spec), elements_are![eq(&"title"), eq(&"field")]);
        expect_that!(
            fields(&spec),
            elements_are![eq(&(
                "password".to_string(),
                "password".to_string(),
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
                current_password => { control: password },
                new_password => { control: password, label: "New password" },
                confirm_new_password => { control: password, label: "Confirm new password" },
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
        let spec = parse(quote! { Source { notes => { control: textarea } } }).unwrap();
        expect_that!(
            fields(&spec),
            elements_are![eq(&("notes".to_string(), "textarea".to_string(), String::new()))]
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
                notes => { control: textarea, label: "Notes", },
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
        let spec = parse(quote! { Quiz { answers[] => { control: textarea } } }).unwrap();
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
        let spec = parse(quote! { Grid { rows[].cells[] => { control: textarea } } }).unwrap();
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
        let msg = err_of(quote! { Source { notes => { contrl: textarea } } });
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
            err_of(quote! { Source { notes { control: textarea } } }),
            contains_substring("=>")
        );
    }

    #[gtest]
    fn a_field_body_must_be_braced() {
        expect_that!(
            err_of(quote! { Source { notes => textarea } }),
            contains_substring("braces")
        );
    }

    #[gtest]
    fn a_nonsense_entry_is_rejected() {
        expect_that!(err_of(quote! { Source { 42 } }).len(), gt(0));
    }

    // ── One entry per path ───────────────────────────────────────────────
    //
    // Two entries naming the same path would silently MERGE rather than
    // conflict: both go through `fields.entry(path).or_default()`, so a second
    // `label` overwrites the first and leaves the control in place. Nothing
    // downstream can notice, which is why the parser has to.

    #[gtest]
    fn a_repeated_path_is_rejected() {
        let msg = err_of(quote! {
            Source {
                notes => { label: "Notes" },
                notes => { control: textarea },
            }
        });
        expect_that!(msg, contains_substring("notes"));
        expect_that!(msg, contains_substring("twice"));
    }

    #[gtest]
    fn a_repeated_nested_path_is_rejected() {
        expect_that!(
            err_of(quote! {
                Event {
                    venue.city => { label: "City" },
                    venue.city => { label: "Town" },
                }
            }),
            contains_substring("venue.city")
        );
    }

    #[gtest]
    fn a_repeated_row_selector_is_rejected() {
        // `[]` is part of the key, so the comparison has to be on the rendered
        // key rather than on the idents alone.
        expect_that!(
            err_of(quote! {
                Trip {
                    venues[].city => { label: "City" },
                    venues[].city => { control: textarea },
                }
            }),
            contains_substring("venues[].city")
        );
    }

    // ── Keyword-named fields ─────────────────────────────────────────────
    //
    // `title`, `validator` and `buttons` are `custom_keyword!`s and therefore
    // also idents, so a model field with one of those names collides with the
    // attribute of the same name. The second token disambiguates: `:` is an
    // attribute, anything else is a field path.

    #[gtest]
    fn a_field_may_be_named_title() {
        // Not a corner case — most models with prose in them have a `title`.
        let spec = parse(quote! { Article { title => { label: "Headline" } } })
            .expect("a field named `title` should parse as a field");
        expect_that!(kinds(&spec), elements_are![eq(&"field")]);
        expect_that!(fields(&spec)[0].0, eq("title"));
    }

    #[gtest]
    fn a_field_may_be_named_validator_or_buttons() {
        let spec = parse(quote! {
            Weird {
                validator => { label: "Validator" },
                buttons => { control: textarea },
            }
        })
        .expect("fields named after the other two attributes should parse too");
        expect_that!(kinds(&spec), elements_are![eq(&"field"), eq(&"field")]);
    }

    #[gtest]
    fn an_attribute_and_a_field_of_the_same_name_coexist() {
        // The form is titled "Article" and also HAS a title. Nothing about one
        // should consume the other.
        let spec = parse(quote! {
            Article {
                title: "Article",
                title => { label: "Headline" },
            }
        })
        .expect("a `title:` attribute and a `title` field are different entries");
        expect_that!(kinds(&spec), elements_are![eq(&"title"), eq(&"field")]);
        expect_that!(fields(&spec)[0].2, eq("\"Headline\""));
    }

    #[gtest]
    fn a_path_rooted_at_a_keyword_is_a_field() {
        // Why the test is for `:` rather than against `=>`: here the second
        // token is `.`, so "not a fat arrow" would have sent this to
        // `parse_title` and failed on the missing colon.
        let spec = parse(quote! { Page { title.text => { control: textarea } } })
            .expect("a dotted path rooted at a keyword should parse as a field");
        expect_that!(fields(&spec)[0].0, eq("title.text"));
    }

    #[gtest]
    fn a_row_selector_on_a_keyword_name_is_a_field() {
        // Same hazard with `[` in the second position instead of `.`.
        let spec = parse(quote! { Deck { buttons[].text => { control: text } } })
            .expect("a row selector rooted at a keyword should parse as a field");
        expect_that!(fields(&spec)[0].0, eq("buttons[].text"));
    }

    // ── Buttons ──────────────────────────────────────────────────────────

    #[gtest]
    fn a_buttons_block_parses() {
        // The shape `tests/reflect/buttons.rs` asks for, verbatim. Also the
        // regression test for dispatch order: `buttons` is a custom keyword and
        // therefore also an `Ident`, so if the field arm is tried first this
        // never reaches `parse_buttons` and dies on the missing `=>`.
        let spec = parse(quote! {
            FakeFormWithButtons {
                some_data => { control: textarea },
                buttons: {
                    delete: { type: destructive, text: "Drop" },
                    reload: { type: reset },
                    update: { type: submit, text: "Save to Db" },
                }
            }
        })
        .expect("a buttons block should parse");

        expect_that!(kinds(&spec), elements_are![eq(&"field"), eq(&"buttons")]);
        expect_that!(
            buttons(&spec),
            elements_are![
                eq(&("delete".to_string(), "Destructive".to_string(), "Drop".to_string())),
                eq(&("reload".to_string(), "Reset".to_string(), String::new())),
                eq(&("update".to_string(), "Submit".to_string(), "Save to Db".to_string())),
            ]
        );
    }

    #[gtest]
    fn button_order_is_source_order() {
        // Display order is the only thing the author can say about layout, so
        // the parser must not sort or dedup into a map.
        let spec = parse(quote! {
            Source {
                buttons: {
                    zebra: { type: cancel },
                    apple: { type: submit },
                }
            }
        })
        .unwrap();
        expect_that!(
            buttons(&spec).iter().map(|b| b.0.clone()).collect::<Vec<_>>(),
            elements_are![eq("zebra"), eq("apple")]
        );
    }

    #[gtest]
    fn every_button_type_is_accepted() {
        let spec = parse(quote! {
            Source {
                buttons: {
                    a: { type: submit },
                    b: { type: reset },
                    c: { type: cancel },
                    d: { type: destructive },
                    e: { type: button },
                }
            }
        })
        .unwrap();
        expect_that!(
            buttons(&spec).iter().map(|b| b.1.clone()).collect::<Vec<_>>(),
            elements_are![
                eq("Submit"),
                eq("Reset"),
                eq("Cancel"),
                eq("Destructive"),
                eq("Button")
            ]
        );
    }

    #[gtest]
    fn an_invocation_overrides_what_the_type_implies() {
        let spec = parse(quote! {
            Source {
                buttons: { preview: { type: button, invocation: if_model_validates } }
            }
        })
        .unwrap();
        let Entry::Buttons(bs) = &spec.entries[0] else {
            panic!("expected a buttons entry");
        };
        expect_that!(bs[0].invocation, some(eq(&Invocation::IfModelValidates)));
    }

    #[gtest]
    fn an_absent_invocation_stays_none() {
        // `None` means "whatever the type implies", which is a different fact
        // from either variant — the renderer needs to be able to tell.
        let spec = parse(quote! { Source { buttons: { go: { type: submit } } } }).unwrap();
        let Entry::Buttons(bs) = &spec.entries[0] else {
            panic!("expected a buttons entry");
        };
        expect_that!(bs[0].invocation, none());
    }

    #[gtest]
    fn trailing_commas_are_allowed_in_both_button_braces() {
        parse(quote! {
            Source {
                buttons: {
                    go: { type: submit, text: "Go", },
                },
            }
        })
        .expect("trailing commas in a buttons block should parse");
    }

    // ── Button errors ────────────────────────────────────────────────────

    #[gtest]
    fn a_button_needs_a_type() {
        let msg = err_of(quote! { Source { buttons: { go: { text: "Go" } } } });
        expect_that!(msg, contains_substring("go"));
        expect_that!(msg, contains_substring("type"));
    }

    #[gtest]
    fn an_unknown_button_type_suggests_a_near_miss() {
        expect_that!(
            err_of(quote! { Source { buttons: { go: { type: submitt } } } }),
            contains_substring("did you mean `submit`")
        );
    }

    #[gtest]
    fn a_capitalized_button_type_is_told_to_lowercase() {
        // The spelling an author copies off the `ButtonType` enum.
        expect_that!(
            err_of(quote! { Source { buttons: { go: { type: Submit } } } }),
            contains_substring("write `submit`")
        );
    }

    #[gtest]
    fn an_unknown_button_key_names_itself() {
        let msg = err_of(quote! { Source { buttons: { go: { type: submit, clas: "x" } } } });
        expect_that!(msg, contains_substring("clas"));
        expect_that!(msg, contains_substring("expected type, text, or invocation"));
    }

    #[gtest]
    fn a_duplicate_button_key_is_rejected() {
        expect_that!(
            err_of(quote! { Source { buttons: { go: { type: submit, type: reset } } } }),
            contains_substring("duplicate")
        );
    }

    #[gtest]
    fn a_repeated_button_name_is_rejected() {
        // Two slots of the same name in the generated handler struct.
        let msg = err_of(quote! {
            Source {
                buttons: {
                    go: { type: submit },
                    go: { type: reset },
                }
            }
        });
        expect_that!(msg, contains_substring("go"));
        expect_that!(msg, contains_substring("twice"));
    }

    #[gtest]
    fn two_buttons_blocks_are_rejected() {
        expect_that!(
            err_of(quote! {
                Source {
                    buttons: { go: { type: submit } },
                    buttons: { stop: { type: cancel } },
                }
            }),
            contains_substring("`buttons` is given twice")
        );
    }

    #[gtest]
    fn an_empty_buttons_block_is_rejected() {
        expect_that!(
            err_of(quote! { Source { buttons: {} } }),
            contains_substring("empty buttons block")
        );
    }

    #[gtest]
    fn a_buttons_block_must_be_braced() {
        expect_that!(
            err_of(quote! { Source { buttons: submit } }),
            contains_substring("braces")
        );
    }

    #[gtest]
    fn a_button_body_must_be_braced() {
        expect_that!(
            err_of(quote! { Source { buttons: { go: submit } } }),
            contains_substring("braces")
        );
    }

    #[gtest]
    fn button_text_must_be_a_literal() {
        // An `Expr` would let a `const` through and make the spec non-constant;
        // the derive path's `text` is a `String` for the same reason.
        expect_that!(
            err_of(quote! { Source { buttons: { go: { type: submit, text: SAVE } } } }).len(),
            gt(0)
        );
    }

    #[gtest]
    fn a_list_and_its_rows_are_different_paths() {
        // The collision that must NOT fire. `answers` is the `ListSet` (its
        // legend) and `answers[]` is every row; naming both in one spec is the
        // normal way to label a list and give its rows a control. A dedup on the
        // idents alone would reject this.
        let spec = parse(quote! {
            Quiz {
                answers => { label: "Answers" },
                answers[] => { control: textarea },
            }
        })
        .expect("a list and its rows are separate targets");
        expect_that!(
            fields(&spec).iter().map(|f| f.0.clone()).collect::<Vec<_>>(),
            elements_are![eq("answers"), eq("answers[]")]
        );
    }

    #[gtest]
    fn two_levels_of_the_same_list_are_different_paths() {
        // Likewise `rows[]` vs `rows[].cells[]` — a prefix is not a duplicate.
        let spec = parse(quote! {
            Grid {
                rows[] => { label: "Row" },
                rows[].cells[] => { control: textarea },
            }
        })
        .expect("a row and a row's field are separate targets");
        expect_that!(fields(&spec).len(), eq(2));
    }

    #[gtest]
    fn a_second_title_is_rejected() {
        expect_that!(
            err_of(quote! { Source { title: "One", title: "Two" } }),
            contains_substring("title")
        );
    }

    #[gtest]
    fn a_second_validator_is_rejected() {
        // The one likelier to be a real mistake: two cross-field checks read as
        // if they would both run, and silently only the second would.
        expect_that!(
            err_of(quote! {
                Source {
                    validator: check_dates,
                    validator: check_urls,
                }
            }),
            contains_substring("validator")
        );
    }
    // ── The control vocabulary ───────────────────────────────────────────

    /// The `ControlRef` of the first field entry.
    fn control_of(spec: &FormSpecInput) -> &ControlRef {
        spec.entries
            .iter()
            .find_map(|e| match e {
                Entry::Field { body, .. } => body.control.as_ref(),
                _ => None,
            })
            .expect("a field with a control")
    }

    #[gtest]
    fn every_name_in_the_table_parses_and_resolves() {
        // Generated from `CONTROL_NAMES`, so a name added to the table without a
        // match arm — or the reverse — fails here rather than at a call site.
        for name in CONTROL_NAMES {
            let id = Ident::new(name, proc_macro2::Span::call_site());
            let spec = parse(quote! { Source { f => { control: #id } } })
                .unwrap_or_else(|e| panic!("`{name}` should parse: {e}"));
            let tokens = control_of(&spec).path().to_string();
            expect_that!(
                &tokens,
                contains_substring(":: formoxus :: reflect :: widgets :: ControlType ::"),
                "for control `{name}`"
            );
        }
    }

    #[gtest]
    fn a_name_resolves_to_the_qualified_two_level_path() {
        // The whole point of the flat vocabulary: `password` on the outside,
        // `Input(Password)` on the inside, and the author never sees the split.
        let spec = parse(quote! { LoginForm { password => { control: password } } }).unwrap();
        let tokens = control_of(&spec).path().to_string();
        expect_that!(tokens, contains_substring("ControlType :: Input"));
        expect_that!(tokens, contains_substring("InputType :: Password"));
    }

    #[gtest]
    fn a_non_input_name_resolves_to_a_bare_variant() {
        let spec = parse(quote! { Source { notes => { control: textarea } } }).unwrap();
        let tokens = control_of(&spec).path().to_string();
        expect_that!(tokens, contains_substring("ControlType :: Textarea"));
        expect_that!(tokens, not(contains_substring("InputType")));
    }

    #[gtest]
    fn an_html_spelling_wins_over_the_enums() {
        // `tel` is HTML's name; the variant is `Telephone`. The divergence is the
        // decoupling working — an author writes what HTML calls it.
        let spec = parse(quote! { Source { phone => { control: tel } } }).unwrap();
        expect_that!(
            control_of(&spec).path().to_string(),
            contains_substring("InputType :: Telephone")
        );
    }

    #[gtest]
    fn a_hyphenated_html_name_is_snake_cased() {
        let spec = parse(quote! { Source { at => { control: datetime_local } } }).unwrap();
        expect_that!(
            control_of(&spec).path().to_string(),
            contains_substring("InputType :: DatetimeLocal")
        );
    }

    #[gtest]
    fn a_near_miss_is_suggested() {
        let msg = err_of(quote! { Source { p => { control: passwrod } } });
        expect_that!(msg, contains_substring("passwrod"));
        expect_that!(msg, contains_substring("did you mean `password`"));
    }

    #[gtest]
    fn the_enum_spelling_is_named_as_a_case_error() {
        // Someone reading `ControlType` will try `Password` and `DatetimeLocal`.
        // Both are in the table under another case, so say so instead of guessing.
        expect_that!(
            err_of(quote! { Source { p => { control: Password } } }),
            contains_substring("control names are lowercase, write `password`")
        );
        expect_that!(
            err_of(quote! { Source { p => { control: DatetimeLocal } } }),
            contains_substring("write `datetime_local`")
        );
    }

    #[gtest]
    fn a_nonsense_control_lists_the_vocabulary() {
        let msg = err_of(quote! { Source { p => { control: fluorescent } } });
        expect_that!(msg, contains_substring("textarea"));
        expect_that!(msg, contains_substring("custom(MyWidget)"));
    }

    #[gtest]
    fn number_is_selectable_but_spelled_only_one_way() {
        // `number` is in the vocabulary — it is a real HTML type, and someone may
        // want the spinner. What is NOT there is `integer`/`float`: those were
        // `InputType` variant names, never HTML ones, and two spellings for one
        // attribute is the drift this table exists to prevent.
        let spec = parse(quote! { Q { n => { control: number } } }).unwrap();
        expect_that!(
            control_of(&spec).path().to_string(),
            contains_substring("InputType :: Number")
        );
        expect_that!(
            err_of(quote! { Q { n => { control: integer } } }),
            contains_substring("unknown control `integer`")
        );
    }

    #[gtest]
    fn the_old_two_level_spelling_is_rejected() {
        // `Input(Password)` was the spelling before the table existed. It now
        // fails on `Input`, which is not a name — and `input` is not one either,
        // so the message falls through to the vocabulary list.
        expect_that!(err_of(quote! { Source { p => { control: Input(Password) } } }).len(), gt(0));
    }

    // ── `custom(…)` ──────────────────────────────────────────────────────

    #[gtest]
    fn a_custom_widget_parses() {
        let spec = parse(quote! { Source { notes => { control: custom(MarkdownWidget) } } })
            .expect("custom(MarkdownWidget) should parse");
        expect_that!(fields(&spec)[0].1, contains_substring("MarkdownWidget"));
    }

    #[gtest]
    fn a_custom_widget_may_be_module_qualified() {
        // The reason it is a `Path` and not an `Ident`: naming a widget should not
        // require importing it.
        let spec = parse(quote! { Source { notes => { control: custom(widgets::MarkdownWidget) } } })
            .expect("a qualified widget path should parse");
        expect_that!(fields(&spec)[0].1, contains_substring("MarkdownWidget"));
    }

    #[gtest]
    fn a_custom_widget_expands_to_a_non_capturing_closure() {
        // Pins the whole contract: the widget is placed INSIDE rsx! (so it gets a
        // component scope and may use hooks) rather than called, the closure
        // captures nothing (so it coerces to `fn(ControlProps) -> Element`), and
        // the readable name rides along because Debug on a fn pointer is an
        // address.
        let spec = parse(quote! { Source { notes => { control: custom(MarkdownWidget) } } }).unwrap();
        let tokens = control_of(&spec).path().to_string();
        expect_that!(tokens, contains_substring("ControlType :: Custom"));
        expect_that!(tokens, contains_substring("name : \"MarkdownWidget\""));
        expect_that!(tokens, contains_substring("render : | __p |"));
        expect_that!(tokens, contains_substring("rsx !"));
        expect_that!(tokens, contains_substring("values : __p . values"));
    }

    #[gtest]
    fn a_qualified_custom_widget_keeps_only_the_last_segment_as_its_name() {
        let spec =
            parse(quote! { Source { notes => { control: custom(a::b::MarkdownWidget) } } }).unwrap();
        expect_that!(
            control_of(&spec).path().to_string(),
            contains_substring("name : \"MarkdownWidget\"")
        );
    }

    #[gtest]
    fn custom_without_a_widget_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes => { control: custom } } }),
            contains_substring("custom(MyWidget)")
        );
    }

    #[gtest]
    fn custom_with_two_widgets_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes => { control: custom(A, B) } } }),
            contains_substring("one widget")
        );
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
            witness_of(quote! { LoginForm { password => { control: password } } }),
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
            witness_of(quote! { Quiz { answers[] => { control: textarea } } }),
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
            witness_of(quote! { Grid { rows[].cells[] => { control: textarea } } }),
            eq(
                "for __row0 in __s . rows . iter () \
                 { for __row1 in __row0 . cells . iter () { let _ = & __row1 ; } }"
            )
        );
    }

    #[gtest]
    fn a_field_below_a_nested_row_keeps_walking() {
        expect_that!(
            witness_of(quote! { Grid { rows[].cells[].text => { label: "Text" } } }),
            eq(
                "for __row0 in __s . rows . iter () \
                 { for __row1 in __row0 . cells . iter () { let _ = & __row1 . text ; } }"
            )
        );
    }

    #[gtest]
    fn the_expansion_carries_one_witness_per_field() {
        // Two fields, two statements, inside a single never-called fn.
        let spec = parse(quote! {
            ChangePasswordForm {
                title: "Change Password",
                current_password => { control: password },
                new_password => { control: password, label: "New password" },
            }
        })
        .unwrap();
        let out = spec.expand().to_string();
        expect_that!(out, contains_substring("fn __paths_exist (__s : & ChangePasswordForm)"));
        expect_that!(out, contains_substring("let _ = & __s . current_password ;"));
        expect_that!(out, contains_substring("let _ = & __s . new_password ;"));
        // And the builder chain is still there beside it.
        expect_that!(out, contains_substring("with_title"));
        expect_that!(out, contains_substring("with_custom_control"));
    }

    #[gtest]
    fn a_spec_with_no_fields_has_an_empty_witness_body() {
        let out = parse(quote! { Article { title: "A" } }).unwrap().expand().to_string();
        expect_that!(out, contains_substring("fn __paths_exist (__s : & Article) { }"));
    }

}
