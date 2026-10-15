//! A field's caption and its required marker, shared by every widget so each
//! is rendered, and later given its class, in one place.

use dioxus::prelude::*;
use formoxus_spec::{BuiltinWidget, FixedKey};

use crate::classes::Classes;

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
    invalid: bool,
    widget: BuiltinWidget,
) -> Element {
    let label_class = classes.part_class(widget, "label", invalid);
    rsx! {
        if let Some(text) = text {
            label {
                class: label_class,
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
pub(crate) fn required_marker(classes: &Classes) -> Element {
    let class_for_required = classes.fixed_class(FixedKey::RequiredMarker, false);
    rsx! {
        span { class: class_for_required, aria_hidden: "true", " *" }
    }
}
