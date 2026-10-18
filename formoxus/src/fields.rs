//! The built-in fields: one [`Field`](crate::field::Field) per value family.

mod bool_field;
mod choice_field;
mod number_fields;
mod string_field;

pub use bool_field::BoolField;
pub use choice_field::ChoiceField;
pub use number_fields::{Number, NumberField};
pub use string_field::StringField;

/// The field a model type gets unless `form!` names another: `u8` gets a
/// `NumberField<u8>`, `Option<u8>` the same.
///
/// The compile-time half of the type → field registry: `form!` emits settings
/// through [`field_for`](crate::field_kind::field_for), and this is what lets
/// rustc work out which field they belong to, and type them. Implemented only
/// for the types formoxus renders without being told; a newtype or anything
/// else exotic needs its field declared.
#[diagnostic::on_unimplemented(
    message = "`{Self}` has no field that takes settings like `min` or `max_length`",
    label = "settings need a field formoxus knows",
    note = "a struct, list or enum takes none; a newtype or another type of your own needs \
            a field declared for it (`field:` in `form!`, not built yet)"
)]
pub trait DefaultField {
    type Field: crate::field::Field + Default + Send + Sync;
}

impl DefaultField for String {
    type Field = StringField;
}

impl DefaultField for bool {
    type Field = BoolField;
}

macro_rules! default_number_fields {
    ($($t:ty),*) => {$(
        impl DefaultField for $t {
            type Field = NumberField<$t>;
        }
    )*};
}
// The same list as `Number`'s impls and `dispatch!`'s numeric arms.
default_number_fields!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);

impl<T: DefaultField> DefaultField for Option<T> {
    type Field = T::Field;
}
