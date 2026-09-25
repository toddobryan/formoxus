//! `widget: <name>` and `widget: custom(…)` — the author-facing widget
//! vocabulary, and the arguments a widget accepts.

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{
    Expr, Ident, Path, Token, braced, parenthesized,
    parse::{Parse, ParseStream},
};

use super::suggest::{edit_distance, to_snake};

/// The author-facing widget vocabulary, and the `WidgetType` each name means.
///
/// **Flat and lowercase, deliberately.** `WidgetType::Input(InputType::Password)`
/// is the shape of formoxus's own enum — grouping the `<input type=X>` family
/// under one variant is a dispatch convenience, not a concept an author has. HTML
/// spells it `type="password"` and so does this. That makes this table the stable
/// surface: the enum below it can be regrouped, renamed, or split without
/// touching a single `form!` call.
///
/// Names follow HTML where HTML has one (`tel`, not `telephone`; `datetime_local`
/// for `datetime-local`, since a hyphen cannot be an ident) and formoxus where it
/// does not (`integer`/`float`, which both become `type="number"`; `textarea`,
/// `select`).
///
/// Every name here is accepted whether or not `ScalarWidget` can render it yet.
/// Gating on that was considered and REJECTED: the macro crate cannot see
/// `ScalarWidget`'s match arms, so an "implemented" list would be a hand-kept copy
/// of a match in another crate — a worse sync hazard than the one this table
/// already has — and its first false positive would be `password`. An unwired
/// widget still panics at render, naming both the widget and the value kind.
macro_rules! widgets {
    (
        $( $name:ident => $variant:ident $( ( $input:ident ) )? ),* $(,)?
    ) => {
        /// The tokens for a known widget name, or `None` if it is not one.
        fn widget_tokens(name: &Ident) -> Option<TokenStream2> {
            match name.to_string().as_str() {
                $( stringify!($name) => Some(quote! {
                    ::formoxus::widgets::WidgetType::$variant
                    $( ( ::formoxus::widgets::InputType::$input ) )?
                }), )*
                _ => None,
            }
        }

        /// Every accepted name, for the "unknown widget" message. Generated from
        /// the same table as the match, so the two cannot disagree.
        const WIDGET_NAMES: &[&str] = &[ $( stringify!($name) ),* ];

        /// The `WidgetType` variant a known name maps to, which is what decides
        /// which fields it can render (see [`rule`]).
        fn widget_variant(name: &str) -> Option<&'static str> {
            match name {
                $( stringify!($name) => Some(stringify!($variant)), )*
                _ => None,
            }
        }
    };
}

widgets! {
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

mod widget_kw {
    syn::custom_keyword!(custom);
}

/// The widgets that can be handed a list to choose from.
///
/// A hand-kept list, and deliberately so: the alternative is gating on
/// `ScalarWidget`'s match arms, which this crate cannot see — the same reason
/// [`widgets!`] accepts every name whether or not it renders yet. The cost of
/// being wrong here is a good error message for a pair that would have panicked
/// at render anyway.
const CHOOSERS: &[&str] = &[
    "select",
    "select_multiple",
    "checkbox_multiple",
    "radio_group",
];

#[derive(Debug)]
pub(crate) struct WidgetRef {
    pub(crate) kind: WidgetKind,
    pub(crate) args: WidgetArgs,
}

#[derive(Debug)]
pub(crate) enum WidgetKind {
    /// One of the names in [`widgets!`], already validated.
    Named(Ident),
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
    ///
    /// Infallible: `parse` rejected anything not in the table, so the lookup here
    /// cannot miss.
    pub(crate) fn path(&self) -> TokenStream2 {
        match &self.kind {
            WidgetKind::Named(name) => {
                widget_tokens(name).expect("parse rejects names that are not in the table")
            }
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

/// Which fields a named widget can render, mirroring `ScalarWidget`'s match
/// arms in formoxus. Derived from the `WidgetType` variant, so a widget added to
/// [`widgets!`] gets a rule without a second list to keep in step.
enum Rule {
    /// Checked against the field's type, with this `field_kind::WidgetClass`
    /// and the fixed message a failed check shows.
    Checked {
        class: &'static str,
        message: &'static str,
    },
    /// Nothing renders it for any field, so `form!` rejects it while parsing.
    NotYet,
}

fn rule(name: &str) -> Rule {
    let variant = widget_variant(name).expect("only called with names from the table");
    match variant {
        "Input" => Rule::Checked {
            class: "Input",
            message: "an `<input>` widget cannot render a bool field; use `checkbox` or `select`",
        },
        "Textarea" => Rule::Checked {
            class: "Textarea",
            message: "`textarea` can only render a String field",
        },
        "Checkbox" => Rule::Checked {
            class: "Checkbox",
            message: "`checkbox` can only render a bool field",
        },
        "Select" | "RadioGroup" => Rule::Checked {
            class: "Chooser",
            message: "this widget needs `choices` unless the field is a bool",
        },
        "SelectMultiple" | "CheckboxMultiple" | "File" => Rule::NotYet,
        other => unreachable!("widget variant `{other}` has no rule; add one here"),
    }
}

impl WidgetRef {
    /// Free `const _` assertions that the field at `shape` can be rendered by
    /// this widget, with the caret on the widget's name. `shape` is a
    /// `field_kind::shape_of(…)` expression; see `FieldBody::type_checks`.
    pub(crate) fn checks(&self, shape: &TokenStream2) -> TokenStream2 {
        let span = match &self.kind {
            WidgetKind::Named(name) => name.span(),
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
        let WidgetKind::Named(name) = &self.kind else {
            return single;
        };
        let not_optional = if &name.to_string() == "radio_group" {
            quote_spanned! { span =>
                const _: () = ::core::assert!(
                    !::formoxus::field_kind::is_optional(#shape),
                    "`radio_group` cannot render an optional field — a picked radio can't be un-picked; use `select`"
                );
            }
        } else {
            TokenStream2::new()
        };
        let Rule::Checked { class, message } = rule(&name.to_string()) else {
            unreachable!("parse rejects widgets that render nothing");
        };
        let class = Ident::new(class, span);
        let has_choices = self.args.choices.is_some();
        quote_spanned! { span=>
            #single
            #not_optional
            const _: () = ::core::assert!(
                ::formoxus::field_kind::renders(
                    #shape,
                    ::formoxus::field_kind::WidgetClass::#class,
                    #has_choices,
                ),
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
        if widget_tokens(&name).is_none() {
            return Err(syn::Error::new(name.span(), unknown_widget(&name)));
        }
        if matches!(rule(&name.to_string()), Rule::NotYet) {
            return Err(syn::Error::new(
                name.span(),
                format!(
                    "`{name}` is not supported yet — nothing renders it, so the field \
                     would silently vanish from the form"
                ),
            ));
        }
        Ok(Self::Named(name))
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
        WidgetKind::Named(name) if CHOOSERS.contains(&name.to_string().as_str()) => Ok(()),
        WidgetKind::Named(name) => Err(syn::Error::new_spanned(
            key,
            format!(
                "`{name}` takes no `choices` — they apply to {}",
                CHOOSERS
                    .iter()
                    .filter(|c| !matches!(rule(c), Rule::NotYet))
                    .map(|c| format!("`{c}`"))
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
fn unknown_widget(name: &Ident) -> String {
    let written = name.to_string();
    let lowered = to_snake(&written);
    if lowered != written && WIDGET_NAMES.contains(&lowered.as_str()) {
        return format!(
            "unknown widget `{written}` — widget names are lowercase, write `{lowered}`"
        );
    }
    match WIDGET_NAMES
        .iter()
        .filter(|n| edit_distance(&lowered, n) <= 2)
        .min_by_key(|n| edit_distance(&lowered, n))
    {
        Some(near) => format!("unknown widget `{written}` — did you mean `{near}`?"),
        None => format!(
            "unknown widget `{written}` — expected one of {}, or `custom(MyWidget)`",
            WIDGET_NAMES.join(", ")
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
        // Generated from `WIDGET_NAMES`, so a name added to the table without a
        // match arm, or without a `rule`, fails here rather than at a call site.
        // A name whose rule is `NotYet` must instead be refused, by name.
        for name in WIDGET_NAMES {
            let id = Ident::new(name, proc_macro2::Span::call_site());
            let parsed = parse(quote! { Source { f => { widget: #id } } });
            if matches!(rule(name), Rule::NotYet) {
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
        expect_that!(tokens, contains_substring("WidgetType :: Input"));
        expect_that!(tokens, contains_substring("InputType :: Password"));
    }

    #[gtest]
    fn a_non_input_name_resolves_to_a_bare_variant() {
        let spec = parse(quote! { Source { notes => { widget: textarea } } }).unwrap();
        let tokens = widget_of(&spec).path().to_string();
        expect_that!(tokens, contains_substring("WidgetType :: Textarea"));
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
        // Whatever it is, it only has to be `IntoIterator<Item: Into<SelectChoice>>`
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
