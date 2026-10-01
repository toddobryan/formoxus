//! The per-field error list every widget renders under its widget.

use dioxus::prelude::*;

use crate::error::ValidationMessage;

/// The per-field error list, rendered under every widget.
///
/// **formoxus ships no stylesheet and depends on no CSS framework.**
/// `fx-field-errors` and `fx-field-error` are its own class names; style them
/// any other markup, with whatever you already use.
///
/// Rendered as an immediate sibling of the widget rather than somewhere
/// further out, which puts the message next to its field in reading order and
/// lets a plain `.fx-control[aria-invalid="true"] + *` sibling selector reach it
/// with no framework and no extra class.
///
/// That selector is no longer the only route: the field wrapper now carries
/// `fx-invalid` whenever there are errors, so `.fx-invalid .fx-field-errors`
/// works too and does not care about order. Which is what lets `Checkbox` put
/// its caption between the box and this list — a checkbox reads "☐ I agree",
/// so the control has to come first there, and adjacency could not survive it.
///
/// A `ul` of `li`, matching [`FormState::render_errors`](crate::FormState).
/// This was a `small` for a while, which asserts "fine print" — wrong for an
/// error, and chosen back when one framework's stylesheet was in mind. A list
/// element asserts something that is simply true: these are several messages,
/// and assistive technology announces the structure and the count. **When
/// error rendering becomes overridable (the same mechanism as custom widgets),
/// this is the component to swap**, and the choice stops being global.
///
/// The `aria-invalid` half lives on each widget's own widget and is not a
/// styling matter at all: it is the W3C ARIA attribute assistive technology
/// reads to announce a field as errored. Leaving it off is an accessibility
/// defect, not a theming preference.
#[component]
pub fn FieldErrors(errors: Vec<ValidationMessage>) -> Element {
    rsx! {
        if !errors.is_empty() {
            ul { class: "fx-field-errors",
                for error in errors.iter() {
                    li { class: "fx-field-error", "{error.0}" }
                }
            }
        }
    }
}
