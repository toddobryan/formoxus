//! The bridge between a control and the value store.
//!
//! `pub` because a custom control has to read and write like any built-in one,
//! and the insert-vs-set distinction is not something each one should rediscover.

use dioxus::prelude::*;

use crate::ValuesByPath;

/// This path's current raw value, or `""` if the map has no entry for it.
///
/// `pub` because a `custom(…)` input lives in the CONSUMING crate and needs
/// exactly what the built-in controls use — without it, every custom control
/// would reimplement the missing-key rule and get it subtly wrong. A path the
/// schema has but the map doesn't is normal, not an error: a variant chosen
/// after mount reveals leaves that were never populated, and an absent key
/// reads as empty, which is the same "empty IS absence" rule `apply_leaves`
/// follows.
pub fn get_current(path: &str, values: ValuesByPath) -> String {
    let slot = values.get_unchecked(path.to_string());
    slot.try_read().map(|v| v.clone()).unwrap_or_default()
}

/// Write this path's raw value back, inserting the key if it wasn't there.
///
/// `pub` for the same reason as [`get_current`]: a custom control has to be able
/// to write, and the insert-vs-set distinction is not something each one should
/// have to rediscover.
pub fn write_value(path: &str, mut values: ValuesByPath, raw: String) {
    let populated = values.peek().contains_key(path);
    if populated {
        values.get_unchecked(path.to_string()).set(raw);
    } else {
        values.insert(path.to_string(), raw);
    }
}
