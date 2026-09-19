use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormError(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError(pub String);

/// A caller reached for a form path that doesn't exist, or a variant name that
/// isn't in the enum — e.g. [`crate::Form::choose_variant`]. Distinct
/// from [`FieldError`]/[`FormError`], which are *validation* results the user
/// can fix by typing: this one means either a bug or a hand-crafted request, so
/// it percolates up to an `ErrorBoundary` rather than rendering beside a field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormAccessError(pub String);

impl Display for FormAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl std::error::Error for FormAccessError {}
