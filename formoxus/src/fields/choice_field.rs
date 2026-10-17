use std::fmt::Debug;

use facet::{Facet, Peek};

use crate::{ValidationMessage, field::Field, widgets::Choice};

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
    pub fn new(choices: impl IntoIterator<Item = (T, impl Into<String>)>) -> Self {
        let choices: Vec<(T, String)> = choices
            .into_iter()
            .map(|(value, label)| (value, label.into()))
            .collect();
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

    #[gtest]
    #[should_panic(expected = "reads as no answer at all")]
    fn a_value_that_displays_as_empty_is_refused() {
        let _ = ChoiceField::new([(String::new(), "Nothing"), ("x".to_string(), "X")]);
    }
}
