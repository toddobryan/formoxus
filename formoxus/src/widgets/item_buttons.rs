//! The buttons that add, remove, move and edit the items of a list or a set.
//!
//! None reads the value store. Adding or dropping an item changes the form's
//! SHAPE, and moving one changes its order, so all any of them does is call
//! its bound field, which puts an [`Edit`](crate::nodes::Edit) on the wire,
//! and let [`ListNode`](crate::nodes::ListNode) or
//! [`SetNode`](crate::nodes::SetNode) rebuild itself.
//!
//! Add is the one a dev can replace: `adder:` in `form!` names a component
//! taking [`AddItemProps`], held as a [`ItemAdder`]. [`AddItemButton`] is a
//! list's default and [`DraftAdder`] a set's.

use std::ops::Deref;
use std::{fmt::Debug, ptr::fn_addr_eq};

use dioxus::prelude::*;
use formoxus_spec::FixedKey;

use crate::bound_field::{BoundField, Items, List, Set};

use super::icons::{CHECK, CHEVRON_DOWN, CHEVRON_UP, PENCIL, PLUS, X, icon};

/// The set item being edited, as a set's adder sees it in
/// [`AddItemProps::editing`]: its text, so the adder can name what Add would
/// now replace (`Replace "rust"`), and [`cancel`](Self::cancel), which stops
/// editing.
///
/// It holds the set's own bound field because the adder's is a
/// `BoundField<Items>`, which cannot edit an item, and a list's adder must
/// not be able to. Only a set makes one, so only a set's adder can cancel.
/// Only formoxus builds one.
#[derive(Debug, Clone, PartialEq)]
pub struct Editing {
    /// The item as the user sees it, `rust` in `Replace "rust"`.
    text: String,
    /// The set the item belongs to, for `cancel`.
    set: BoundField<Set>,
}

impl Editing {
    pub(crate) fn new(text: String, set: BoundField<Set>) -> Self {
        Self { text, set }
    }

    /// The edited item as the user sees it, as its view's
    /// [`text`](crate::widgets::SetItemProps::text) has it.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Stop editing, leaving the item as it was, and empty the draft: the
    /// set's [`edit_item`](crate::bound_field::BoundField::<Set>::edit_item)
    /// with `None`.
    pub fn cancel(&self) {
        self.set.edit_item(None);
    }
}

/// What a list's or a set's adder is handed: [`AddItemButton`],
/// [`DraftAdder`], or a dev's own component named by `adder:` in `form!`.
///
/// An adder appends an item by calling `add` on its bound field
/// ([`field`](Self::field)), which addresses the edit to the list or set
/// itself. With `Some(ItemValue::new(item))` the item arrives filled in, and
/// focus is left alone, so a user picking several choices in a row stays on
/// the adder. With `None` it means "add what the adder holds": in a list a
/// blank item, which takes focus, and in a set its `draft`.
#[derive(Debug, Props, Clone, PartialEq)]
pub struct AddItemProps {
    pub(crate) field: BoundField<Items>,
    /// How many items the list or set has now, for an adder that stops at a
    /// maximum.
    pub item_count: usize,
    /// The most items it may have (`max_items:`), so an adder can stop
    /// there as Add does: at `item_count == max_items`, one more is too many.
    pub max_items: Option<usize>,
    /// A set's draft, rendered: the blank item, in the item type's own
    /// widget, that `Edit::AddItem { value: None }` makes an item. `None` for
    /// a list. An adder that sends its own value (a search-to-add) can leave
    /// it out.
    pub draft: Option<Element>,
    /// The item being edited, whose value the draft holds, so Add would
    /// replace it: [`DraftAdder`] then says Replace, named with
    /// [`Editing::text`] (`Replace "rust"`), and offers Cancel through
    /// [`Editing::cancel`]. `None` when nothing is being edited, and always
    /// for a list.
    pub editing: Option<Editing>,
}

impl AddItemProps {
    /// The bound field, owned, for an event handler, which must be
    /// `'static`. For a read, the props deref to it.
    pub fn field(&self) -> BoundField<Items> {
        self.field.clone()
    }
}

impl Deref for AddItemProps {
    type Target = BoundField<Items>;

    fn deref(&self) -> &Self::Target {
        &self.field
    }
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

/// The widget that appends an item to a list, and the default [`ItemAdder`].
///
/// Like [`VariantSelect`](super::VariantSelect), it reads nothing from the
/// value store — adding an item is a change to the form's *shape*, so all it
/// does is call [`add`](crate::bound_field::BoundField::<Items>::add), which
/// puts an [`Edit`](crate::nodes::Edit) on the wire. `type="button"` is load-bearing: inside a `<form>` a bare `<button>`
/// defaults to `type="submit"`, so omitting it would submit the form instead of
/// adding an item.
#[component]
pub fn AddItemButton(props: AddItemProps) -> Element {
    let AddItemProps {
        field,
        item_count,
        max_items,
        ..
    } = props;
    let full = at_most(item_count, max_items);
    rsx! {
        button {
            r#type: "button",
            class: field.classes().fixed_class(FixedKey::AddItemButton, false),
            // Disabled, not hidden, so the layout holds still and the reason
            // can be given.
            disabled: full.is_some(),
            title: full,
            onclick: move |_| field.add(None),
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
/// While an item is being edited (box 8), Add is Replace, named for the item
/// it replaces (`Replace "rust"`), and Cancel, or Escape in the draft, stops
/// the edit and leaves the item as it was. Not "Save": a form's own submit
/// button is often `save`, and two buttons would then share a name. Named by
/// the item being replaced, not by the draft's text, so the name stays true
/// as the user types. The visible word begins the name (WCAG 2.5.3), so a
/// voice-control user can say "click Replace".
///
/// After Add or Cancel, focus goes back to the draft, so a user adding
/// several items types one after another. The draft is rendered in the same
/// place, so the element is the same one, now under the next key.
#[component]
pub fn DraftAdder(props: AddItemProps) -> Element {
    let AddItemProps {
        field,
        draft,
        editing,
        item_count,
        max_items,
        ..
    } = props;
    let draft_id = format!("{}-draft", field.id());
    let add = {
        let field = field.clone();
        move || field.add(None)
    };
    let cancel = {
        let editing = editing.clone();
        move || {
            if let Some(e) = &editing {
                e.cancel();
            }
        }
    };
    let on_key = (add.clone(), cancel.clone());
    let focus_draft = move || {
        document::eval(&format!(
            "document.getElementById('{draft_id}')?.querySelector('input, select, textarea')?.focus()"
        ));
    };
    let after_cancel = focus_draft.clone();
    let replacing = editing
        .as_ref()
        .map(|e| format!("Replace \"{}\"", e.text()));
    let is_editing = editing.is_some();
    // Full only for adding: a replacement leaves the count as it is.
    let full = at_most(item_count, max_items).filter(|_| !is_editing);
    let can_add = full.is_none();
    rsx! {
        div { class: field.classes().fixed_class(FixedKey::SetAdder, false),
        div {
            id: "{field.id()}-draft",
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::Enter {
                    e.prevent_default();
                    if can_add {
                        (on_key.0)();
                    }
                } else if is_editing && e.key() == Key::Escape {
                    e.prevent_default();
                    (on_key.1)();
                }
            },
            {draft}
        }
        button {
            r#type: "button",
            class: field.classes().fixed_class(FixedKey::AddItemButton, false),
            aria_label: replacing,
            disabled: !can_add,
            title: full,
            onclick: move |_| {
                add();
                focus_draft();
            },
            if is_editing {
                {icon(CHECK)}
                "Replace"
            } else {
                {icon(PLUS)}
                "Add"
            }
        }
        if is_editing {
            button {
                r#type: "button",
                class: field.classes().fixed_class(FixedKey::Button(formoxus_spec::ButtonType::Cancel), false),
                onclick: move |_| {
                    cancel();
                    after_cancel();
                },
                "Cancel"
            }
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
    field: BoundField<Set>,
    index: usize,
    /// The item as the user sees it, for the button's name.
    text: String,
    /// Whether this item is the one being edited.
    editing: bool,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: field.classes().fixed_class(FixedKey::EditItemButton, false),
            aria_label: "Edit \"{text}\"",
            aria_pressed: if editing { "true" } else { "false" },
            onclick: {
                let field = field.clone();
                move |_| field.edit_item((!editing).then_some(index))
            },
            {icon(PENCIL)}
        }
    }
}

/// The widget that drops one item from a list or a set.
///
/// Addressed by POSITION, not by the item's key: the list is what applies the
/// edit and it works in terms of `items`, so a position is what it can act on
/// directly. Keys identify an item across edits; a position locates one at an
/// instant, which is all a click needs to say.
///
/// Named by `text` when there is one, `Remove "rust"`, as a set's is: a set
/// is sorted, so "Remove item 2" names nothing a user can see. A list's item
/// has no one text to name it by, so it is named by its position, which an
/// `<ol>` numbers on screen by default and a screen reader announces.
#[component]
pub fn RemoveItemButton(
    field: BoundField<Items>,
    index: usize,
    /// The item as the user sees it, for the button's name.
    text: Option<String>,
    /// At the list's or set's `min_items`, where one fewer is too few.
    #[props(default)]
    disabled: bool,
) -> Element {
    // 1-based for humans: this is the only place an item's position is spoken
    // aloud, and it is never used as a path segment.
    let label = match text {
        Some(text) => format!("Remove \"{text}\""),
        None => format!("Remove item {}", index + 1),
    };
    rsx! {
        button {
            r#type: "button",
            class: field.classes().fixed_class(FixedKey::RemoveItemButton, false),
            aria_label: "{label}",
            disabled,
            onclick: move |_| field.remove(index),
            {icon(X)}
        }
    }
}

/// The ↑ and ↓ buttons that move one item of a reorderable list, rendered on
/// the item's left, so neither is ever next to its Remove.
///
/// Each moves the item one place, by position as [`RemoveItemButton`] removes
/// one. ↑ is disabled on the first item and ↓ on the last rather than left out,
/// so every item's controls line up and a keyboard user tabbing down the list
/// meets the same buttons in each.
#[component]
pub fn MoveItemButtons(
    field: BoundField<List>,
    index: usize,
    /// How many items the list has, which is what says this one is the last.
    count: usize,
) -> Element {
    let ordinal = index + 1;
    rsx! {
        div { class: field.classes().fixed_class(FixedKey::MoveItemButtons, false),
            button {
                r#type: "button",
                class: field.classes().fixed_class(FixedKey::MoveItemUpButton, false),
                aria_label: "Move item {ordinal} up",
                disabled: index == 0,
                onclick: {
                    let field = field.clone();
                    move |_| {
                        if let Some(to) = index.checked_sub(1) {
                            field.move_item(index, to);
                        }
                    }
                },
                {icon(CHEVRON_UP)}
            }
            button {
                r#type: "button",
                class: field.classes().fixed_class(FixedKey::MoveItemDownButton, false),
                aria_label: "Move item {ordinal} down",
                disabled: index + 1 == count,
                onclick: {
                    let field = field.clone();
                    move |_| field.move_item(index, index + 1)
                },
                {icon(CHEVRON_DOWN)}
            }
        }
    }
}

/// Why Add is disabled, for its `title`, when `count` items already reach
/// `max`; `None` while there is room.
fn at_most(count: usize, max: Option<usize>) -> Option<String> {
    let max = max?;
    (count >= max).then(|| {
        if max == 1 {
            "At most 1 item".to_string()
        } else {
            format!("At most {max} items")
        }
    })
}
