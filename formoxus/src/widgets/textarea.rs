//! The `<textarea>`.

use dioxus::prelude::*;
use formoxus_attrs::FieldType;

use super::errors::FieldErrors;
use super::types::FieldProps;
use super::values::{get_current, write_value};
use crate::fields::FieldAttrs;
use crate::members::ValuesStore;

/// A multi-line text input bound to one path in the value map.
///
/// Otherwise identical to [`Input`] — same controlled-value binding, same
/// `aria-invalid` rule, same label layout. There is no hidden-input branch to
/// mirror: `Textarea` is only ever chosen as an override on a `Text` field, and
/// nothing here needs the extra `InputType` cases (`Password`'s masking,
/// `Hidden`'s bare markup) that make `Input` carry one.
#[component]
pub fn Textarea(
    field_type: FieldType,
    field_attrs: FieldAttrs,
    values: ValuesStore,
    props: FieldProps,
    #[props(extends = textarea)] attrs: Vec<Attribute>,
) -> Element {
    let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(field_type, attrs);

    let field_class = props.field_class();

    let FieldProps {
        path,
        label: label_text,
        required,
        errors,
        aria_invalid,
    } = props;

    let current = get_current(&path, values);

    rsx! {
        label { class: field_class,
            if let Some(text) = label_text {
                // The marker is INSIDE the label span, not a sibling: a
                // consumer who makes `.fx-field-label` a block — the natural
                // choice above an input — would otherwise push a lone asterisk
                // onto its own line. `aria-hidden` because it is a VISUAL
                // convention only; `required` on the input is what tells a
                // screen reader, so the asterisk would just be noise in the
                // accessible name.
                span { class: "fx-field-label",
                    "{text}"
                    if required {
                        span { class: "fx-required", aria_hidden: "true", " *" }
                    }
                }
            }
            textarea {
                class: "fx-control fx-textarea",
                name: "{path}",
                value: "{current}",
                required,
                aria_invalid,
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
