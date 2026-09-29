//! The checkbox.

use dioxus::prelude::*;

use super::errors::FieldErrors;
use super::types::FieldProps;
use super::values::{get_current, write_value};
use crate::ValuesByPath;

#[component]
pub fn Checkbox(
    mut values: ValuesByPath,
    props: FieldProps,
    #[props(extends = input)] attrs: Vec<Attribute>,
) -> Element {
    let FieldProps {
        path,
        label,
        errors,
        required: _,
        aria_invalid,
    } = props;

    // `required` is deliberately dropped rather than forwarded. HTML `required`
    // on a checkbox means "must be ticked", which is not what a required `bool`
    // field asks for — unticked is a complete answer. For the same reason there
    // is no ` *` marker: it would promise a rule nothing enforces.

    let input_element = rsx! {
        input {
            name: "{path}",
            r#type: "checkbox",
            checked: get_current(&path, values) == "true",
            // A checkbox has nowhere to put an invalid icon, but the
            // attribute still drives any border or adjacent-message styling a consumer
            // writes, and it is what a screen reader announces either way.
            aria_invalid,
            onchange: move |e: FormEvent| write_value(&path, values, e.value()),
            // Last: rsx! reads anything after a spread as children.
            ..attrs,
        }
        FieldErrors { errors: errors.clone() }
    };

    if let Some(label_text) = label.clone() {
        rsx! {
            label {
                "{label_text}"
                { input_element }
            }
        }
    } else {
        input_element
    }
}
