//! `widget: <name>` and `widget: path::To::MyWidget { prop: value }` — the
//! author-facing widget vocabulary.

use formoxus_spec::WidgetName;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Expr, ExprLit, ExprUnary, Ident, Lit, Path, Token, UnOp, braced,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token,
};

use super::suggest::{edit_distance, to_snake};

// The author-facing widget vocabulary is `WidgetName::form_name`, in
// `formoxus-spec`: flat and lowercase, deliberately. A dev writes `password`,
// as HTML writes `type="password"`, never the type `PasswordInput` the name
// stands for. That keeps the names a stable surface: the types under them can
// be regrouped or renamed without touching a single `form!` call. Names follow HTML where HTML has one (`tel`, `datetime_local`
// for `datetime-local`, since a hyphen cannot be an ident) and formoxus where
// it does not (`textarea`, `select`, `radio_group`).

/// Every name `form!` knows after `widget:`, for the "unknown widget"
/// message: the rendered widgets in `WidgetName::ALL`'s order, then the
/// not-yet ones, so a near miss of `file` is still told it is coming.
fn widget_names() -> impl Iterator<Item = &'static str> {
    WidgetName::ALL
        .iter()
        .map(|widget_name| widget_name.form_name())
        .chain(WidgetName::NOT_YET.iter().copied())
}

/// `widget`, as the path of the type implementing `::formoxus::widgets::Widget`
/// that renders it: `WidgetName::MODULE`, then its `type_name`. A row in
/// `WidgetName` is all a new built-in needs here; `formoxus` tests that each
/// row names the right type.
pub(crate) fn builtin_type(widget: WidgetName) -> TokenStream2 {
    let module = WidgetName::MODULE
        .iter()
        .map(|segment| format_ident!("{segment}"));
    let name = format_ident!("{}", widget.type_name());
    quote! { #( ::#module )* :: #name }
}

#[derive(Debug)]
pub(crate) struct WidgetRef {
    pub(crate) kind: WidgetKind,
    pub(crate) props: Punctuated<syn::FieldValue, Token![,]>,
}

#[derive(Debug)]
pub(crate) enum WidgetKind {
    /// A built-in widget, by the name the author wrote (kept for its span and
    /// for messages) and what it resolved to. Never a widget that renders
    /// nothing yet: `parse` refuses those.
    Named { name: Ident, widget: WidgetName },
    /// `MarkdownWidget` — a `Path`, not an `Ident`, so that
    /// `inputs::MarkdownWidget` works without importing the input.
    Path(Path),
}

impl WidgetRef {
    /// The widget's type: a built-in name through `builtin_type`, a path as
    /// written.
    pub(crate) fn type_tokens(&self) -> TokenStream2 {
        match &self.kind {
            WidgetKind::Named { widget, .. } => builtin_type(*widget),
            WidgetKind::Path(path) => quote! { #path },
        }
    }

    /// The widget as the dev wrote it, for messages: the lowercase name, or
    /// a path's last segment.
    pub(crate) fn written(&self) -> String {
        match &self.kind {
            WidgetKind::Named { name, .. } => name.to_string(),
            WidgetKind::Path(path) => path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default(),
        }
    }

    /// The widget's value: a struct expression of its type, with the props
    /// written in its braces and `Default` for the rest.
    ///
    /// With no props, `Default::default()` alone rather than `#ty {
    /// ..Default::default() }` with nothing before the `..`, which trips
    /// clippy's `needless_update` in the dev's crate.
    pub(crate) fn expr(&self) -> TokenStream2 {
        let ty = self.type_tokens();
        if self.props.is_empty() {
            return quote! { <#ty as ::core::default::Default>::default() };
        }
        let props = self.props.iter().map(|prop| {
            let attrs = &prop.attrs;
            let member = &prop.member;
            let value = prop_value(&prop.expr);
            quote! { #(#attrs)* #member: #value }
        });
        quote! { #ty { #(#props,)* ..::core::default::Default::default() } }
    }
}

/// One prop's value as the struct expression gets it. A bare number literal
/// goes in as written, so it takes the prop's type the way it would in
/// hand-written Rust; `Into::into` would leave it an `i32`. Anything else goes
/// through `Into::into`, so `placeholder: "Name"` fills a `String`. Fully
/// qualified rather than `.into()` so precedence cannot bite (`a + b` then
/// `.into()` would convert `b` alone), and spanned at the value so a wrong
/// type points there.
fn prop_value(expr: &Expr) -> TokenStream2 {
    if is_number_literal(expr) {
        quote! { #expr }
    } else {
        quote_spanned! { expr.span()=> ::core::convert::Into::into(#expr) }
    }
}

/// `5`, `2.5`, and either negated.
fn is_number_literal(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(ExprLit {
            lit: Lit::Int(_) | Lit::Float(_),
            ..
        }) => true,
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr,
            ..
        }) => is_number_literal(expr),
        _ => false,
    }
}

/// Why `widget` cannot render an optional field, for the widgets whose
/// `ACCEPTS_OPTIONAL` is `false`. A `select` can say "no answer", with its
/// blank option, which is why it is the default for an `Option<bool>`.
fn optional_refusal(widget_name: WidgetName) -> &'static str {
    match widget_name {
        WidgetName::RadioGroup => {
            "`radio_group` cannot render an optional field — a picked radio can't be un-picked; use `select`"
        }
        WidgetName::CheckboxInput => {
            "`checkbox` cannot render an optional field — it has two states and an `Option<bool>` has three; use `select`"
        }
        // Asserted for every widget, so one that accepts an optional field
        // needs a message too, though its assert always holds. Listed rather
        // than `_`, so a new widget has to say which kind it is.
        WidgetName::TextInput
        | WidgetName::PasswordInput
        | WidgetName::HiddenInput
        | WidgetName::NumberInput
        | WidgetName::EmailInput
        | WidgetName::TelInput
        | WidgetName::UrlInput
        | WidgetName::SearchInput
        | WidgetName::ColorInput
        | WidgetName::DateInput
        | WidgetName::TimeInput
        | WidgetName::DatetimeLocalInput
        | WidgetName::MonthInput
        | WidgetName::WeekInput
        | WidgetName::Textarea
        | WidgetName::Select => "this widget cannot render an optional field",
    }
}

/// What a failed `renders` check says, by the widget's `ACCEPTS` and
/// `TAKES_CHOICES`. Every widget listed, so a new one needs a message.
/// Whether a built-in takes `choices`, for the message naming the ones that
/// do. `W::TAKES_CHOICES` is the rule rustc checks; this is only wording, so
/// it lives with the other messages. Every widget listed, so a new one has to
/// say.
fn takes_choices(widget_name: WidgetName) -> bool {
    match widget_name {
        WidgetName::Select | WidgetName::RadioGroup => true,
        WidgetName::TextInput
        | WidgetName::PasswordInput
        | WidgetName::HiddenInput
        | WidgetName::NumberInput
        | WidgetName::EmailInput
        | WidgetName::TelInput
        | WidgetName::UrlInput
        | WidgetName::SearchInput
        | WidgetName::ColorInput
        | WidgetName::DateInput
        | WidgetName::TimeInput
        | WidgetName::DatetimeLocalInput
        | WidgetName::MonthInput
        | WidgetName::WeekInput
        | WidgetName::CheckboxInput
        | WidgetName::Textarea => false,
    }
}

fn cannot_render(widget_name: WidgetName) -> &'static str {
    match widget_name {
        WidgetName::CheckboxInput => "`checkbox` can only render a bool field",
        WidgetName::Textarea => "`textarea` can only render a String field",
        WidgetName::Select | WidgetName::RadioGroup => {
            "this widget needs `choices` unless the field is a bool"
        }
        WidgetName::TextInput
        | WidgetName::PasswordInput
        | WidgetName::HiddenInput
        | WidgetName::NumberInput
        | WidgetName::EmailInput
        | WidgetName::TelInput
        | WidgetName::UrlInput
        | WidgetName::SearchInput
        | WidgetName::ColorInput
        | WidgetName::DateInput
        | WidgetName::TimeInput
        | WidgetName::DatetimeLocalInput
        | WidgetName::MonthInput
        | WidgetName::WeekInput => {
            "an `<input>` widget cannot render a bool field; use `checkbox` or `select`"
        }
    }
}

impl WidgetRef {
    /// A free `const _` assertion that this widget takes the `choices` written
    /// at `key`, with the caret on the key: `W::TAKES_CHOICES`, read by rustc
    /// in the dev's crate, so a dev's widget that sets it to `false` is
    /// refused like `text` is. A built-in's message names the widgets that do
    /// take them.
    pub(crate) fn choices_check(&self, key: &Ident) -> TokenStream2 {
        let ty = self.type_tokens();
        let message = match &self.kind {
            WidgetKind::Named { name, .. } => format!(
                "`{name}` takes no `choices` — they apply to {}",
                WidgetName::ALL
                    .iter()
                    .filter(|w| takes_choices(**w))
                    .map(|w| format!("`{}`", w.form_name()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            WidgetKind::Path(_) => format!("`{}` takes no `choices`", self.written()),
        };
        quote_spanned! { key.span()=>
            #[allow(clippy::assertions_on_constants)]
            const _: () = ::core::assert!(
                <#ty as ::formoxus::widgets::Widget>::TAKES_CHOICES,
                #message
            );
        }
    }

    /// Free `const _` assertions that the field at `shape` can be rendered by
    /// this widget, with the caret on the widget's name. `shape` is a
    /// `field_kind::shape_of(…)` expression; see `FieldBody::type_checks`.
    pub(crate) fn checks(&self, shape: &TokenStream2, has_choices: bool) -> TokenStream2 {
        let ty = self.type_tokens();
        let span = match &self.kind {
            WidgetKind::Named { name, .. } => name.span(),
            WidgetKind::Path(path) => path
                .segments
                .last()
                .map_or_else(proc_macro2::Span::call_site, |s| s.ident.span()),
        };
        let single = quote_spanned! { span =>
            const _: () = ::core::assert!(
                ::formoxus::field_kind::is_single_value(#shape),
                "a widget applies only to a single-value field, not a struct, list or enum"
            );
        };
        // The same asserts for a built-in and a dev's widget: rustc reads
        // `W`'s consts in the dev's crate. Only the wording differs, since a
        // built-in's says why and what to use instead.
        let (cannot_render, optional_refusal) = match &self.kind {
            WidgetKind::Named {
                widget: widget_name,
                ..
            } => (
                cannot_render(*widget_name).to_string(),
                optional_refusal(*widget_name).to_string(),
            ),
            WidgetKind::Path(_) => {
                let written = self.written();
                (
                    format!("`{written}` cannot render this field"),
                    format!("`{written}` cannot render an optional field"),
                )
            }
        };
        quote_spanned! { span=>
            #single
            const _: () = ::core::assert!(
                ::formoxus::field_kind::optional_is_okay::<#ty>(#shape),
                #optional_refusal
            );
            const _: () = ::core::assert!(
                ::formoxus::field_kind::renders::<#ty>(#shape, #has_choices),
                #cannot_render
            );
        }
    }
}

impl Parse for WidgetRef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let kind = input.parse::<WidgetKind>()?;
        let mut props = Punctuated::new();
        if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            props = Punctuated::parse_terminated(&content)?;
        }
        Ok(Self { kind, props })
    }
}

impl Parse for WidgetKind {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let is_name = input.peek(Ident)
            && !input.peek2(Token![::])
            && input
                .cursor()
                .ident()
                .is_some_and(|(id, _)| id.to_string().starts_with(char::is_lowercase));
        if !is_name {
            return Ok(Self::Path(Path::parse_mod_style(input)?));
        }

        let name: Ident = input.parse()?;
        let written = name.to_string();
        if WidgetName::NOT_YET.contains(&written.as_str()) {
            return Err(syn::Error::new(
                name.span(),
                format!(
                    "`{name}` is not supported yet — nothing renders it, so the field \
                     would silently vanish from the form"
                ),
            ));
        }
        let Some(widget) = WidgetName::from_form_name(&written) else {
            return Err(syn::Error::new(name.span(), unknown_widget(&name, true)));
        };
        Ok(Self::Named { name, widget })
    }
}

/// The message for a name that is not in the table.
///
/// Three cases, in order of how likely the author is to have meant them: a name
/// that IS in the table under a different case (`datetimeLocal`, or in a
/// `classes_plus:` selector `Password`), a near miss, and no idea.
///
/// `path_ok` is whether a path to the dev's own widget type is an option where
/// the name was written: after `widget:` it is, in a `classes_plus:` selector it
/// is not (a custom widget has no class slots).
pub(crate) fn unknown_widget(name: &Ident, path_ok: bool) -> String {
    let written = name.to_string();
    let lowered = to_snake(&written);
    if lowered != written && widget_names().any(|n| n == lowered) {
        return format!(
            "unknown widget `{written}` — widget names are lowercase, write `{lowered}`"
        );
    }
    match widget_names()
        .filter(|n| edit_distance(&lowered, n) <= 2)
        .min_by_key(|n| edit_distance(&lowered, n))
    {
        Some(near) => format!("unknown widget `{written}` — did you mean `{near}`?"),
        None => format!(
            "unknown widget `{written}` — expected one of {}{}",
            widget_names().collect::<Vec<_>>().join(", "),
            if path_ok {
                ", or a path to your own widget type"
            } else {
                ""
            }
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::tests::{err_of, fields, parse};
    use crate::form::{Entry, FormSpecInput};
    use googletest::prelude::*;
    use quote::quote;
    use std::collections::HashSet;

    // ── The widget vocabulary ───────────────────────────────────────────

    /// The `WidgetRef` of the first field entry.
    fn widget_of(spec: &FormSpecInput) -> &WidgetRef {
        spec.entries
            .iter()
            .find_map(|e| match e {
                Entry::Field { body, .. } => body.widget.as_ref(),
                _ => None,
            })
            .expect("a field with a widget")
    }

    /// The widget type `widget: <written>` resolves to.
    fn type_of(written: &proc_macro2::TokenStream) -> String {
        let spec = parse(quote! { Source { f => { widget: #written } } })
            .unwrap_or_else(|e| panic!("`{written}` should parse: {e}"));
        widget_of(&spec).type_tokens().to_string()
    }

    #[gtest]
    fn every_name_in_the_table_parses_and_resolves_to_its_own_type() {
        // Generated from `WidgetName::ALL`, so every widget's name is tried.
        // Distinct types, so a row copied without its name changed fails here.
        let mut types = HashSet::new();
        for widget in WidgetName::ALL {
            let name = widget.form_name();
            let id = Ident::new(name, proc_macro2::Span::call_site());
            let parsed = parse(quote! { Source { f => { widget: #id } } });
            let spec = parsed.unwrap_or_else(|e| panic!("`{name}` should parse: {e}"));
            let tokens = widget_of(&spec).type_tokens().to_string();
            expect_that!(
                &tokens,
                starts_with(":: formoxus :: widgets ::"),
                "for widget `{name}`"
            );
            expect_that!(
                types.insert(tokens.clone()),
                eq(true),
                "`{name}` resolves to `{tokens}`, as an earlier name did"
            );
        }
    }

    /// A widget that renders nothing yet is refused by name, not as unknown.
    #[gtest]
    fn every_not_yet_name_is_refused_as_not_supported_yet() {
        for name in WidgetName::NOT_YET {
            let id = Ident::new(name, proc_macro2::Span::call_site());
            expect_that!(
                err_of(quote! { Source { f => { widget: #id } } }),
                contains_substring(format!("`{name}` is not supported yet")),
                "{name}"
            );
        }
    }

    #[gtest]
    fn a_name_resolves_to_its_widget_type() {
        // The whole point of the flat vocabulary: `password` on the outside,
        // `PasswordInput` on the inside, and the author never sees the type.
        expect_that!(
            type_of(&quote!(password)),
            eq(":: formoxus :: widgets :: PasswordInput")
        );
        expect_that!(
            type_of(&quote!(textarea)),
            eq(":: formoxus :: widgets :: Textarea")
        );
    }

    #[gtest]
    fn an_html_spelling_wins_over_the_enums() {
        // `tel` is HTML's name; the variant is `Telephone`. The divergence is the
        // decoupling working — an author writes what HTML calls it.
        expect_that!(type_of(&quote!(tel)), ends_with(":: TelInput"));
    }

    #[gtest]
    fn a_hyphenated_html_name_is_snake_cased() {
        expect_that!(
            type_of(&quote!(datetime_local)),
            ends_with(":: DatetimeLocalInput")
        );
    }

    #[gtest]
    fn a_near_miss_is_suggested() {
        let msg = err_of(quote! { Source { p => { widget: passwrod } } });
        expect_that!(msg, contains_substring("passwrod"));
        expect_that!(msg, contains_substring("did you mean `password`"));
    }

    /// A lowercase name in the wrong case is still named as a case error. A
    /// capitalized one never reaches the table: it is a path.
    #[gtest]
    fn a_camel_cased_name_is_named_as_a_case_error() {
        expect_that!(
            err_of(quote! { Source { p => { widget: datetimeLocal } } }),
            contains_substring("widget names are lowercase, write `datetime_local`")
        );
    }

    #[gtest]
    fn a_nonsense_widget_lists_the_vocabulary() {
        let msg = err_of(quote! { Source { p => { widget: fluorescent } } });
        expect_that!(msg, contains_substring("textarea"));
        expect_that!(msg, contains_substring("or a path to your own widget type"));
    }

    #[gtest]
    fn number_is_selectable_but_spelled_only_one_way() {
        // `number` is in the vocabulary — it is a real HTML type, and someone may
        // want the spinner. What is NOT there is `integer`/`float`: those were
        // `InputType` variant names, never HTML ones, and two spellings for one
        // attribute is the drift this table exists to prevent.
        expect_that!(type_of(&quote!(number)), ends_with(":: NumberInput"));
        expect_that!(
            err_of(quote! { Q { n => { widget: integer } } }),
            contains_substring("unknown widget `integer`")
        );
    }

    #[gtest]
    fn the_old_two_level_spelling_is_rejected() {
        // `Input(Password)` was the spelling before the table existed. `Input`
        // now parses as a path, and the parenthesis after it is a syntax error.
        expect_that!(
            err_of(quote! { Source { p => { widget: Input(Password) } } }).len(),
            gt(0)
        );
    }

    /// The message functions match on `WidgetName`, every variant listed, so
    /// a new widget cannot compile without one. What the compiler cannot see
    /// is an arm on the WRONG widget: these pin the ones with their own text.
    #[gtest]
    fn each_widget_with_its_own_message_gets_it() {
        expect_that!(
            optional_refusal(WidgetName::CheckboxInput),
            starts_with("`checkbox` cannot render an optional field")
        );
        expect_that!(
            optional_refusal(WidgetName::RadioGroup),
            starts_with("`radio_group` cannot render an optional field")
        );
        expect_that!(
            cannot_render(WidgetName::CheckboxInput),
            eq("`checkbox` can only render a bool field")
        );
        expect_that!(
            cannot_render(WidgetName::Textarea),
            eq("`textarea` can only render a String field")
        );
        expect_that!(
            cannot_render(WidgetName::TelInput),
            starts_with("an `<input>` widget")
        );
        let choosers: Vec<WidgetName> = WidgetName::ALL
            .iter()
            .copied()
            .filter(|w| takes_choices(*w))
            .collect();
        expect_that!(
            choosers,
            elements_are![eq(&WidgetName::Select), eq(&WidgetName::RadioGroup)]
        );
    }

    // ── `choices:`, beside `widget:` ─────────────────────────────────────

    #[gtest]
    fn a_chooser_takes_choices_beside_it() {
        let spec = parse(quote! { Address { state => { widget: select, choices: STATES } } })
            .expect("`select` accepts choices");
        expect_that!(fields(&spec)[0].1, eq("select"));
    }

    /// Choices make the field a `ChoiceField`, through `choice_field_for`, and
    /// the widget override still lands beside it.
    #[gtest]
    fn choices_reach_the_expansion_as_a_choice_field() {
        let spec =
            parse(quote! { Address { state => { widget: select, choices: STATES } } }).unwrap();
        let tokens = spec.expand().to_string();
        expect_that!(
            tokens,
            contains_substring(":: formoxus :: field_kind :: choice_field_for")
        );
        expect_that!(tokens, contains_substring("STATES"));
        expect_that!(tokens, contains_substring("with_widget"));
        expect_that!(tokens, not(contains_substring("field_kind :: field_for")));
    }

    /// No `widget:` is fine: a choice field defaults to a select at runtime.
    #[gtest]
    fn choices_need_no_widget() {
        let spec = parse(quote! { Address { state => { choices: STATES } } }).unwrap();
        let tokens = spec.expand().to_string();
        expect_that!(tokens, contains_substring("choice_field_for"));
        expect_that!(tokens, not(contains_substring("with_widget")));
    }

    #[gtest]
    fn an_arbitrary_expression_is_accepted_as_a_list() {
        // The macro never inspects it; `choice_field_for` types it.
        let spec = parse(quote! { Address { state => { choices: states() } } }).unwrap();
        expect_that!(spec.expand().to_string(), contains_substring("states ()"));
    }

    /// Not a parse error: `form!` emits a `const _` on the widget's
    /// `TAKES_CHOICES`, which rustc evaluates in the dev's crate. The goldens
    /// (`form_choices_on_a_non_chooser`) pin what the dev sees.
    #[gtest]
    fn a_non_chooser_is_told_which_widgets_take_choices() {
        let spec =
            parse(quote! { Address { state => { widget: text, choices: STATES } } }).unwrap();
        let tokens = spec.expand().to_string();
        expect_that!(
            tokens,
            contains_substring(
                "< :: formoxus :: widgets :: TextInput as :: formoxus :: widgets :: Widget > :: TAKES_CHOICES"
            )
        );
        expect_that!(
            tokens,
            contains_substring("`text` takes no `choices` — they apply to `select`, `radio_group`")
        );
    }

    /// Either order: the check is emitted once both keys are known.
    #[gtest]
    fn the_widget_check_does_not_depend_on_the_order_written() {
        let spec =
            parse(quote! { Address { state => { choices: STATES, widget: text } } }).unwrap();
        expect_that!(
            spec.expand().to_string(),
            contains_substring("`text` takes no `choices`")
        );
    }

    /// Choices reach every widget through `WidgetProps`, so a dev's own widget
    /// may take them.
    #[gtest]
    fn a_path_widget_takes_choices() {
        let spec = parse(quote! { Source { notes => { widget: Picker, choices: STATES } } })
            .expect("a dev's widget receives choices like any other");
        expect_that!(
            spec.expand().to_string(),
            contains_substring("choice_field_for")
        );
    }

    /// `choices` is a legal key, so a near miss is pointed at it.
    #[gtest]
    fn a_misspelled_choices_is_suggested() {
        expect_that!(
            parse(quote! { Address { state => { choice: STATES } } })
                .expect_err("`choice` is not a key")
                .to_string(),
            contains_substring("Did you mean `choices`?")
        );
    }

    #[gtest]
    fn choices_given_twice_are_rejected() {
        expect_that!(
            parse(quote! { Address { state => { choices: A, choices: B } } })
                .expect_err("the second silently winning would be worse")
                .to_string(),
            contains_substring("`choices` is given twice")
        );
    }

    /// Decision B: choices fix the values, so a validated key beside them is
    /// refused rather than quietly ignored.
    #[gtest]
    fn choices_refuse_a_validated_key_beside_them() {
        expect_that!(
            parse(quote! { Address { state => { choices: STATES, max_length: 2 } } })
                .expect_err("the choices fix the values")
                .to_string(),
            contains_substring("`max_length`")
        );
    }

    #[gtest]
    fn a_widget_without_choices_makes_no_choice_field() {
        let spec = parse(quote! { Address { state => { widget: select } } }).unwrap();
        expect_that!(fields(&spec)[0].1, eq("select"));
        expect_that!(
            spec.expand().to_string(),
            not(contains_substring("choice_field_for"))
        );
    }

    // ── A path to the dev's own widget ───────────────────────────────────

    #[gtest]
    fn a_capitalized_name_is_a_path() {
        // Even one spelled like a built-in's variant: `Password` is a type the
        // dev must have in scope, and rustc's "cannot find type" says so.
        expect_that!(type_of(&quote!(MarkdownWidget)), eq("MarkdownWidget"));
        expect_that!(type_of(&quote!(Password)), eq("Password"));
    }

    #[gtest]
    fn a_path_may_be_module_qualified() {
        // The reason it is a `Path` and not an `Ident`: naming a widget should not
        // require importing it. Lowercase throughout is still a path: the `::`
        // after the first segment is what says so.
        expect_that!(
            type_of(&quote!(widgets::MarkdownWidget)),
            eq("widgets :: MarkdownWidget")
        );
        expect_that!(
            type_of(&quote!(widgets::markdown)),
            eq("widgets :: markdown")
        );
    }

    #[gtest]
    fn a_path_from_a_keyword_or_the_root_is_a_path() {
        expect_that!(type_of(&quote!(crate::Markdown)), eq("crate :: Markdown"));
        expect_that!(type_of(&quote!(self::Markdown)), eq("self :: Markdown"));
        expect_that!(type_of(&quote!(::ui::Markdown)), eq(":: ui :: Markdown"));
    }

    #[gtest]
    fn a_path_widget_is_the_last_segment_in_messages() {
        let spec = parse(quote! { Source { notes => { widget: a::b::MarkdownWidget } } }).unwrap();
        expect_that!(widget_of(&spec).written(), eq("MarkdownWidget"));
        let spec = parse(quote! { Source { notes => { widget: textarea } } }).unwrap();
        expect_that!(widget_of(&spec).written(), eq("textarea"));
    }

    // ── Props in the widget's braces ─────────────────────────────────────

    /// The widget's value, `WidgetRef::expr`, for `widget: <written>`.
    fn expr_of(written: &proc_macro2::TokenStream) -> String {
        let spec = parse(quote! { Source { f => { widget: #written } } })
            .unwrap_or_else(|e| panic!("`{written}` should parse: {e}"));
        widget_of(&spec).expr().to_string()
    }

    /// Not `Textarea { ..Default::default() }`, which is clippy's
    /// `needless_update` in the dev's crate.
    #[gtest]
    fn no_props_is_the_default_alone() {
        expect_that!(
            expr_of(&quote!(textarea)),
            eq(
                "< :: formoxus :: widgets :: Textarea as :: core :: default :: Default > :: default ()"
            )
        );
        expect_that!(
            expr_of(&quote!(MarkdownWidget)),
            eq("< MarkdownWidget as :: core :: default :: Default > :: default ()")
        );
    }

    #[gtest]
    fn empty_braces_are_no_props() {
        expect_that!(
            expr_of(&quote!(textarea {})),
            eq(&expr_of(&quote!(textarea)))
        );
    }

    #[gtest]
    fn props_fill_a_struct_expression_and_default_the_rest() {
        let expr = expr_of(&quote!(MarkdownWidget { rows: 5 }));
        expect_that!(
            expr,
            eq("MarkdownWidget { rows : 5 , .. :: core :: default :: Default :: default () }")
        );
    }

    /// A number literal takes the prop's type, as in hand-written Rust;
    /// through `Into::into` it would be an `i32`.
    #[gtest]
    fn a_number_literal_goes_in_as_written() {
        let expr = expr_of(&quote!(MarkdownWidget {
            rows: 5,
            scale: 2.5,
            offset: -3
        }));
        expect_that!(expr, contains_substring("rows : 5 ,"));
        expect_that!(expr, contains_substring("scale : 2.5 ,"));
        expect_that!(expr, contains_substring("offset : - 3 ,"));
    }

    #[gtest]
    fn anything_else_goes_through_into() {
        let expr = expr_of(&quote!(MarkdownWidget {
            hint: "Notes",
            rows: ROWS,
            wrap: a + b
        }));
        expect_that!(
            expr,
            contains_substring("hint : :: core :: convert :: Into :: into (\"Notes\")")
        );
        expect_that!(
            expr,
            contains_substring("rows : :: core :: convert :: Into :: into (ROWS)")
        );
        // The whole sum, not `b` alone, which `a + b.into()` would convert.
        expect_that!(
            expr,
            contains_substring("wrap : :: core :: convert :: Into :: into (a + b)")
        );
    }

    #[gtest]
    fn a_shorthand_prop_is_its_own_variable() {
        expect_that!(
            expr_of(&quote!(MarkdownWidget { rows })),
            contains_substring("rows : :: core :: convert :: Into :: into (rows)")
        );
    }

    #[gtest]
    fn the_dev_does_not_write_the_rest_themselves() {
        expect_that!(
            err_of(quote! { Source { f => { widget: MarkdownWidget { ..Default::default() } } } })
                .len(),
            gt(0)
        );
    }

    #[gtest]
    fn the_widget_reaches_the_builder_through_with_widget() {
        let spec =
            parse(quote! { Source { notes => { widget: MarkdownWidget { rows: 5 } } } }).unwrap();
        let tokens = spec.expand().to_string();
        expect_that!(
            tokens,
            contains_substring(". with_widget (\"notes\" , MarkdownWidget { rows : 5 ,")
        );
        expect_that!(tokens, not(contains_substring("with_custom_widget")));
    }
}
