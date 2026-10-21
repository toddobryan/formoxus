//! `widget: <name>` and `widget: path::To::MyWidget { prop: value }` — the
//! author-facing widget vocabulary.

use formoxus_spec::{BuiltinWidget, RenderedInputType};
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

// The author-facing widget vocabulary is `BuiltinWidget::form_name`, in
// `formoxus-spec`: flat and lowercase, deliberately. `Input(Password)` is the
// shape of formoxus's own enum, grouping the `<input type=X>` family under one
// variant for dispatch, not a concept an author has. HTML spells it
// `type="password"` and so does `form!`. That keeps the names a stable surface:
// the enums under them can be regrouped or renamed without touching a single
// `form!` call. Names follow HTML where HTML has one (`tel`, `datetime_local`
// for `datetime-local`, since a hyphen cannot be an ident) and formoxus where
// it does not (`textarea`, `select`, `radio_group`).

/// Every name `form!` accepts after `widget:`, for the "unknown widget"
/// message, in `BuiltinWidget::ALL`'s order.
fn widget_names() -> impl Iterator<Item = &'static str> {
    BuiltinWidget::ALL.iter().map(BuiltinWidget::form_name)
}

/// `widget`, as the expression `::formoxus::attrs::BuiltinWidget::…`.
///
/// The variant idents come from `Debug`, which for a unit variant is exactly
/// its name; `formoxus-spec` tests the same about `RenderedInputType`.
pub(crate) fn builtin_tokens(widget: BuiltinWidget) -> TokenStream2 {
    match widget {
        BuiltinWidget::Input(input_type) => {
            let input_type = rendered_input_ident(input_type);
            quote! {
                ::formoxus::attrs::BuiltinWidget::Input(
                    ::formoxus::attrs::RenderedInputType::#input_type
                )
            }
        }
        other => {
            let variant = format_ident!("{other:?}");
            quote! { ::formoxus::attrs::BuiltinWidget::#variant }
        }
    }
}

/// `widget`, as the type implementing `::formoxus::widgets::Widget` that
/// renders it. Exhaustive, with no wildcard, so a new built-in cannot compile
/// without a row here.
fn builtin_type(widget: BuiltinWidget) -> TokenStream2 {
    use RenderedInputType as T;
    let name = match widget {
        BuiltinWidget::Input(input_type) => match input_type {
            T::Text => "TextInput",
            T::Password => "PasswordInput",
            T::Hidden => "HiddenInput",
            T::Number => "NumberInput",
            T::Email => "EmailInput",
            T::Telephone => "TelInput",
            T::Url => "UrlInput",
            T::Search => "SearchInput",
            T::Color => "ColorInput",
            T::Date => "DateInput",
            T::Time => "TimeInput",
            T::DatetimeLocal => "DatetimeLocalInput",
            T::Month => "MonthInput",
            T::Week => "WeekInput",
            T::Checkbox => "CheckboxInput",
        },
        BuiltinWidget::Textarea => "Textarea",
        BuiltinWidget::Select => "Select",
        BuiltinWidget::RadioGroup => "RadioGroup",
        BuiltinWidget::SelectMultiple | BuiltinWidget::CheckboxMultiple | BuiltinWidget::File => {
            unreachable!("parse rejects widgets that render nothing")
        }
    };
    let name = format_ident!("{name}");
    quote! { ::formoxus::widgets::#name }
}

fn rendered_input_ident(input_type: RenderedInputType) -> Ident {
    format_ident!("{input_type:?}")
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
    Named { name: Ident, widget: BuiltinWidget },
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
/// `accepts_optional` is `false`. A `select` can say "no answer", with its
/// blank option, which is why it is the default for an `Option<bool>`.
fn optional_refusal(widget: BuiltinWidget) -> &'static str {
    match widget {
        BuiltinWidget::RadioGroup => {
            "`radio_group` cannot render an optional field — a picked radio can't be un-picked; use `select`"
        }
        BuiltinWidget::Input(RenderedInputType::Checkbox) => {
            "`checkbox` cannot render an optional field — it has two states and an `Option<bool>` has three; use `select`"
        }
        other => unreachable!("`{other:?}` accepts an optional field; give it a refusal here"),
    }
}

/// What a failed `renders` check says, by the rule `BuiltinWidget::renders`
/// applied.
fn cannot_render(widget: BuiltinWidget) -> &'static str {
    match widget {
        BuiltinWidget::Input(RenderedInputType::Checkbox) => {
            "`checkbox` can only render a bool field"
        }
        BuiltinWidget::Input(_) => {
            "an `<input>` widget cannot render a bool field; use `checkbox` or `select`"
        }
        BuiltinWidget::Textarea => "`textarea` can only render a String field",
        BuiltinWidget::Select | BuiltinWidget::RadioGroup => {
            "this widget needs `choices` unless the field is a bool"
        }
        BuiltinWidget::SelectMultiple | BuiltinWidget::CheckboxMultiple | BuiltinWidget::File => {
            unreachable!("parse rejects widgets that render nothing")
        }
    }
}

impl WidgetRef {
    /// Free `const _` assertions that the field at `shape` can be rendered by
    /// this widget, with the caret on the widget's name. `shape` is a
    /// `field_kind::shape_of(…)` expression; see `FieldBody::type_checks`.
    pub(crate) fn checks(&self, shape: &TokenStream2, has_choices: bool) -> TokenStream2 {
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
        let WidgetKind::Named { widget, .. } = &self.kind else {
            return single;
        };
        let not_optional = if widget.accepts_optional() {
            TokenStream2::new()
        } else {
            let message = optional_refusal(*widget);
            quote_spanned! { span =>
                const _: () = ::core::assert!(
                    !::formoxus::field_kind::is_optional(#shape),
                    #message
                );
            }
        };
        let message = cannot_render(*widget);
        let builtin = builtin_tokens(*widget);
        quote_spanned! { span=>
            #single
            #not_optional
            const _: () = ::core::assert!(
                ::formoxus::field_kind::renders(#shape, #builtin, #has_choices),
                #message
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
        let Some(widget) = BuiltinWidget::from_form_name(&name.to_string()) else {
            return Err(syn::Error::new(name.span(), unknown_widget(&name, true)));
        };
        if !widget.is_rendered() {
            return Err(syn::Error::new(
                name.span(),
                format!(
                    "`{name}` is not supported yet — nothing renders it, so the field \
                     would silently vanish from the form"
                ),
            ));
        }
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
    if lowered != written && BuiltinWidget::from_form_name(&lowered).is_some() {
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
        // Generated from `BuiltinWidget::ALL`, so every widget's name is tried.
        // A widget that renders nothing yet must instead be refused, by name.
        // Distinct types, so a row copied without its name changed fails here.
        let mut types = HashSet::new();
        for widget in BuiltinWidget::ALL {
            let name = widget.form_name();
            let id = Ident::new(name, proc_macro2::Span::call_site());
            let parsed = parse(quote! { Source { f => { widget: #id } } });
            if !widget.is_rendered() {
                let err = parsed.err().map(|e| e.to_string()).unwrap_or_default();
                expect_that!(
                    err,
                    contains_substring("is not supported yet"),
                    "`{name}` renders nothing, so it should be refused"
                );
                continue;
            }
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

    /// `checks` calls these only for a widget some `form!` uses, so a widget
    /// added without a message would otherwise surface as a proc-macro panic
    /// in an author's build. Calls each wherever `checks` could.
    #[gtest]
    fn every_widget_checks_can_reach_has_a_message() {
        for &widget in BuiltinWidget::ALL.iter().filter(|w| w.is_rendered()) {
            expect_that!(cannot_render(widget), not(eq("")), "{widget:?}");
            if !widget.accepts_optional() {
                expect_that!(optional_refusal(widget), not(eq("")), "{widget:?}");
            }
        }
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

    #[gtest]
    fn a_non_chooser_is_told_which_widgets_take_choices() {
        let msg = parse(quote! { Address { state => { widget: text, choices: STATES } } })
            .expect_err("`text` has nothing to choose from")
            .to_string();
        expect_that!(msg, contains_substring("`text` takes no `choices`"));
        expect_that!(msg, contains_substring("`select`"));
        expect_that!(msg, contains_substring("`radio_group`"));
    }

    /// Either order: the check runs once both keys are known.
    #[gtest]
    fn the_widget_check_does_not_depend_on_the_order_written() {
        expect_that!(
            parse(quote! { Address { state => { choices: STATES, widget: text } } })
                .expect_err("`text` has nothing to choose from")
                .to_string(),
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
