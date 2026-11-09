//! The bridge between a widget and the value store.
//!
//! `pub` for [`get_errors`], which a custom widget reads like any built-in one.
//! Reading and writing a VALUE is crate-private: a widget does both through a
//! [`FieldHandle`](crate::widgets::FieldHandle), so that a write cannot happen
//! without the form hearing of it.

use dioxus::prelude::*;

use crate::{error::ValidationMessage, nodes::ErrorsStore};

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
