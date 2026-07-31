use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::FieldError;

/// One field's state: where it started (`initial`), its current value
/// (`None` = empty/unfilled), and any errors currently attached to it.
///
/// `value` carries the emptiness, so `T` is always the field's *cleaned* type
/// (`FormField<String>`, not `FormField<Option<String>>`).
#[derive(Clone, Debug, Default, Serialize, Deserialize, Store)]
pub struct FormField<T: Clone> {
    pub initial: Option<T>,
    pub value: Option<T>,
    pub errors: Vec<FieldError>,
}

impl<T: Clone> FormField<T> {
    /// A pristine field seeded with a concrete value — a required field in edit
    /// mode (`initial == value`, no errors).
    pub fn with_value(value: T) -> Self {
        Self {
            initial: Some(value.clone()),
            value: Some(value),
            errors: Vec::new(),
        }
    }

    /// A pristine field seeded from an already-optional value — an optional field.
    pub fn with_optional(value: Option<T>) -> Self {
        Self {
            initial: value.clone(),
            value,
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
            Some(v) => Some(v.clone()),
            None => {
                self.errors
                    .push(FieldError("This field is required.".to_string()));
                None
            }
        }
    }

    /// Clean an **optional** field: yield the value as-is — `None` is not an error.
    pub fn optional(&self) -> Option<T> {
        self.value.clone()
    }
}
