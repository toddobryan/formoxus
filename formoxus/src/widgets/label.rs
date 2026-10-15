//! A field's caption and its required marker, shared by every widget so each
//! is rendered, and later given its class, in one place.

use dioxus::prelude::*;

use crate::Classes;

/// The caption, pointing at the form control by `id`, with the required marker
/// when `marked`. Nothing at all when there is no caption, so the marker never
/// floats on its own: an asterisk with nothing to qualify means nothing.
///
/// The marker is INSIDE the label, not a sibling: a consumer who makes
/// `.fx-field-label` a block (the natural choice above an input) would
/// otherwise push a lone asterisk onto its own line.
pub(crate) fn field_label(
    id: &str,
    text: Option<String>,
    marked: bool,
    classes: &Classes,
) -> Element {
    rsx! {
        if let Some(text) = text {
            label {
                class: "fx-field-label",
                r#for: "{id}",
                "{text}"
                if marked {
                    {required_marker(classes)}
                }
            }
        }
    }
}

/// The ` *` after a required field's caption, in a `<label>` or a `<legend>`.
///
/// `aria-hidden` because it is a VISUAL convention only: the `required`
/// attribute is what a screen reader announces, so the asterisk would just be
/// noise in the accessible name.
pub(crate) fn required_marker(_classes: &Classes) -> Element {
    rsx! {
        span { class: "fx-required", aria_hidden: "true", " *" }
    }
}
