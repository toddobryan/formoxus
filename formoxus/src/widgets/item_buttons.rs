//! The buttons that add, remove and move a list's rows.
//!
//! None reads the value store. Adding or dropping a row changes the form's
//! SHAPE, and moving one changes its order, so all any of them does is put an [`Edit`] on the wire and let
//! [`ListNode`](crate::nodes::ListNode) rebuild itself.
//!
//! Add is the one a dev can replace: `adder:` in `form!` names a component
//! taking [`AddItemProps`], held as a [`ItemAdder`]. [`AddItemButton`] is a
//! list's default and [`DraftAdder`] a set's.

use std::rc::Rc;
use std::{fmt::Debug, ptr::fn_addr_eq};

use dioxus::prelude::*;
use formoxus_spec::FixedKey;

use crate::nodes::ValuesStore;
use crate::{classes::Classes, nodes::Edit};

use super::icons::{CHECK, CHEVRON_DOWN, CHEVRON_UP, PENCIL, PLUS, X, icon};

/// What a list's or a set's adder is handed: [`AddItemButton`],
/// [`DraftAdder`], or a dev's own component named by `adder:` in `form!`.
///
/// An adder appends a row by sending an [`Edit::AddItem`] through `on_edit`.
/// With `value: Some(ItemValue::new(item))` it arrives filled in, and focus is
/// left alone, so a user picking several choices in a row stays on the adder.
/// With `value: None` it means "add what the adder holds": in a list a blank
/// row, which takes focus, and in a set its `draft`.
#[derive(Debug, Props, Clone, PartialEq)]
pub struct AddItemProps {
    /// The list's own path, which an `Edit::AddItem` is addressed to. Not a
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
    /// A set's draft, rendered: the blank item, in the item type's own
    /// widget, that `Edit::AddItem { value: None }` makes an item. `None` for
    /// a list. An adder that sends its own value (a search-to-add) can leave
    /// it out.
    pub draft: Option<Element>,
    /// Whether the draft holds an item being edited (box 8), so Add would
    /// replace it: [`DraftAdder`] then says Save, and offers Cancel.
    pub editing: bool,
}

/// A list's adder: the component that renders in place of Add.
///
/// A newtype rather than a bare `fn` pointer because [`FieldSpec`] derives
/// `PartialEq`, and a derived comparison of a `fn` pointer trips the
/// `unpredictable_function_pointer_comparisons` lint. The comparison is
/// written out here instead, with [`fn_addr_eq`].
///
/// The field is `pub` because `form!` builds one in the dev's own crate:
/// `adder: SearchToAdd` becomes `ItemAdder(SearchToAdd)`.
///
/// [`FieldSpec`]: crate::form::FieldSpec
#[derive(Clone, Copy)]
pub struct ItemAdder(pub fn(AddItemProps) -> Element);

impl Default for ItemAdder {
    fn default() -> Self {
        Self(AddItemButton)
    }
}

// Not derived: a `fn` pointer's `Debug` is its address, which changes from
// run to run and names nothing a reader could look up.
impl Debug for ItemAdder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ItemAdder(..)")
    }
}

impl PartialEq for ItemAdder {
    fn eq(&self, other: &Self) -> bool {
        fn_addr_eq(self.0, other.0)
    }
}

/// The widget that appends a row to a list, and the default [`ItemAdder`].
///
/// Like [`VariantSelect`](super::VariantSelect), it reads nothing from the value store — adding a row
/// is a change to the form's *shape*, so all it does is put an [`Edit`] on the
/// wire. `type="button"` is load-bearing: inside a `<form>` a bare `<button>`
/// defaults to `type="submit"`, so omitting it would submit the form instead of
/// adding a row.
#[component]
pub fn AddItemButton(props: AddItemProps) -> Element {
    let AddItemProps {
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
                on_edit.call(Edit::AddItem { path: path.clone(), value: None, before: None });
            },
            // Icon and word: there is one Add per list, so the text costs
            // nothing, and a lone "+" under a list says less than "+ Add".
            {icon(PLUS)}
            "Add"
        }
    }
}

/// A set's default [`ItemAdder`]: its draft, then Add, which makes the draft
/// an item (`SETS_PLAN.md` 7d).
///
/// Enter in the draft is Add too, and `prevent_default` is what stops it
/// also submitting the whole form, which Enter in a text box inside a
/// `<form>` otherwise does. Only the draft is wrapped: Enter on Add already
/// clicks it, so a wrapper around both would add twice.
///
/// While an item is being edited (box 8), Add is Save, since it replaces
/// that item, and Cancel, or Escape in the draft, stops the edit and leaves
/// the item as it was.
///
/// After Add or Cancel, focus goes back to the draft, so a user adding
/// several items types one after another. The draft is rendered in the same
/// place, so the element is the same one, now under the next key.
#[component]
pub fn DraftAdder(props: AddItemProps) -> Element {
    let AddItemProps {
        path,
        id,
        on_edit,
        classes,
        draft,
        editing,
        ..
    } = props;
    let draft_id = format!("{id}-draft");
    let add_path = path.clone();
    let add = move || {
        on_edit.call(Edit::AddItem {
            path: add_path.clone(),
            value: None,
            before: None,
        });
    };
    let cancel = move || {
        on_edit.call(Edit::EditItem {
            path: path.clone(),
            index: None,
        });
    };
    let on_key = (add.clone(), cancel.clone());
    let focus_draft = move || {
        document::eval(&format!(
            "document.getElementById('{draft_id}')?.querySelector('input, select, textarea')?.focus()"
        ));
    };
    let after_cancel = focus_draft.clone();
    rsx! {
        div {
            id: "{id}-draft",
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::Enter {
                    e.prevent_default();
                    (on_key.0)();
                } else if editing && e.key() == Key::Escape {
                    e.prevent_default();
                    (on_key.1)();
                }
            },
            {draft}
        }
        button {
            r#type: "button",
            class: classes.fixed_class(FixedKey::AddRowButton, false),
            onclick: move |_| {
                add();
                focus_draft();
            },
            if editing {
                {icon(CHECK)}
                "Save"
            } else {
                {icon(PLUS)}
                "Add"
            }
        }
        if editing {
            button {
                r#type: "button",
                class: classes.fixed_class(FixedKey::Button(formoxus_spec::ButtonType::Cancel), false),
                onclick: move |_| {
                    cancel();
                    after_cancel();
                },
                "Cancel"
            }
        }
    }
}

/// A set item's Edit (box 8): puts the item into the adder's draft, marking
/// it as the one being edited. A toggle, `aria-pressed` while its item is
/// being edited, so a second press stops the edit, as Cancel does.
///
/// Named by the item's text, not its position: a set is sorted, so "item 2"
/// names nothing a user can see.
#[component]
pub fn EditItemButton(
    path: String,
    index: usize,
    /// The item as the user sees it, for the button's name.
    text: String,
    /// Whether this item is the one being edited.
    editing: bool,
    on_edit: Callback<Edit>,
    classes: Rc<Classes>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: classes.fixed_class(FixedKey::EditItemButton, false),
            aria_label: "Edit “{text}”",
            aria_pressed: if editing { "true" } else { "false" },
            onclick: move |_| {
                on_edit.call(Edit::EditItem {
                    path: path.clone(),
                    index: (!editing).then_some(index),
                });
            },
            {icon(PENCIL)}
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
pub fn RemoveItemButton(
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
                on_edit.call(Edit::RemoveItem { path: path.clone(), index });
            },
            {icon(X)}
        }
    }
}

/// The ↑ and ↓ buttons that move one row of a reorderable list, rendered on
/// the row's left, so neither is ever next to its Remove.
///
/// Each moves the row one place, by position as [`RemoveItemButton`] removes
/// one. ↑ is disabled on the first row and ↓ on the last rather than left out,
/// so every row's controls line up and a keyboard user tabbing down the list
/// meets the same buttons in each.
#[component]
pub fn MoveItemButtons(
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
                    if let Some(to) = index.checked_sub(1) {
                        on_edit.call(Edit::MoveItem { path: up_path.clone(), from: index, to });
                    }
                },
                {icon(CHEVRON_UP)}
            }
            button {
                r#type: "button",
                class: classes.fixed_class(FixedKey::MoveRowDownButton, false),
                aria_label: "Move row {ordinal} down",
                disabled: index + 1 == count,
                onclick: move |_| {
                    on_edit.call(Edit::MoveItem { path: path.clone(), from: index, to: index + 1 });
                },
                {icon(CHEVRON_DOWN)}
            }
        }
    }
}
