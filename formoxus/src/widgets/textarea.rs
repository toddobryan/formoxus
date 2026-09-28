//! The `<textarea>`.

use dioxus::prelude::*;

use super::errors::FieldErrors;
use super::types::FieldProps;
use super::values::{get_current, write_value};
use crate::ValuesByPath;

/// A multi-line text input bound to one path in the value map.
///
/// Otherwise identical to [`Input`] — same controlled-value binding, same
/// `aria-invalid` rule, same label layout. There is no hidden-input branch to
/// mirror: `Textarea` is only ever chosen as an override on a `Text` field, and
/// nothing here needs the extra `InputType` cases (`Password`'s masking,
/// `Hidden`'s bare markup) that make `Input` carry one.
#[component]
pub fn Textarea(
    values: ValuesByPath,
    props: FieldProps,
    #[props(extends = textarea)] attrs: Vec<Attribute>,
) -> Element {
    let FieldProps {
        path,
        label: label_text,
        required,
        errors,
    } = props;

    let current = get_current(&path, values);

    let invalid = (!errors.is_empty()).then_some("true");

    rsx! {
        label { class: "form-field",
            if let Some(text) = label_text {
                // The marker is INSIDE the label span, not a sibling: a
                // consumer who makes `.field-label` a block — the natural
                // choice above an input — would otherwise push a lone asterisk
                // onto its own line. `aria-hidden` because it is a VISUAL
                // convention only; `required` on the input is what tells a
                // screen reader, so the asterisk would just be noise in the
                // accessible name.
                span { class: "field-label",
                    "{text}"
                    if required {
                        span { class: "required", aria_hidden: "true", " *" }
                    }
                }
            }
            textarea {
                name: "{path}",
                value: "{current}",
                required,
                aria_invalid: invalid,
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&path, values, raw);
                },
                // TODO: delete pattern, since not allowed in HTML
                ..attrs,
            }
            FieldErrors { errors }
        }
    }
}
