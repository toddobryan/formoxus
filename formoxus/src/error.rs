use std::fmt::{self, Debug, Display};

use serde::{Deserialize, Serialize};

use crate::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ValidationMessage(pub String);

impl From<&str> for ValidationMessage {
    fn from(value: &str) -> Self {
        ValidationMessage(value.to_string())
    }
}

impl From<String> for ValidationMessage {
    fn from(value: String) -> Self {
        ValidationMessage(value)
    }
}

pub struct ValidationError<T> {
    pub(crate) path: Option<Path<T>>,
    pub(crate) message: ValidationMessage,
}

// Hand-written rather than derived, for the reason `Path`'s are: a derive would
// demand `T: Clone`/`T: Debug`/`T: PartialEq`, because it cannot see that `T`
// appears only inside `Path<T>` — which holds no `T` either. Every real model
// happens to satisfy those bounds, so the derive compiled; it would only bite
// generic code that needs a `ValidationError<T>` without the same bounds on `T`.
impl<T> Clone for ValidationError<T> {
    fn clone(&self) -> Self {
        Self {
            path: self.path,
            message: self.message.clone(),
        }
    }
}

impl<T> Debug for ValidationError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ValidationError")
            .field("path", &self.path)
            .field("message", &self.message)
            .finish()
    }
}

impl<T> PartialEq for ValidationError<T> {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.message == other.message
    }
}

impl<T> Eq for ValidationError<T> {}

impl<T> ValidationError<T> {
    pub fn at(path: Path<T>, message: impl Into<ValidationMessage>) -> Self {
        ValidationError {
            path: Some(path),
            message: message.into(),
        }
    }

    pub fn form(message: impl Into<ValidationMessage>) -> Self {
        ValidationError {
            path: None,
            message: message.into(),
        }
    }

    pub fn message(&self) -> ValidationMessage {
        self.message.clone()
    }

    pub fn message_string(&self) -> String {
        self.message.0.clone()
    }
}

/*
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormError(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldError(pub String);
*/

/// A caller reached for a form path that doesn't exist, or a variant name that
/// isn't in the enum — e.g. [`crate::FormState::choose_variant`]. Distinct
/// from [`ValidationError`]/[`ValidationMessage`], which are *validation* results the user
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
