//! The two error payloads: one per field, one per form.

use std::{error::Error, fmt::Display};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormError(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormAccessError(pub String);

impl Display for FormAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Error for FormAccessError {}

