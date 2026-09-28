//! Locating a field by its ACCESSIBLE NAME.
//!
//! **Not `get_by_label`.** The two are different, and the difference is exactly
//! where formoxus's `aria-hidden` required marker lives:
//!
//! - `get_by_label` matches the `<label>`'s TEXT, which still contains the ` *`.
//!   An exact `get_by_label("Name")` therefore times out.
//! - `get_by_role` matches the computed accessible name, which skips
//!   `aria-hidden` subtrees, so `"Name"` matches.
//!
//! Reaching for the first makes the `aria-hidden` look broken when it is working.
//! These helpers exist so that mistake is not available.

use playwright_rs::Locator;
use playwright_rs::protocol::locator::{AriaRole, GetByRoleOptions};

use crate::Session;

fn named(s: &Session, role: AriaRole, name: &str) -> Locator {
    // `GetByRoleOptions` is `#[non_exhaustive]`, so it has to be built from
    // `default()` rather than with a struct literal.
    let mut o = GetByRoleOptions::default();
    o.name = Some(name.to_string());
    o.exact = Some(true);
    s.page.get_by_role(role, Some(o))
}

/// A single-line text input, by accessible name.
pub fn textbox(s: &Session, name: &str) -> Locator {
    named(s, AriaRole::Textbox, name)
}

/// A number input. `type="number"` has role `spinbutton`, NOT `textbox` — worth
/// knowing, because the `bounds` form is the only one that uses it and a
/// `textbox` lookup there fails for a reason that looks like a missing label.
pub fn spinbutton(s: &Session, name: &str) -> Locator {
    named(s, AriaRole::Spinbutton, name)
}

/// A checkbox, by accessible name.
pub fn checkbox(s: &Session, name: &str) -> Locator {
    named(s, AriaRole::Checkbox, name)
}

/// A `<select>`, by accessible name. Role is `combobox` for a single-select.
pub fn combobox(s: &Session, name: &str) -> Locator {
    named(s, AriaRole::Combobox, name)
}

/// One radio in a group, by its own accessible name — the choice's display text,
/// not the group's legend.
pub fn radio(s: &Session, name: &str) -> Locator {
    named(s, AriaRole::Radio, name)
}

/// The submit button every test form carries.
pub fn submit(s: &Session) -> Locator {
    s.page.locator("button[type=\"submit\"]")
}
