//! The `<input>` family — every `type=` whose value the DOM hands back as
//! plain text.

use dioxus::prelude::*;
use formoxus_attrs::FieldType;

use super::errors::FieldErrors;
use super::types::{FieldProps, InputType};
use super::values::{get_current, write_value};
use crate::fields::FieldAttrs;
use crate::members::ValuesStore;

#[component]
pub fn Input(
    input_type: InputType,
    field_type: FieldType,
    field_attrs: FieldAttrs,
    values: ValuesStore,
    props: FieldProps,
    #[props(extends = input)] attrs: Vec<Attribute>,
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

    // A hidden widget renders BARE. The wrapper below is a `label` with a caption
    // and a required marker, which for `type="hidden"` would put visible text
    // and an asterisk on screen beside a widget nobody can see — and label an
    // unlabelable element for a screen reader.
    if matches!(input_type, InputType::Hidden) {
        return rsx! {
            input {
                r#type: "hidden",
                name: "{path}",
                value: current,
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&path, values, raw);
                },
            }
        };
    }

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
            input {
                class: "fx-control fx-input",
                r#type: input_type.html_type(),
                name: "{path}",
                value: "{current}",
                required,
                aria_invalid,
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&path, values, raw);
                },
                // Last, and it has to be: rsx! treats everything after a spread
                // as children, so an attribute below this is a parse error.
                ..attrs,
            }
            FieldErrors { errors }
        }
    }
}
