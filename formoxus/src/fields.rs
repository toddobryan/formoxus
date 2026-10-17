//! The built-in fields: one [`Field`](crate::field::Field) per value family.

mod bool_field;
mod choice_field;
mod number_fields;
mod string_field;

pub use bool_field::BoolField;
pub use choice_field::ChoiceField;
pub use number_fields::{Number, NumberField};
pub use string_field::StringField;
