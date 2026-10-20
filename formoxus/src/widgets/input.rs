//! The `<input>` family: every `type=` formoxus renders, routed by
//! [`RenderedInputType`] to one of three bodies. Text-like types (whose value
//! the DOM hands back as plain text) read caption-then-input; a checkbox
//! reads box-then-caption and binds `checked`; a hidden input renders bare.

use dioxus::prelude::*;
use formoxus_spec::{
    Attr, AttrSet, BuiltinWidget, Element as FxElement, FieldControl, InputType, RenderedInputType,
};

use super::errors::FieldErrors;
use super::label::field_label;
use super::types::FieldProps;
use super::values::{get_current, write_value};
use crate::classes::Slot;
use crate::field_attrs::FieldAttrs;
use crate::nodes::ValuesStore;
use crate::widgets::{Widget, widget_props::WidgetProps};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct TextInput;

impl Widget for TextInput {
    const NAME: &'static str = "text";

    const EXTENDS: AttrSet =
        AttrSet::valid_on(FxElement::Control(FieldControl::Input(InputType::Text)));

    fn render(&self, props: WidgetProps) -> Element {
        rsx! {
            InputView {
                input_type: RenderedInputType::Text,
                field_attrs: props.attrs,
                values: props.values,
                props: FieldProps {
                    id: props.id,
                    path: props.path,
                    label: props.label,
                    classes: props.classes,
                    errors: props.errors,
                },
                attrs: props.extra_attrs,
            }
        }
    }

    fn default_classes() -> Vec<(&'static str, Slot)> {
        vec![
            ("div", Slot::new("fx-form-field").with_invalid("fx-invalid")),
            ("label", Slot::new("fx-field-label")),
            ("input", Slot::new("fx-control fx-input")),
        ]
    }
}

#[component]
pub(crate) fn InputView(
    input_type: RenderedInputType,
    field_attrs: FieldAttrs,
    values: ValuesStore,
    props: FieldProps,
    #[props(extends = input)] attrs: Vec<Attribute>,
) -> Element {
    // No wildcard arm, on purpose: a type that joins `RenderedInputType`
    // (`Range` is the likely one) has to be routed here before it compiles.
    match input_type {
        RenderedInputType::Hidden => hidden(props.path, values),
        RenderedInputType::Checkbox => checkbox(&field_attrs, values, props, attrs),
        RenderedInputType::Text
        | RenderedInputType::Password
        | RenderedInputType::Number
        | RenderedInputType::Email
        | RenderedInputType::Telephone
        | RenderedInputType::Url
        | RenderedInputType::Search
        | RenderedInputType::Color
        | RenderedInputType::Date
        | RenderedInputType::Time
        | RenderedInputType::DatetimeLocal
        | RenderedInputType::Month
        | RenderedInputType::Week => text_like(input_type, &field_attrs, values, props, attrs),
    }
}

/// A hidden input renders BARE. Every other body wraps the input with a
/// `<label>` caption and a required marker, which for `type="hidden"` would put
/// visible text and an asterisk on screen beside a widget nobody can see, and
/// label an unlabelable element for a screen reader.
fn hidden(path: String, values: ValuesStore) -> Element {
    let current = get_current(&path, values);
    rsx! {
        input {
            r#type: "hidden",
            name: "{path}",
            value: current,
            oninput: move |e: FormEvent| {
                let raw = e.value();
                write_value(&path, values, raw);
            },
        }
    }
}

// A PASSWORD IS AN ORDINARY CONTROLLED INPUT HERE. `type="password"` masks the
// glyphs; nothing else about it is special, and `value` is bound like every
// other field's.
//
// Django's `render_value=False` and Rails' non-echoing `password_field` are
// real conventions, but they are SERVER-RENDERING ones: there the value would
// land in an HTTP response body, and so in proxy logs, shared caches, browser
// history and view-source. No response body carries it here — the value goes
// keystroke -> DOM -> a wasm-side store in the user's own browser, and binding
// it back writes to the very node they typed into. The only viewer is the
// person who just typed it. A server-rendered response with a populated
// password would bring the concern back; this architecture produces none,
// because the SSR pass renders an empty form and every later re-render is
// client-side.
//
// Withholding it cost more than it bought: a field nothing could CLEAR
// programmatically (so "reset after a successful change" became impossible),
// and the only asymmetric input in the set.
fn text_like(
    input_type: RenderedInputType,
    field_attrs: &FieldAttrs,
    values: ValuesStore,
    props: FieldProps,
    attrs: Vec<Attribute>,
) -> Element {
    let my_widget = BuiltinWidget::Input(input_type);
    let is_invalid = props.is_invalid();

    let class: Option<String> =
        field_attrs.class(props.classes.part(my_widget, "input"), props.is_invalid());
    let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(attrs);

    let field_class = props.classes.part_class(my_widget, "div", is_invalid);

    let current = get_current(&props.path, values);

    let label = field_label(
        &props.id,
        props.label,
        field_attrs.contains(Attr::Required),
        &props.classes,
        is_invalid,
        my_widget,
    );

    rsx! {
        div {
            class: field_class,
            {label}
            input {
                class: class,
                r#type: input_type.to_input_type().html_type(),
                id: "{props.id}",
                name: "{props.path}",
                value: "{current}",
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&props.path, values, raw);
                },
                // Last, and it has to be: rsx! treats everything after a spread
                // as children, so an attribute below this is a parse error.
                ..attrs,
            }
            FieldErrors { field_id: &props.id, errors: props.errors, classes: props.classes.clone() }
        }
    }
}

fn checkbox(
    field_attrs: &FieldAttrs,
    values: ValuesStore,
    props: FieldProps,
    attrs: Vec<Attribute>,
) -> Element {
    let my_widget = BuiltinWidget::Input(RenderedInputType::Checkbox);
    let is_invalid = props.is_invalid();

    let class = field_attrs.class(props.classes.part(my_widget, "input"), props.is_invalid());
    let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(attrs);

    let field_class = props.classes.part_class(my_widget, "div", is_invalid);

    let label = field_label(
        &props.id,
        props.label,
        field_attrs.contains(Attr::RequiredTrue),
        &props.classes,
        is_invalid,
        my_widget,
    );

    // The wrapper is unconditional and the caption is not, exactly as for a
    // text input. Until 2026-09-29 this widget wrapped an unclassed `<label>`
    // only when it had text, and put that text in no span, so
    // `.fx-form-field` and `.fx-field-label` both silently missed every
    // checkbox on the page.
    rsx! {
        div {
            class: field_class,
            // The box comes BEFORE its text. Every other widget reads
            // caption-then-control, but a checkbox reads "☐ I agree", not
            // "I agree ☐": the label is what the box means, not what to type
            // into it.
            input {
                class: class,
                name: "{props.path}",
                id: "{props.id}",
                r#type: "checkbox",
                checked: get_current(&props.path, values) == "true",
                onchange: move |e: FormEvent| write_value(&props.path, values, e.value()),
                // Last: rsx! reads anything after a spread as children.
                ..attrs,
            }
            // Marked by `required_true`, not `required`: on a bool, `required`
            // is presence, which a checkbox always has.
            {label}
            FieldErrors { field_id: &props.id, errors: props.errors, classes: props.classes.clone() }
        }
    }
}
