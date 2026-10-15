//! The `<input>` family: every `type=` formoxus renders, routed by
//! [`RenderedInputType`] to one of three bodies. Text-like types (whose value
//! the DOM hands back as plain text) read caption-then-input; a checkbox
//! reads box-then-caption and binds `checked`; a hidden input renders bare.

use dioxus::prelude::*;
use formoxus_spec::{Attr, FieldType, RenderedInputType};

use super::errors::FieldErrors;
use super::label::field_label;
use super::types::FieldProps;
use super::values::{get_current, write_value};
use crate::fields::FieldAttrs;
use crate::members::ValuesStore;

#[component]
pub fn Input(
    input_type: RenderedInputType,
    field_type: FieldType,
    field_attrs: FieldAttrs,
    values: ValuesStore,
    props: FieldProps,
    #[props(extends = input)] attrs: Vec<Attribute>,
) -> Element {
    // No wildcard arm, on purpose: a type that joins `RenderedInputType`
    // (`Range` is the likely one) has to be routed here before it compiles.
    match input_type {
        RenderedInputType::Hidden => hidden(props.path, values),
        RenderedInputType::Checkbox => checkbox(field_type, &field_attrs, values, props, attrs),
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
        | RenderedInputType::Week => {
            text_like(input_type, field_type, &field_attrs, values, props, attrs)
        }
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
    field_type: FieldType,
    field_attrs: &FieldAttrs,
    values: ValuesStore,
    props: FieldProps,
    attrs: Vec<Attribute>,
) -> Element {
    let class: String = field_attrs.class("fx-control fx-input");
    let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(field_type, attrs);

    let field_class = props.field_class();

    let current = get_current(&props.path, values);

    rsx! {
        div {
            class: "{field_class}",
            {field_label(&props.id, props.label, field_attrs.contains(Attr::Required), &props.classes)}
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
    field_type: FieldType,
    field_attrs: &FieldAttrs,
    values: ValuesStore,
    props: FieldProps,
    attrs: Vec<Attribute>,
) -> Element {
    let class = field_attrs.class("fx-control fx-checkbox");
    let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(field_type, attrs);

    let field_class = props.field_class();

    // The wrapper is unconditional and the caption is not, exactly as for a
    // text input. Until 2026-09-29 this widget wrapped an unclassed `<label>`
    // only when it had text, and put that text in no span, so
    // `.fx-form-field` and `.fx-field-label` both silently missed every
    // checkbox on the page.
    rsx! {
        div {
            class: "{field_class}",
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
            {field_label(&props.id, props.label, field_attrs.contains(Attr::RequiredTrue), &props.classes)}
            FieldErrors { field_id: &props.id, errors: props.errors, classes: props.classes.clone() }
        }
    }
}
