//! Value-level field validators — parameterized checks on a *present* value.
//!
//! Each validator is a trait implemented ONLY for the field-value types it makes
//! sense for. `#[derive(Form)]` emits a call bounded by that trait, so applying a
//! validator to an incompatible field type is a **compile error at the derive
//! site**: the trait bound *is* the type-compatibility check — sound, alias-aware,
//! and with no syntactic type-guessing in the macro. `#[diagnostic::on_unimplemented]`
//! (stable since Rust 1.78) turns the raw "trait not satisfied" into a message that
//! names the offending attribute.
//!
//! Adding a validator stays fully local: define its trait (+ its `on_unimplemented`
//! message), impl it for the compatible value types, and emit one call from the
//! derive. There is no central validator×type matrix to maintain — the set of impls
//! *is* the "which types does this apply to" answer.
//!
//! These validate a *present* value being unacceptable — NOT presence. Presence is
//! type-driven (a non-`Option` field is required; a blank/whitespace-only string maps
//! to `Empty` and fails presence on its own). So there is deliberately no `non_empty`
//! validator: that was only ever "a required text field," which the type already says.
//!
//! Validators run on the cleaned value and return `Result<(), FieldError>` rather
//! than mutating, so a validator stays pure and owns its message; the generated code
//! collects the error onto the field (multiple validators on one field accumulate).
//!
//! # What the derive emits
//!
//! For `#[form(min_len = 8)] name: String`, after the field's presence step, on the
//! cleaned value `v: &String`:
//!
//! ```ignore
//! if let Err(e) = ::formoxus::validators::MinLen::min_len(v, 8usize) {
//!     name_errors.push(e);
//! }
//! ```

use crate::error::FieldError;

/// `#[form(min_len = N)]` — a text field with a minimum character count.
///
/// A *parameterized* validator: the derive threads the attribute's literal through as
/// the `min` argument. A numeric bound (`#[form(at_least = N)]` on an `i32`) would look
/// identical except the trait is impl'd for the numeric types instead — that impl set is
/// the whole compatibility story. Implemented only for text types, so `#[form(min_len =
/// …)]` on, say, an `i32` field fails to compile with the message below rather than
/// being silently ignored.
#[diagnostic::on_unimplemented(
    message = "`#[form(min_len = …)]` can only be applied to text fields (`String`)",
    label = "`min_len` is not valid for a field of this type"
)]
pub trait MinLen {
    fn min_len(&self, min: usize) -> Result<(), FieldError>;
}

impl MinLen for String {
    fn min_len(&self, min: usize) -> Result<(), FieldError> {
        if self.chars().count() < min {
            Err(FieldError(format!("Must be at least {min} characters.")))
        } else {
            Ok(())
        }
    }
}
