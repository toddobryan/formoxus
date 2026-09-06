//! A runtime-reflection alternative to formoxus's derive macros: build a form
//! from a model's `Facet` shape instead of from a hand-written form struct.

pub mod build;
pub mod error;
pub mod fields;
pub mod form;
pub mod members;

// A flat root, so `use facet_form_spike::*` (and the test modules' `use crate::*`)
// reaches the whole vocabulary without knowing which module each name lives in.
pub use error::{FieldError, FormError};
pub use fields::{FieldValue, FormField};
pub use form::{Form, empty_form, form_for};
pub use members::{FieldSet, FormMember, ListSet, VariantChoice, VariantSet};

// The one crate-internal item the test modules reach for directly.
#[cfg(test)]
pub(crate) use members::ABSENT_DISPLAY;

#[cfg(test)]
mod tests;
