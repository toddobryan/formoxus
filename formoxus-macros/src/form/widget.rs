//! `widget: <name>` and `widget: custom(…)` — the author-facing widget
//! vocabulary, and the arguments a widget accepts.

use formoxus_spec::{BuiltinWidget, RenderedInputType};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Expr, Ident, Path, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
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

/// `widget`, as the `::formoxus::widgets::WidgetType` it renders as.
fn widget_type_tokens(widget: BuiltinWidget) -> TokenStream2 {
    let builtin = builtin_tokens(widget);
    quote! { ::formoxus::widgets::WidgetType::Builtin(#builtin) }
}

fn rendered_input_ident(input_type: RenderedInputType) -> Ident {
    format_ident!("{input_type:?}")
}

mod widget_kw {
    syn::custom_keyword!(custom);
}

#[derive(Debug)]
pub(crate) struct WidgetRef {
    pub(crate) kind: WidgetKind,
    pub(crate) args: WidgetArgs,
}

#[derive(Debug)]
pub(crate) enum WidgetKind {
    /// A built-in widget, by the name the author wrote (kept for its span and
    /// for messages) and what it resolved to. Never a widget that renders
    /// nothing yet: `parse` refuses those.
    Named { name: Ident, widget: BuiltinWidget },
    /// `custom(MarkdownWidget)` — a `Path`, not an `Ident`, so that
    /// `custom(inputs::MarkdownWidget)` works without importing the input.
    Custom(Path),
}

/// What goes inside `widget: select { … }`.
///
/// Separate from the widget's identity because these are per-FIELD settings
/// that happen to be gated by widget — `FieldSpec` is where they land, not
/// `WidgetType`, which is the dispatch discriminant. Room here for the HTML
/// attribute keys (`rows`, `placeholder`, `class`) that will join `choices`.
#[derive(Debug, Default)]
pub(crate) struct WidgetArgs {
    pub(crate) choices: Option<Expr>,
}

impl WidgetRef {
    /// The `WidgetType` expression this names, fully qualified.
    pub(crate) fn path(&self) -> TokenStream2 {
        match &self.kind {
            WidgetKind::Named { widget, .. } => widget_type_tokens(*widget),
            // A NON-CAPTURING closure, which coerces to `fn(WidgetProps) ->
            // Element`. The input goes inside `rsx!` rather than being called,
            // so it gets a component scope of its own and may use hooks.
            //
            // The name is carried separately because `Debug` on a fn pointer
            // prints an address, and panic messages and test assertions want
            // "MarkdownWidget".
            WidgetKind::Custom(component) => {
                let name = last_segment_string(component);
                quote! {
                    ::formoxus::widgets::WidgetType::Custom {
                        name: #name,
                        render: |__p| ::dioxus::prelude::rsx! {
                            #component { values: __p.values, props: __p.props }
                        },
                    }
                }
            }
        }
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
    pub(crate) fn checks(&self, shape: &TokenStream2) -> TokenStream2 {
        let span = match &self.kind {
            WidgetKind::Named { name, .. } => name.span(),
            WidgetKind::Custom(path) => path
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
        let has_choices = self.args.choices.is_some();
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

/// The last segment of a path, as a string — `"MarkdownWidget"` for
/// `inputs::MarkdownWidget`.
fn last_segment_string(path: &Path) -> String {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

impl Parse for WidgetRef {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let kind = input.parse::<WidgetKind>()?;
        let args = if input.peek(syn::token::Brace) {
            parse_widget_args(input, &kind)?
        } else {
            WidgetArgs::default()
        };
        Ok(Self { kind, args })
    }
}

impl Parse for WidgetKind {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.peek(widget_kw::custom) {
            let kw: widget_kw::custom = input.parse()?;
            if !input.peek(syn::token::Paren) {
                return Err(syn::Error::new(
                    kw.span,
                    "`custom` needs the widget it renders — write `custom(MyWidget)`",
                ));
            }
            let inner;
            parenthesized!(inner in input);
            let component: Path = inner.parse()?;
            if !inner.is_empty() {
                return Err(inner.error("`custom` takes one component and nothing else"));
            }
            return Ok(Self::Custom(component));
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

/// `{ choices: STATES }` — the arguments one widget accepts.
///
/// Gated by widget rather than accepted everywhere, so `widget: text { choices:
/// … }` is a compile error naming the widgets that would have worked, instead of
/// a setting that is silently ignored at render.
fn parse_widget_args(input: ParseStream<'_>, kind: &WidgetKind) -> syn::Result<WidgetArgs> {
    let body;
    let braces = braced!(body in input);
    let mut args = WidgetArgs::default();

    while !body.is_empty() {
        let key: Ident = body.parse()?;
        let _colon: Token![:] = body.parse()?;
        match key.to_string().as_str() {
            "choices" if args.choices.is_some() => {
                return Err(syn::Error::new_spanned(&key, "duplicate `choices`"));
            }
            "choices" => {
                reject_choices_unless_chooser(&key, kind)?;
                args.choices = Some(body.parse()?);
            }
            other => {
                return Err(syn::Error::new_spanned(
                    &key,
                    format!("unknown widget argument `{other}` — expected `choices`"),
                ));
            }
        }
        if body.peek(Token![,]) {
            body.parse::<Token![,]>()?;
        }
    }

    if args.choices.is_none() {
        return Err(syn::Error::new(
            braces.span.join(),
            "empty widget arguments — drop the braces if there are none",
        ));
    }
    Ok(args)
}

fn reject_choices_unless_chooser(key: &Ident, kind: &WidgetKind) -> syn::Result<()> {
    match kind {
        WidgetKind::Named { widget, .. } if widget.takes_choices() => Ok(()),
        WidgetKind::Named { name, .. } => Err(syn::Error::new_spanned(
            key,
            format!(
                "`{name}` takes no `choices` — they apply to {}",
                BuiltinWidget::ALL
                    .iter()
                    .filter(|w| w.takes_choices() && w.is_rendered())
                    .map(|w| format!("`{}`", w.form_name()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        )),
        // A custom widget receives `(values, props)` and nothing else, so a list
        // handed to it here would go nowhere. Its own choices are its business.
        WidgetKind::Custom(_) => Err(syn::Error::new_spanned(
            key,
            "a custom widget supplies its own choices",
        )),
    }
}

/// The message for a name that is not in the table.
///
/// Three cases, in order of how likely the author is to have meant them: a name
/// that IS in the table under a different case (`Password`, `DatetimeLocal` — the
/// old `WidgetType`-shaped spelling), a near miss, and no idea.
///
/// `custom` is whether `custom(MyWidget)` is an option where the name was
/// written: after `widget:` it is, in a `classes_plus:` selector it is not (a
/// custom widget has no class slots).
pub(crate) fn unknown_widget(name: &Ident, custom: bool) -> String {
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
            if custom {
                ", or `custom(MyWidget)`"
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

    #[gtest]
    fn every_name_in_the_table_parses_and_resolves() {
        // Generated from `BuiltinWidget::ALL`, so every widget's name is tried.
        // A widget that renders nothing yet must instead be refused, by name.
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
            let tokens = widget_of(&spec).path().to_string();
            expect_that!(
                &tokens,
                contains_substring(":: formoxus :: widgets :: WidgetType ::"),
                "for widget `{name}`"
            );
        }
    }

    #[gtest]
    fn a_name_resolves_to_the_qualified_two_level_path() {
        // The whole point of the flat vocabulary: `password` on the outside,
        // `Input(Password)` on the inside, and the author never sees the split.
        let spec = parse(quote! { LoginForm { password => { widget: password } } }).unwrap();
        let tokens = widget_of(&spec).path().to_string();
        expect_that!(tokens, contains_substring("WidgetType :: Builtin"));
        expect_that!(tokens, contains_substring("BuiltinWidget :: Input"));
        expect_that!(tokens, contains_substring("InputType :: Password"));
    }

    #[gtest]
    fn a_non_input_name_resolves_to_a_bare_variant() {
        let spec = parse(quote! { Source { notes => { widget: textarea } } }).unwrap();
        let tokens = widget_of(&spec).path().to_string();
        expect_that!(tokens, contains_substring("BuiltinWidget :: Textarea"));
        expect_that!(tokens, not(contains_substring("InputType")));
    }

    #[gtest]
    fn an_html_spelling_wins_over_the_enums() {
        // `tel` is HTML's name; the variant is `Telephone`. The divergence is the
        // decoupling working — an author writes what HTML calls it.
        let spec = parse(quote! { Source { phone => { widget: tel } } }).unwrap();
        expect_that!(
            widget_of(&spec).path().to_string(),
            contains_substring("InputType :: Telephone")
        );
    }

    #[gtest]
    fn a_hyphenated_html_name_is_snake_cased() {
        let spec = parse(quote! { Source { at => { widget: datetime_local } } }).unwrap();
        expect_that!(
            widget_of(&spec).path().to_string(),
            contains_substring("InputType :: DatetimeLocal")
        );
    }

    #[gtest]
    fn a_near_miss_is_suggested() {
        let msg = err_of(quote! { Source { p => { widget: passwrod } } });
        expect_that!(msg, contains_substring("passwrod"));
        expect_that!(msg, contains_substring("did you mean `password`"));
    }

    #[gtest]
    fn the_enum_spelling_is_named_as_a_case_error() {
        // Someone reading `WidgetType` will try `Password` and `DatetimeLocal`.
        // Both are in the table under another case, so say so instead of guessing.
        expect_that!(
            err_of(quote! { Source { p => { widget: Password } } }),
            contains_substring("widget names are lowercase, write `password`")
        );
        expect_that!(
            err_of(quote! { Source { p => { widget: DatetimeLocal } } }),
            contains_substring("write `datetime_local`")
        );
    }

    #[gtest]
    fn a_nonsense_widget_lists_the_vocabulary() {
        let msg = err_of(quote! { Source { p => { widget: fluorescent } } });
        expect_that!(msg, contains_substring("textarea"));
        expect_that!(msg, contains_substring("custom(MyWidget)"));
    }

    #[gtest]
    fn number_is_selectable_but_spelled_only_one_way() {
        // `number` is in the vocabulary — it is a real HTML type, and someone may
        // want the spinner. What is NOT there is `integer`/`float`: those were
        // `InputType` variant names, never HTML ones, and two spellings for one
        // attribute is the drift this table exists to prevent.
        let spec = parse(quote! { Q { n => { widget: number } } }).unwrap();
        expect_that!(
            widget_of(&spec).path().to_string(),
            contains_substring("InputType :: Number")
        );
        expect_that!(
            err_of(quote! { Q { n => { widget: integer } } }),
            contains_substring("unknown widget `integer`")
        );
    }

    #[gtest]
    fn the_old_two_level_spelling_is_rejected() {
        // `Input(Password)` was the spelling before the table existed. It now
        // fails on `Input`, which is not a name — and `input` is not one either,
        // so the message falls through to the vocabulary list.
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

    // ── `widget: name { … }` ─────────────────────────────────────────────

    #[gtest]
    fn a_chooser_takes_a_choices_argument() {
        let spec = parse(quote! { Address { state => { widget: select { choices: STATES } } } })
            .expect("`select` accepts choices");
        expect_that!(fields(&spec)[0].1, eq("select"));
    }

    #[gtest]
    fn choices_reach_the_expansion_as_a_with_choices_call() {
        let spec =
            parse(quote! { Address { state => { widget: select { choices: STATES } } } }).unwrap();
        let tokens = spec.expand().to_string();
        expect_that!(tokens, contains_substring("with_choices"));
        expect_that!(tokens, contains_substring("STATES"));
        // The widget override still lands; choices are an addition, not a
        // replacement.
        expect_that!(tokens, contains_substring("with_custom_widget"));
    }

    #[gtest]
    fn an_arbitrary_expression_is_accepted_as_a_list() {
        // Whatever it is, it only has to be `IntoIterator<Item: Into<Choice>>`
        // at the call site — the macro never inspects it.
        let spec = parse(quote! { Address { state => { widget: select { choices: states() } } } })
            .unwrap();
        expect_that!(spec.expand().to_string(), contains_substring("states ()"));
    }

    #[gtest]
    fn a_non_chooser_is_told_which_widgets_take_choices() {
        let msg = parse(quote! { Address { state => { widget: text { choices: STATES } } } })
            .expect_err("`text` has nothing to choose from")
            .to_string();
        expect_that!(msg, contains_substring("`text` takes no `choices`"));
        expect_that!(msg, contains_substring("`select`"));
        expect_that!(msg, contains_substring("`radio_group`"));
    }

    #[gtest]
    fn a_custom_widget_is_told_to_manage_its_own() {
        expect_that!(
            parse(quote! { Source { notes => { widget: custom(Picker) { choices: STATES } } } })
                .expect_err("a custom widget never receives them")
                .to_string(),
            contains_substring("supplies its own choices")
        );
    }

    #[gtest]
    fn an_unknown_argument_names_what_was_expected() {
        expect_that!(
            parse(quote! { Address { state => { widget: select { rows: 4 } } } })
                .expect_err("`rows` is not an argument yet")
                .to_string(),
            contains_substring("unknown widget argument `rows`")
        );
    }

    #[gtest]
    fn two_choices_keys_are_rejected() {
        expect_that!(
            parse(quote! { Address { state => { widget: select { choices: A, choices: B } } } })
                .expect_err("the second silently winning would be worse")
                .to_string(),
            contains_substring("duplicate `choices`")
        );
    }

    #[gtest]
    fn an_empty_argument_block_is_rejected() {
        expect_that!(
            parse(quote! { Address { state => { widget: select {} } } })
                .expect_err("braces that say nothing")
                .to_string(),
            contains_substring("empty widget arguments")
        );
    }

    /// The braces are optional, so everything written before they existed still
    /// parses unchanged.
    #[gtest]
    fn a_widget_without_braces_is_unaffected() {
        let spec = parse(quote! { Address { state => { widget: select } } }).unwrap();
        expect_that!(fields(&spec)[0].1, eq("select"));
        expect_that!(
            spec.expand().to_string(),
            not(contains_substring("with_choices"))
        );
    }

    #[gtest]
    fn a_trailing_comma_inside_the_braces_is_allowed() {
        expect_that!(
            parse(quote! { Address { state => { widget: select { choices: STATES, } } } }),
            ok(anything())
        );
    }

    // ── `custom(…)` ──────────────────────────────────────────────────────

    #[gtest]
    fn a_custom_widget_parses() {
        let spec = parse(quote! { Source { notes => { widget: custom(MarkdownWidget) } } })
            .expect("custom(MarkdownWidget) should parse");
        expect_that!(fields(&spec)[0].1, contains_substring("MarkdownWidget"));
    }

    #[gtest]
    fn a_custom_widget_may_be_module_qualified() {
        // The reason it is a `Path` and not an `Ident`: naming a widget should not
        // require importing it.
        let spec =
            parse(quote! { Source { notes => { widget: custom(widgets::MarkdownWidget) } } })
                .expect("a qualified component path should parse");
        expect_that!(fields(&spec)[0].1, contains_substring("MarkdownWidget"));
    }

    #[gtest]
    fn a_custom_widget_expands_to_a_non_capturing_closure() {
        // Pins the whole contract: the input is placed INSIDE rsx! (so it gets a
        // component scope and may use hooks) rather than called, the closure
        // captures nothing (so it coerces to `fn(WidgetProps) -> Element`), and
        // the readable name rides along because Debug on a fn pointer is an
        // address.
        let spec =
            parse(quote! { Source { notes => { widget: custom(MarkdownWidget) } } }).unwrap();
        let tokens = widget_of(&spec).path().to_string();
        expect_that!(tokens, contains_substring("WidgetType :: Custom"));
        expect_that!(tokens, contains_substring("name : \"MarkdownWidget\""));
        expect_that!(tokens, contains_substring("render : | __p |"));
        expect_that!(tokens, contains_substring("rsx !"));
        expect_that!(tokens, contains_substring("values : __p . values"));
    }

    #[gtest]
    fn a_qualified_custom_widget_keeps_only_the_last_segment_as_its_name() {
        let spec =
            parse(quote! { Source { notes => { widget: custom(a::b::MarkdownWidget) } } }).unwrap();
        expect_that!(
            widget_of(&spec).path().to_string(),
            contains_substring("name : \"MarkdownWidget\"")
        );
    }

    #[gtest]
    fn custom_without_a_widget_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes => { widget: custom } } }),
            contains_substring("custom(MyWidget)")
        );
    }

    #[gtest]
    fn custom_with_two_widgets_is_rejected() {
        expect_that!(
            err_of(quote! { Source { notes => { widget: custom(A, B) } } }),
            contains_substring("one component")
        );
    }
}
