//! A field's hint, `help_text:`: Django's `help_text`, a persistent line
//! under the caption saying what the field wants ("At least 8 characters").
//!
//! Not a `placeholder`, which vanishes once the user types and is not
//! reliably announced as a description. The hint stays, and the control is
//! described by it (`aria-describedby`), so a screen reader reads it after
//! the field's name, before any error.

use dioxus::prelude::*;
use formoxus_spec::FixedKey;

use crate::classes::Classes;
use crate::widgets::errors::errors_id;

/// The id of a field's hint, `{field id}-help`, which its control's
/// `aria-describedby` names.
pub fn help_id(field_id: &str) -> String {
    format!("{field_id}-help")
}

/// What a control or group is described by: its hint, when it has one, then
/// its error list, when it is invalid; `None` when it has neither, since
/// naming an element that is not rendered is itself a defect.
///
/// Hint first, so a screen reader reads what the field wants before what
/// was wrong with the last answer, which is also the order they render in.
pub fn described_by(field_id: &str, has_help: bool, invalid: bool) -> Option<String> {
    match (has_help, invalid) {
        (false, false) => None,
        (true, false) => Some(help_id(field_id)),
        (false, true) => Some(errors_id(field_id)),
        (true, true) => Some(format!("{} {}", help_id(field_id), errors_id(field_id))),
    }
}

/// The hint, a `<div>` with the id [`described_by`] names, or nothing at all
/// without one. A custom widget renders it where the built-ins do: after the
/// caption, before the control.
pub fn field_help(field_id: &str, text: Option<&str>, classes: &Classes) -> Element {
    let class = classes.fixed_class(FixedKey::HelpText, false);
    rsx! {
        if let Some(text) = text {
            div { class, id: "{help_id(field_id)}", "{text}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::described_by;
    use googletest::prelude::*;

    const ID: &str = "aB3xY9kq-name";

    /// Spelled out as literals, so a change to either id's format has to
    /// change this test too, on purpose.
    #[gtest]
    fn described_by_names_the_hint_then_the_errors() {
        expect_that!(described_by(ID, false, false), none());
        expect_that!(
            described_by(ID, true, false),
            some(eq("aB3xY9kq-name-help"))
        );
        expect_that!(
            described_by(ID, false, true),
            some(eq("aB3xY9kq-name-errors"))
        );
        expect_that!(
            described_by(ID, true, true),
            some(eq("aB3xY9kq-name-help aB3xY9kq-name-errors"))
        );
    }
}
