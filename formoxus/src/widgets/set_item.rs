//! What shows one item of a set: [`ItemText`] by default, or a dev's own
//! component named by `item:` in `form!` (`SETS_PLAN.md` box 9).
//!
//! It shows the item's TEXT only. The set still renders the `<li>` around it,
//! the item's own errors, and Edit and Remove, so a dev's chip can neither
//! drop the buttons nor lose their names ("Edit “rust”").

use std::rc::Rc;
use std::{fmt::Debug, ptr::fn_addr_eq};

use dioxus::prelude::*;

use crate::classes::Classes;
use crate::nodes::ValuesStore;

/// What an item's view is handed: [`ItemText`], or a dev's own component
/// named by `item:`.
///
/// No typed value: a set holds only single values (a string, a number, a
/// unit enum, a newtype over one), so `text` already is the value, written
/// out: a newtype's inner value, a unit enum's variant name.
#[derive(Debug, Props, Clone, PartialEq)]
pub struct SetItemProps {
    /// The item as the user sees it: its value as its leaf would show it, or
    /// its choice's label when the item's field has choices.
    pub text: String,
    /// The item's own path (`tags.#1`), whose leaf in `values` holds `text`.
    pub path: String,
    /// The item's place in the set as it is sorted now.
    pub index: usize,
    /// Whether the item is the one being edited, its value in the draft.
    pub editing: bool,
    /// The whole form's values, for a view that shows more than its own item.
    pub values: ValuesStore,
    /// The form's resolved classes, so a dev's view can follow the app's
    /// preset.
    pub classes: Rc<Classes>,
}

/// A set's item view: the component that shows an item's text.
///
/// A newtype over a `fn` pointer for the reason [`ItemAdder`] is one:
/// [`FieldSpec`] derives `PartialEq`, and a derived comparison of a `fn`
/// pointer trips `unpredictable_function_pointer_comparisons`.
///
/// [`ItemAdder`]: super::ItemAdder
/// [`FieldSpec`]: crate::form::FieldSpec
#[derive(Clone, Copy)]
pub struct ItemView(pub fn(SetItemProps) -> Element);

impl Default for ItemView {
    fn default() -> Self {
        Self(ItemText)
    }
}

// Not derived: a `fn` pointer's `Debug` is its address, which names nothing.
impl Debug for ItemView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ItemView(..)")
    }
}

impl PartialEq for ItemView {
    fn eq(&self, other: &Self) -> bool {
        fn_addr_eq(self.0, other.0)
    }
}

/// The default [`ItemView`]: the item's text, and nothing around it.
#[component]
pub fn ItemText(props: SetItemProps) -> Element {
    rsx! { "{props.text}" }
}
