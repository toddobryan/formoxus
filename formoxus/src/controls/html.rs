//! The `<input>` family and `<textarea>` — everything whose value is just
//! text as far as the DOM is concerned.

use dioxus::prelude::*;

use super::errors::FieldErrors;
use super::types::{FieldProps, InputType};
use super::values::{get_current, write_value};
use crate::ValuesByPath;

#[component]
pub fn HtmlInput(input_type: InputType, values: ValuesByPath, props: FieldProps) -> Element {
    let FieldProps {
        path,
        label: label_text,
        required,
        errors,
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

    // Present ONLY when there is an error. `aria-invalid="false"` is NOT the
    // neutral value — per ARIA it asserts "checked, and passed", so an
    // untouched form would claim to have validated every field, and a screen
    // reader would say so. Stylesheets that paint a validated-and-clean state
    // key off it too. Absent is the only neutral state. Dioxus omits an
    // attribute whose value is `None`, which is what makes absence
    // expressible at all.
    //
    // Unlike the `small` in `FieldErrors`, this is not a styling choice with a
    // framework behind it: `aria-invalid` is the W3C ARIA attribute assistive
    // technology reads to announce a field as errored, so it belongs here
    // whatever CSS the consumer brings.
    let invalid = (!errors.is_empty()).then_some("true");

    // A hidden control renders BARE. The wrapper below is a `label` with a caption
    // and a required marker, which for `type="hidden"` would put visible text
    // and an asterisk on screen beside a control nobody can see — and label an
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
        label { class: "form-field",
            if let Some(text) = label_text {
                span { class: "field-label", "{text}" }
            }
            if required {
                span { class: "required", " *" }
            }
            input {
                r#type: input_type.html_type(),
                name: "{path}",
                value: "{current}",
                required,
                aria_invalid: invalid,
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&path, values, raw);
                },
            }
            FieldErrors { errors }
        }
    }
}

/// A multi-line text input bound to one path in the value map.
///
/// Otherwise identical to [`HtmlInput`] — same controlled-value binding, same
/// `aria-invalid` rule, same label layout. There is no hidden-input branch to
/// mirror: `Textarea` is only ever chosen as an override on a `Text` field, and
/// nothing here needs the extra `InputType` cases (`Password`'s masking,
/// `Hidden`'s bare markup) that make `HtmlInput` carry one.
#[component]
pub fn TextareaInput(values: ValuesByPath, props: FieldProps) -> Element {
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
                span { class: "field-label", "{text}" }
            }
            if required {
                span { class: "required", " *" }
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
            }
            FieldErrors { errors }
        }
    }
}
