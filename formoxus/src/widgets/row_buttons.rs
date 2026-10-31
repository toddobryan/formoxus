//! The buttons that add, remove and move a list's rows.
//!
//! None reads the value store. Adding or dropping a row changes the form's
//! SHAPE, and moving one changes its order, so all any of them does is put an [`Edit`] on the wire and let
//! [`ListSet`](crate::nodes::ListSet) rebuild itself.
//!
//! Add is the one a dev can replace: `adder:` in `form!` names a component
//! taking [`AddRowProps`], held as a [`RowAdder`], and [`AddRowButton`] is the
//! default.

use std::rc::Rc;
use std::{fmt::Debug, ptr::fn_addr_eq};

use dioxus::prelude::*;
use formoxus_spec::FixedKey;

use crate::nodes::ValuesStore;
use crate::{classes::Classes, nodes::Edit};

use super::icons::{CHEVRON_DOWN, CHEVRON_UP, PLUS, X, icon};

/// What a list's adder is handed: [`AddRowButton`], or a dev's own component
/// named by `adder:` in `form!`.
///
/// An adder appends a row by sending an [`Edit::AddRow`] through `on_edit`.
/// With `value: None` the row arrives blank and takes focus. With
/// `value: Some(RowValue::new(row))` it arrives filled in, and focus is left
/// alone, so a user picking several choices in a row stays on the adder.
#[derive(Debug, Props, Clone, PartialEq)]
pub struct AddRowProps {
    /// The list's own path, which an `Edit::AddRow` is addressed to. Not a
    /// row's: an adder acts on the list.
    pub path: String,
    /// The list's own id, `{form}-{path}`, a base for the ids of the adder's
    /// elements (a `<select>` and the `<label for>` naming it). Suffix it
    /// rather than using it as-is: the list's `<fieldset>` already has it.
    pub id: String,
    /// The whole form's values. Add ignores it; an adder offering a fixed set
    /// of choices can read the rows already there, to leave those out. The
    /// adder is a component of its own, so reading it re-renders only the
    /// adder.
    pub values: ValuesStore,
    /// Where the adder sends its `Edit`.
    pub on_edit: Callback<Edit>,
    /// The form's resolved classes, so a custom adder can borrow the
    /// built-in Add's slot (`classes.fixed_class(FixedKey::AddRowButton, false)`)
    /// and follow the app's preset.
    pub classes: Rc<Classes>,
    /// How many rows the list has now, for an adder that stops at a maximum.
    pub rows: usize,
}

/// A list's adder: the component that renders in place of Add.
///
/// A newtype rather than a bare `fn` pointer because [`FieldSpec`] derives
/// `PartialEq`, and a derived comparison of a `fn` pointer trips the
/// `unpredictable_function_pointer_comparisons` lint. The comparison is
/// written out here instead, with [`fn_addr_eq`].
///
/// The field is `pub` because `form!` builds one in the dev's own crate:
/// `adder: SearchToAdd` becomes `RowAdder(SearchToAdd)`.
///
/// [`FieldSpec`]: crate::form::FieldSpec
#[derive(Clone, Copy)]
pub struct RowAdder(pub fn(AddRowProps) -> Element);

impl Default for RowAdder {
    fn default() -> Self {
        Self(AddRowButton)
    }
}

// Not derived: a `fn` pointer's `Debug` is its address, which changes from
// run to run and names nothing a reader could look up.
impl Debug for RowAdder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RowAdder(..)")
    }
}

impl PartialEq for RowAdder {
    fn eq(&self, other: &Self) -> bool {
        fn_addr_eq(self.0, other.0)
    }
}

/// The widget that appends a row to a list, and the default [`RowAdder`].
///
/// Like [`VariantSelect`](super::VariantSelect), it reads nothing from the value store — adding a row
/// is a change to the form's *shape*, so all it does is put an [`Edit`] on the
/// wire. `type="button"` is load-bearing: inside a `<form>` a bare `<button>`
/// defaults to `type="submit"`, so omitting it would submit the form instead of
/// adding a row.
#[component]
pub fn AddRowButton(props: AddRowProps) -> Element {
    let AddRowProps {
        path,
        on_edit,
        classes,
        ..
    } = props;
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
