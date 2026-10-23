use std::fmt::Debug;

use facet::{Facet, Peek};

use crate::{
    ValidationMessage,
    field::Field,
    widgets::{AnyWidget, Choice, Select},
};

/// A value that must be one of a fixed list, typed: a `ChoiceField<u8>`
/// offers `u8`s, and membership compares `u8` to `u8`.
///
/// Django's `TypedChoiceField`. A string-valued choice field makes no sense
/// in Rust, where the value already has a type.
///
/// **The strings a widget writes are derived, never written by hand.** Each
/// value is formatted through facet's display vtable, the same one
/// [`Node::raw_value`](crate::nodes::Node::raw_value) uses, so what a
/// `<select>` submits is exactly what `parse` reads back. A hand-written
/// `Choice` could say `"True"` for a `bool` and never match.
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceField<T> {
    /// Each value with the label the user reads, in display order.
    choices: Vec<(T, String)>,
}

impl<T> ChoiceField<T>
where
    T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    /// # Panics
    ///
    /// If a value displays as `""`. An empty input IS absence in formoxus, so
    /// a choice that submits `""` could never be told apart from no answer.
    /// The "no value" entry belongs to the widget, not to the list.
    pub fn new(choices: impl IntoIterator<Item = impl IntoChoice<T>>) -> Self {
        let choices: Vec<(T, String)> = choices.into_iter().map(IntoChoice::into_choice).collect();
        for (value, label) in &choices {
            assert!(
                !raw(value).is_empty(),
                "the choice labelled {label:?} has a value that displays as \"\", \
                 which formoxus reads as no answer at all"
            );
        }
        Self { choices }
    }

    /// The list as a widget renders it: each value as the string it submits.
    pub fn as_choices(&self) -> Vec<Choice> {
        self.choices
            .iter()
            .map(|(value, label)| Choice::new(raw(value), label.clone()))
            .collect()
    }
}

/// A value that converts to a choice field's `T`: `T` itself, or a `&str`
/// for a `String` field, so a `const` table of `(&str, &str)` pairs works.
///
/// A trait of its own rather than `Into<T>`, because `Into` is implemented
/// for too much: with it, the bare `1` in `(1, "Small")` falls back to `i32`
/// and has no conversion to a `u8` field. Here the only integer that converts
/// to `u8` is `u8`, so rustc infers the literal as the field's type, and
/// `(300, "Huge")` is its out-of-range error.
pub trait ChoiceValue<T> {
    fn into_value(self) -> T;
}

impl<T> ChoiceValue<T> for T {
    fn into_value(self) -> T {
        self
    }
}

impl ChoiceValue<String> for &str {
    fn into_value(self) -> String {
        self.to_string()
    }
}

/// One entry of a choice list, as written: a `(value, label)` pair, or a
/// reference to one, since iterating a `const` slice yields references.
pub trait IntoChoice<T> {
    fn into_choice(self) -> (T, String);
}

impl<T, V: ChoiceValue<T>, L: Into<String>> IntoChoice<T> for (V, L) {
    fn into_choice(self) -> (T, String) {
        (self.0.into_value(), self.1.into())
    }
}

impl<T, V: ChoiceValue<T> + Clone, L: Into<String> + Clone> IntoChoice<T> for &(V, L) {
    fn into_choice(self) -> (T, String) {
        (self.0.clone().into_value(), self.1.clone().into())
    }
}

/// `value` as the string a widget submits for it.
fn raw<T: for<'f> Facet<'f>>(value: &T) -> String {
    Peek::new(value).to_string()
}

impl<T> Field for ChoiceField<T>
where
    T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    type Value = T;

    type Context = ();

    /// Membership. Not left to the widget: a submission that skipped the
    /// browser, or a list that changed while the form was open, can still send
    /// a value the list does not hold.
    fn check(&self, value: &T) -> Vec<ValidationMessage> {
        if self.choices.iter().any(|(choice, _)| choice == value) {
            Vec::new()
        } else {
            vec![ValidationMessage(
                "not one of the available choices".to_string(),
            )]
        }
    }

    /// A select, optional or not. Its choices reach it through
    /// `WidgetProps`, from [`Self::choices`], not as a prop.
    fn default_widget(&self, _optional: bool) -> AnyWidget {
        Select.into()
    }

    fn choices(&self) -> Option<Vec<Choice>> {
        Some(self.as_choices())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::parse_scalar;
    use googletest::prelude::*;

    fn sizes() -> ChoiceField<u8> {
        ChoiceField::new([(1, "Small"), (2, "Medium"), (3, "Large")])
    }

    #[gtest]
    fn a_listed_value_passes() {
        expect_that!(Field::check(&sizes(), &2), is_empty());
    }

    #[gtest]
    fn an_unlisted_value_is_refused() {
        expect_that!(
            Field::check(&sizes(), &4),
            elements_are![eq(&ValidationMessage(
                "not one of the available choices".to_string()
            ))]
        );
    }

    #[gtest]
    fn the_widget_gets_each_value_as_the_string_it_submits() {
        expect_that!(
            sizes().as_choices(),
            elements_are![
                eq(&Choice::new("1", "Small")),
                eq(&Choice::new("2", "Medium")),
                eq(&Choice::new("3", "Large")),
            ]
        );
    }

    /// The point of deriving the strings: whatever a widget submits parses
    /// back to the value it came from. `bool` is the case a hand-written list
    /// got wrong (`"True"`).
    #[gtest]
    fn every_submitted_string_parses_back_to_its_value() {
        let answers = ChoiceField::new([(true, "Yes"), (false, "No")]);
        for (choice, (value, _)) in answers.as_choices().iter().zip(&answers.choices) {
            expect_that!(parse_scalar::<bool>(&choice.value), ok(eq(value)));
        }
    }

    /// The bare integers in `sizes()` are inferred as the field's `u8`; a
    /// `const` table of `&str` pairs works for a `String` field, iterated by
    /// reference as a slice is.
    #[gtest]
    fn a_const_str_table_makes_a_string_field() {
        const STATES: &[(&str, &str)] = &[("AL", "Alabama"), ("AK", "Alaska")];
        let states = ChoiceField::<String>::new(STATES);
        expect_that!(Field::check(&states, &"AK".to_string()), is_empty());
        expect_that!(Field::check(&states, &"TX".to_string()), len(eq(1)));
    }

    #[gtest]
    #[should_panic(expected = "reads as no answer at all")]
    fn a_value_that_displays_as_empty_is_refused() {
        let _ = ChoiceField::new([(String::new(), "Nothing"), ("x".to_string(), "X")]);
    }

    #[gtest]
    fn the_default_widget_is_a_select_optional_or_not() {
        expect_that!(sizes().default_widget(false), eq(&AnyWidget::from(Select)));
        expect_that!(sizes().default_widget(true), eq(&AnyWidget::from(Select)));
    }
}
