//! The checkbox.

use dioxus::prelude::*;

use super::errors::FieldErrors;
use super::types::FieldProps;
use super::values::{get_current, write_value};
use crate::members::ValuesStore;

#[component]
pub fn Checkbox(
    mut values: ValuesStore,
    props: FieldProps,
    /// `required_true` in `form!`: the box must be ticked. Shows the ` *`
    /// marker. The HTML `required` attribute does not come from here; it
    /// arrives in `attrs`, from `FormField::constraint_attributes`, like every other
    /// constraint.
    required_true: bool,
    #[props(extends = input)] attrs: Vec<Attribute>,
) -> Element {
    let field_class = props.field_class();

    let FieldProps {
        path,
        label,
        errors,
        required: _,
        aria_invalid,
    } = props;

    // `props.required` is deliberately dropped rather than forwarded. It means
    // PRESENCE, and for a `bool` unticked is a complete answer, while HTML
    // `required` on a checkbox means "must be ticked". That second meaning is
    // `required_true`, which is a separate prop so that `required` keeps one
    // meaning in the Rust API.

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
                class: "fx-control fx-checkbox",
                name: "{path}",
                r#type: "checkbox",
                checked: get_current(&path, values) == "true",
                // A checkbox has nowhere to put an invalid icon, but the
                // attribute still drives any border or adjacent-message styling
                // a consumer writes, and it is what a screen reader announces
                // either way.
                aria_invalid,
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
                    if required_true {
                        span { class: "fx-required", aria_hidden: "true", " *" }
                    }
                }
            }
            FieldErrors { errors }
        }
    }
}
