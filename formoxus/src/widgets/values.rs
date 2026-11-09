//! The bridge between a widget and the value store.
//!
//! `pub` for [`get_errors`], which a custom widget reads like any built-in one.
//! Reading and writing a VALUE is crate-private: a widget does both through a
//! [`FieldHandle`](crate::widgets::FieldHandle), so that a write cannot happen
//! without the form hearing of it.

use dioxus::prelude::*;

use crate::{
    error::ValidationMessage,
    nodes::{ErrorsStore, ValuesStore},
};

/// This path's current raw value, or `""` if the map has no entry for it.
///
/// Crate-private: a widget reads through
/// [`FieldHandle::current`](crate::widgets::FieldHandle::current), which calls
/// this. A path the schema has but the map doesn't is normal, not an error: a
/// variant chosen after mount reveals leaves that were never populated, and an
/// absent key reads as empty, which is the same "empty IS absence" rule
/// `distribute_values` follows.
pub(crate) fn get_current(path: &str, values: ValuesStore) -> String {
    let slot = values.get_unchecked(path.to_string());
    slot.try_read().map(|v| v.clone()).unwrap_or_default()
}

/// This path's current errors, or an empty `Vec` if it has none.
///
/// `None`, no key and an empty `Vec` all read as empty, so a widget never sees
/// the store's `Option`.
///
/// **Reads one key, not the map.** `errors.read()` would subscribe the caller to
/// every path, so one field's error would wake every widget. A key that is not
/// there yet still wakes its reader when `insert` first writes it.
pub fn get_errors(path: &str, errors: ErrorsStore) -> Vec<ValidationMessage> {
    let slot = errors.get_unchecked(path.to_string());
    slot.try_read()
        .ok()
        .and_then(|messages| messages.clone())
        .unwrap_or_default()
}

/// Write this path's raw value back, inserting the key if it wasn't there.
///
/// A bare write: it tells the form nothing, so no live check follows and a
/// clean result is not made stale. Crate-private for that reason. A widget
/// writes through a [`FieldHandle`](crate::widgets::FieldHandle), which calls
/// this and then reports the write. The form calls it directly only where it
/// settles the errors itself, as [`absorb`](crate::Form::absorb) (the reply
/// carries its own) and [`reset`](crate::Form::reset) (which clears them) do.
/// Inserts the key when it is missing, and sets it otherwise, so its existing
/// subscribers fire.
pub(crate) fn write_value(path: &str, mut values: ValuesStore, raw: String) {
    let populated = values.peek().contains_key(path);
    if populated {
        values.get_unchecked(path.to_string()).set(raw);
    } else {
        values.insert(path.to_string(), raw);
    }
}
