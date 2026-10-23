use std::fmt::{self, Debug, Display};

use serde::{Deserialize, Serialize};

use crate::path::Path;

/// One message telling the user what to fix, written for display as is.
///
/// The single error type a form stores, wherever the message came from: a
/// field that failed to parse or was left empty while required, a field's own
/// checks, a whole-model validator (by way of [`ValidationError`]), or the
/// server through [`Form::push_error`](crate::Form::push_error). Whether it
/// shows beside a field or beneath the form depends on which list holds it,
/// not on the message, which is why there is one type rather than a field
/// error and a form error.
///
/// It crosses the wire inside [`FormErrors`](crate::FormErrors), and a
/// newtype over a `String` serializes as the bare string. Build one from a
/// `&str` or `String` with `.into()`.
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

/// One complaint from a form's whole-model validator: a message, and optionally
/// the field it belongs beside.
///
/// A validator is the `validator:` entry in [`form!`](macro@crate::form) (or
/// [`FormSpec::with_validator`](crate::FormSpec::with_validator)), a
/// `fn(&T) -> Vec<ValidationError<T>>`. It runs last in `validate`, only once
/// every field has parsed and the model has been built, so it sees a real `T`
/// and is the place for checks that span fields — "the two passwords match",
/// "at least one answer is correct". An empty `Vec` means the model passed.
///
/// `T` is the form's **model type**, the same `T` as in `FormSpec<T>`. It
/// appears only in the [`Path<T>`], which is what stops a validator from
/// pointing at a field of some other form.
///
/// Where the message shows up:
/// - [`form`](Self::form): at form level, beneath the fields.
/// - [`at`](Self::at): beside the field the path names.
/// - an [`at`](Self::at) path no rendered field claims (an unchosen enum
///   variant's field, say) falls back to form level, prefixed with the path,
///   rather than being dropped.
///
/// ```ignore
/// fn passwords_match(form: &ChangePassword) -> Vec<ValidationError<ChangePassword>> {
///     if form.new_password == form.confirm {
///         vec![]
///     } else {
///         vec![ValidationError::at(path!(ChangePassword.confirm), "Passwords don't match")]
///     }
/// }
/// ```
pub struct ValidationError<T> {
    /// `None` for a form-level error. Turned into its string form and dropped
    /// as soon as `validate` routes the error, so it is never stored.
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
    /// An error shown beside the field `path` names. Build the path with
    /// [`path!`](macro@crate::path) so a typo is a compile error:
    /// `ValidationError::at(path!(Model.field), "…")`.
    pub fn at(path: Path<T>, message: impl Into<ValidationMessage>) -> Self {
        ValidationError {
            path: Some(path),
            message: message.into(),
        }
    }

    /// An error about the form as a whole, shown beneath the fields rather than
    /// beside any one of them.
    pub fn form(message: impl Into<ValidationMessage>) -> Self {
        ValidationError {
            path: None,
            message: message.into(),
        }
    }

    /// The message, cloned.
    pub fn message(&self) -> ValidationMessage {
        self.message.clone()
    }

    /// The message as a plain `String`, cloned.
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
