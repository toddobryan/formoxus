use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, str::FromStr};

use crate::error::FieldError;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Store)]
pub enum FieldValue<T: Debug + Clone + FromStr> {
    Empty,
    Valid(T),
    Invalid {
        raw: String,
        error: FieldError,
    },
}

// Hand-written rather than `#[derive(Default)]`: the derive macro always adds
// a blanket `T: Default` bound to a generic type's impl, even though the
// `Empty` variant it defaults to carries no `T` at all — a well-known
// limitation (rust-lang/rust#26925), not something specific to this type.
// That spurious bound would rule out a `T` with no sensible "empty" value of
// its own (e.g. `Ref<Source>` — a record reference has no default row to
// point to), which is exactly the kind of type a field needs to support.
impl<T: Debug + Clone + FromStr> Default for FieldValue<T> {
    fn default() -> Self {
        FieldValue::Empty
    }
}

impl<T: Debug + Clone + FromStr> FieldValue<T> {
    pub fn is_empty(&self) -> bool {
        match self {
            FieldValue::Empty => true,
            _ => false,
        }
    }
}

/// One field's state: where it started (`initial`), its current value
/// (`None` = empty/unfilled), and any errors currently attached to it.
///
/// `value` carries the emptiness, so `T` is always the field's *cleaned* type
/// (`FormField<String>`, not `FormField<Option<String>>`).
#[derive(Clone, Debug, Serialize, Deserialize, Store)]
pub struct FormField<T: Debug + Clone + FromStr> {
    pub initial: Option<T>,
    pub value: FieldValue<T>,
    pub errors: Vec<FieldError>,
}

// Hand-written for the same reason as `FieldValue`'s: `#[derive(Default)]`
// would add a spurious `T: Default` bound that a `Ref<Source>`-typed field
// (no sensible default record to point to) could never satisfy, even though
// nothing here actually needs one — `initial: None`, `value:
// FieldValue::Empty`, and an empty `errors` are all buildable for any `T`.
impl<T: Debug + Clone + FromStr> Default for FormField<T> {
    fn default() -> Self {
        Self {
            initial: None,
            value: FieldValue::default(),
            errors: Vec::new(),
        }
    }
}

impl<T: Clone + Debug + FromStr> FormField<T> {
    /// A pristine field seeded with a concrete value — a required field in edit
    /// mode (`initial == value`, no errors).
    pub fn with_value(value: T) -> Self {
        Self {
            initial: Some(value.clone()),
            value: FieldValue::Valid(value),
            errors: Vec::new(),
        }
    }

    /// A pristine field seeded from an already-optional value — an optional field.
    pub fn with_optional(value: Option<T>) -> Self {
        Self {
            initial: value.clone(),
            value: match value {
                None => FieldValue::Empty,
                Some(t) => FieldValue::Valid(t),
            },
            errors: Vec::new(),
        }
    }

    /// Drop this field's errors. Called at the start of each validation pass so
    /// errors don't accumulate across re-validations.
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    /// Clean a **required** field: yield the value, or stamp a "required" error
    /// (in place) and yield `None`.
    pub fn required(&mut self) -> Option<T> {
        match &self.value {
            FieldValue::Empty => {
                self.errors
                    .push(FieldError("This field is required.".to_string()));
                None
            }
            FieldValue::Valid(t) => Some(t.clone()),
            FieldValue::Invalid { raw: _, error: _ } => None,
        }
    }

    /// Clean an **optional** field: yield the value as-is — `None` is not an error.
    pub fn optional(&self) -> Option<T> {
        match &self.value {
            FieldValue::Empty => None,
            FieldValue::Valid(t) => Some(t.clone()),
            FieldValue::Invalid { raw: _, error: _ } => None,
        }
    }

    /// A validate-time error, or an unparseable `Invalid` value (which carries its
    /// own parse error). The `Invalid` branch is why this isn't just `!errors.is_empty()`.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty() || matches!(self.value, FieldValue::Invalid { .. })
    }

    pub fn add_error(&mut self, error_string: &str) {
        self.errors.push(FieldError(error_string.to_string()));
    }
}
