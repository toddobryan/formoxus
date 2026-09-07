//! A runtime-reflection alternative to formoxus's derive macros: build a form
//! from a model's `Facet` shape instead of from a hand-written form struct.
//!
//! This path and the `#[derive(Form)]` path currently coexist. They share
//! [`crate::error`] and nothing else — in particular this module has its own
//! [`fields::FormField`], distinct from [`crate::fields::FormField`]: the derive
//! path's is generic over a `FromStr` value type and lives in a `Store`, while
//! this one carries its own name/label and converts through facet's vtables, so
//! a model needs only `derive(Facet)`. Whether they stay side by side or one
//! replaces the other is still undecided — see `REFLECT_PLAN.md`.

pub mod build;
pub mod fields;
pub mod form;
pub mod members;
pub mod widgets;

// A flat root, so `use formoxus::reflect::*` (and the test modules' `use
// crate::reflect::*`) reaches the whole vocabulary without knowing which module
// each name lives in.
pub use crate::error::{FieldError, FormError};
pub use fields::{FieldValue, FormField};
pub use form::{Form, FormState, empty_form, form_for, use_form, use_form_values};
pub use members::{
    Edit, FieldSet, FormMember, ListSet, RenderCtx, ValuesByPath, VariantChoice, VariantSet,
    model_path,
};

// The one crate-internal item the test modules reach for directly.
#[cfg(test)]
pub(crate) use widgets::ABSENT_DISPLAY;

#[cfg(test)]
mod tests;
