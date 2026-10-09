//! The checkbox.

use dioxus::prelude::*;
use formoxus_attrs::{Attr, FieldType};

use super::errors::FieldErrors;
use super::types::FieldProps;
use super::values::{get_current, write_value};
use crate::fields::FieldAttrs;
use crate::members::ValuesStore;

#[component]
pub fn Checkbox(
    field_type: FieldType,
    field_attrs: FieldAttrs,
    mut values: ValuesStore,
    props: FieldProps,
    #[props(extends = input)] attrs: Vec<Attribute>,
) -> Element {
    let class = field_attrs.class("fx-control fx-checkbox");
    let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(field_type, attrs);

    let field_class = props.field_class();

    let FieldProps {
        path,
        label,
        errors,
    } = props;

    // The wrapper is unconditional and the caption is not, exactly as in
    // `Input`. Until 2026-09-29 this widget wrapped an unclassed `<label>` only
    // when it had text, and put that text in no span — so `.fx-form-field` and
    // `.fx-field-label` both silently missed every checkbox on the page.
    rsx! {
        label { class: field_class,
            // The box comes BEFORE its text. Every other widget reads
            // caption-then-control, but a checkbox reads "☐ I agree", not
            // "I agree ☐" — the label is what the box means, not what to type
            // into it.
            input {
                class: class,
                name: "{path}",
                r#type: "checkbox",
                checked: get_current(&path, values) == "true",
                onchange: move |e: FormEvent| write_value(&path, values, e.value()),
                // Last: rsx! reads anything after a spread as children.
                ..attrs,
            }
            if let Some(label_text) = label {
                // Inside the label span and `aria-hidden`, exactly as in
                // `Input`: the `required` attribute is what a screen reader
                // announces, and the asterisk is only for the eye.
                span { class: "fx-field-label",
                    "{label_text}"
                    if field_attrs.contains(Attr::RequiredTrue) {
                        span { class: "fx-required", aria_hidden: "true", " *" }
                    }
                }
            }
            FieldErrors { errors }
        }
    }
}
