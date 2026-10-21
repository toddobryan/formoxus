use std::fmt::Debug;

use facet::Facet;
use formoxus_spec::{Attr, AttrValue, Bound, FieldType};

use crate::widgets::{AnyWidget, TextInput};
use crate::{ValidationMessage, field::Field, field_attrs::FieldAttrs};

mod sealed {
    /// Public in a private module: nameable as a supertrait, implementable by
    /// no one outside this crate. That is what seals `Number`.
    pub trait Sealed {}
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a number type `NumberField` supports",
    note = "use i8–i64, u8–u64, f32 or f64; usize and isize are left out on purpose (see `build.rs`)"
)]
pub trait Number:
    sealed::Sealed
    + Copy
    + PartialOrd
    + std::fmt::Display
    + Clone
    + Debug
    + PartialEq
    + for<'f> Facet<'f>
    + 'static
{
    const KIND: FieldType; // Int or Float

    /// This value as the untyped bound `min`/`max` are emitted from.
    fn to_bound(self) -> Bound;

    fn is_nan(&self) -> bool {
        false
    }
}

macro_rules! numbers {
    ($kind:ident: $($t:ty),*) => {$(
        impl sealed::Sealed for $t {}
        impl Number for $t {
            const KIND: FieldType = FieldType::$kind;
            fn to_bound(self) -> Bound {
                Bound::Int(i128::from(self))
            }
        }
    )*};
}
numbers!(Int: i8, i16, i32, i64, u8, u16, u32, u64);

impl sealed::Sealed for f32 {}

impl Number for f32 {
    const KIND: FieldType = FieldType::Float;
    fn to_bound(self) -> Bound {
        Bound::Float(f64::from(self))
    }
    fn is_nan(&self) -> bool {
        f32::is_nan(*self)
    }
}

impl sealed::Sealed for f64 {}

impl Number for f64 {
    const KIND: FieldType = FieldType::Float;
    fn to_bound(self) -> Bound {
        Bound::Float(self)
    }
    fn is_nan(&self) -> bool {
        f64::is_nan(*self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberField<T: Number> {
    /// Inclusive.
    pub min: Option<T>,
    /// Inclusive.
    pub max: Option<T>,
}

/// Hand-written: a derived `Default` would demand `T: Default`, which
/// `Number` does not promise.
impl<T: Number> Default for NumberField<T> {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
        }
    }
}

impl<T: Number> Field for NumberField<T> {
    type Value = T;

    type Context = ();

    /// A text input, not a number input: `type="number"` hands back `""` for
    /// a half-typed value like `-` or `1e`, so `parse` could never say what
    /// was wrong with it.
    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }

    /// `min` and `max`, so the browser checks what `check` does.
    fn widget_attrs(&self) -> FieldAttrs {
        let mut attrs = FieldAttrs::default();
        if let Some(min) = self.min {
            attrs = attrs.with_default(Attr::Min, AttrValue::Bound(min.to_bound()));
        }
        if let Some(max) = self.max {
            attrs = attrs.with_default(Attr::Max, AttrValue::Bound(max.to_bound()));
        }
        attrs
    }

    fn check(&self, value: &Self::Value) -> Vec<ValidationMessage> {
        // NaN compares false to everything, so it would slip past both bounds;
        // a bounded field rejects it outright.
        let below = self.min.is_some_and(|min| value.is_nan() || *value < min);
        let above = self.max.is_some_and(|max| value.is_nan() || *value > max);
        range_message(self.min, self.max, below, above)
            .into_iter()
            .collect()
    }
}

fn range_message<N: std::fmt::Display>(
    min: Option<N>,
    max: Option<N>,
    below: bool,
    above: bool,
) -> Option<ValidationMessage> {
    let message = match (min, max) {
        (Some(min), Some(max)) if below || above => {
            format!("number must be in the range {min} up to (and including) {max}")
        }
        (Some(min), None) if below => format!("number must be at least {min}"),
        (None, Some(max)) if above => format!("number must be at most {max}"),
        _ => return None,
    };
    Some(ValidationMessage(message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    /// `Field::check`, spelled out: googletest's prelude has a `check` too.
    fn messages<T: Number>(field: &NumberField<T>, value: T) -> Vec<String> {
        Field::check(field, &value)
            .into_iter()
            .map(|e| e.0)
            .collect()
    }

    fn between<T: Number>(min: Option<T>, max: Option<T>) -> NumberField<T> {
        NumberField { min, max }
    }

    #[gtest]
    fn no_bounds_never_complains() {
        expect_that!(
            messages(&NumberField::<i64>::default(), i64::MIN),
            is_empty()
        );
    }

    #[gtest]
    fn bounds_are_inclusive() {
        let field = between(Some(1_u32), Some(10));
        expect_that!(messages(&field, 0), len(eq(1)));
        expect_that!(messages(&field, 1), is_empty());
        expect_that!(messages(&field, 10), is_empty());
        expect_that!(messages(&field, 11), len(eq(1)));
    }

    /// One message names the whole range when both ends exist; a one-sided
    /// bound says which side.
    #[gtest]
    fn the_message_names_the_bounds_that_exist() {
        expect_that!(
            messages(&between(Some(1_i32), Some(10)), 11),
            elements_are![eq("number must be in the range 1 up to (and including) 10")]
        );
        expect_that!(
            messages(&between(Some(0_i32), None), -1),
            elements_are![eq("number must be at least 0")]
        );
        expect_that!(
            messages(&between(None, Some(100_i32)), 101),
            elements_are![eq("number must be at most 100")]
        );
    }

    /// NaN compares false to everything, so a bounded field rejects it outright;
    /// with no bound, there is nothing for it to be outside of.
    #[gtest]
    fn nan_passes_an_unbounded_float_and_fails_a_bounded_one() {
        expect_that!(
            messages(&NumberField::<f64>::default(), f64::NAN),
            is_empty()
        );
        expect_that!(messages(&between(Some(0.0), None), f64::NAN), len(eq(1)));
        expect_that!(messages(&between(None, Some(1.0)), f64::NAN), len(eq(1)));
    }

    #[gtest]
    fn infinity_is_compared_like_any_other_float() {
        expect_that!(
            messages(&NumberField::<f64>::default(), f64::INFINITY),
            is_empty()
        );
        expect_that!(
            messages(&between(None, Some(100.0)), f64::INFINITY),
            len(eq(1))
        );
        expect_that!(
            messages(&between(None, Some(100.0)), f64::NEG_INFINITY),
            is_empty()
        );
    }

    #[gtest]
    fn kind_says_int_or_float() {
        expect_that!(<u8 as Number>::KIND, eq(FieldType::Int));
        expect_that!(<f32 as Number>::KIND, eq(FieldType::Float));
    }

    /// Not a number input, for an int or a float: see `default_widget`.
    #[gtest]
    fn the_default_widget_is_a_text_input_optional_or_not() {
        for optional in [false, true] {
            expect_that!(
                between::<i32>(None, None).default_widget(optional),
                eq(&AnyWidget::from(TextInput))
            );
            expect_that!(
                between::<f64>(None, None).default_widget(optional),
                eq(&AnyWidget::from(TextInput))
            );
        }
    }
}
