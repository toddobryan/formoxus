//! The buttons that add, remove and move a list's rows.
//!
//! None reads the value store. Adding or dropping a row changes the form's
//! SHAPE, and moving one changes its order, so all any of them does is put an [`Edit`] on the wire and let
//! [`ListSet`](crate::nodes::ListSet) rebuild itself.

use std::rc::Rc;

use dioxus::prelude::*;
use formoxus_spec::FixedKey;

use crate::{classes::Classes, nodes::Edit};

use super::icons::{CHEVRON_DOWN, CHEVRON_UP, PLUS, X, icon};

/// The widget that appends a row to a list.
///
/// Like [`VariantSelect`](super::VariantSelect), it reads nothing from the value store — adding a row
/// is a change to the form's *shape*, so all it does is put an [`Edit`] on the
/// wire. `type="button"` is load-bearing: inside a `<form>` a bare `<button>`
/// defaults to `type="submit"`, so omitting it would submit the form instead of
/// adding a row.
#[component]
pub fn AddRowButton(path: String, on_edit: Callback<Edit>, classes: Rc<Classes>) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: classes.fixed_class(FixedKey::AddRowButton, false),
            onclick: move |_| {
                // Append. `before` exists for mid-list insertion, which needs a
                // widget between every pair of rows — a UI question that hasn't
                // been answered yet, not a limitation of the edit.
                on_edit.call(Edit::AddRow { path: path.clone(), value: None, before: None });
            },
            // Icon and word: there is one Add per list, so the text costs
            // nothing, and a lone "+" under a list says less than "+ Add".
            {icon(PLUS)}
            "Add"
        }
    }
}

/// The widget that drops one row from a list.
///
/// Addressed by POSITION, not by the row's key: the list is what applies the
/// edit and it works in terms of `rows`, so a position is what it can act on
/// directly. Keys identify a row across edits; a position locates one at an
/// instant, which is all a click needs to say.
#[component]
pub fn RemoveRowButton(
    path: String,
    index: usize,
    on_edit: Callback<Edit>,
    classes: Rc<Classes>,
) -> Element {
    // 1-based for humans: this is the only place a row's position is spoken
    // aloud, and it is never used as a path segment.
    let ordinal = index + 1;
    rsx! {
        button {
            r#type: "button",
            class: classes.fixed_class(FixedKey::RemoveRowButton, false),
            aria_label: "Remove row {ordinal}",
            onclick: move |_| {
                on_edit.call(Edit::RemoveRow { path: path.clone(), index });
            },
            {icon(X)}
        }
    }
}

/// The ↑ and ↓ buttons that move one row of a reorderable list, rendered on
/// the row's left, so neither is ever next to its Remove.
///
/// Each moves the row one place, by position as [`RemoveRowButton`] removes
/// one. ↑ is disabled on the first row and ↓ on the last rather than left out,
/// so every row's controls line up and a keyboard user tabbing down the list
/// meets the same buttons in each.
#[component]
pub fn MoveRowButtons(
    path: String,
    index: usize,
    /// How many rows the list has, which is what says this one is the last.
    count: usize,
    on_edit: Callback<Edit>,
    classes: Rc<Classes>,
) -> Element {
    let ordinal = index + 1;
    let up_path = path.clone();
    rsx! {
        div { class: classes.fixed_class(FixedKey::MoveRowButtons, false),
            button {
                r#type: "button",
                class: classes.fixed_class(FixedKey::MoveRowUpButton, false),
                aria_label: "Move row {ordinal} up",
                disabled: index == 0,
                onclick: move |_| {
                    on_edit.call(Edit::MoveRow { path: up_path.clone(), from: index, to: index - 1 });
                },
                {icon(CHEVRON_UP)}
            }
            button {
                r#type: "button",
                class: classes.fixed_class(FixedKey::MoveRowDownButton, false),
                aria_label: "Move row {ordinal} down",
                disabled: index + 1 == count,
                onclick: move |_| {
                    on_edit.call(Edit::MoveRow { path: path.clone(), from: index, to: index + 1 });
                },
                {icon(CHEVRON_DOWN)}
            }
        }
    }
}
