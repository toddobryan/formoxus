//! Fields whose value is one of a list (`CUSTOM_FIELDS_PLAN.md` decisions AS,
//! AT and AU).
//!
//! A choice owns its text, as Django's `TypedChoiceField` does: each option
//! is a [`Choice`], a key the widget submits and the text the user reads. A
//! [`ChoiceField`] turns the chosen key into a value (`coerce`) and a value
//! back into its key (`key`), and formoxus does the rest: membership first,
//! so `coerce` only ever sees a key that was offered.
//!
//! Two ways in:
//!
//! 1. **`choices:` in `form!`, for a built-in type** (`String`, `bool`, a
//!    number): a `const` table such as `&[("AL", "Alabama")]`, or
//!    `[(1, "Small"), (2, "Medium")]` on a `u8`. `form!` builds a
//!    [`ScalarChoices`].
//! 2. **`field:` with a type of the dev's own that implements
//!    [`ChoiceField`],** for anything else, such as a `Ref<Question>` picked
//!    from a course's questions.
//!
//! A list known only at run time needs nothing more: build the form with a
//! function that is given the list, and call it on both sides, the browser
//! after loading the list and the submit server function after loading its
//! own copy. Only values cross the wire.

use std::fmt::Debug;

use facet::{Facet, Peek};
use formoxus_spec::FieldType;

use crate::error::ValidationMessage;
use crate::field::{Field, Sealed, parse_scalar};
use crate::field_kind::field_type_of;
use crate::widgets::{AnyWidget, Select};

/// One option of a list: the key it submits, and the text the user reads.
///
/// The key is what a form's values hold and what crosses the wire, so it is
/// exactly the text [`ChoiceField::coerce`] reads. Never `""`: an empty input
/// IS absence in formoxus, so a choice that submitted `""` could not be told
/// apart from no answer.
///
/// A value choice. A unit enum's variants are offered with the same type, by
/// the node that builds the enum.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// What the widget submits for this option.
    pub key: String,
    /// What the user reads.
    pub display: String,
}

impl Choice {
    pub fn new(key: impl Into<String>, display: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            display: display.into(),
        }
    }
}

// A choice list is usually written as a literal table, and `Choice` holds
// `String`s, so it cannot be a `const` array. These conversions are what let
// the list be a plain `&[(&str, &str)]`, which CAN be `const`, and still
// arrive as choices. The borrowed-tuple impl exists because iterating a slice
// yields references.
impl From<(&str, &str)> for Choice {
    fn from((key, display): (&str, &str)) -> Self {
        Self::new(key, display)
    }
}

impl From<&(&str, &str)> for Choice {
    fn from(pair: &(&str, &str)) -> Self {
        Self::from(*pair)
    }
}

impl From<(String, String)> for Choice {
    fn from((key, display): (String, String)) -> Self {
        Self::new(key, display)
    }
}

/// A key that is its own display text.
impl From<&str> for Choice {
    fn from(key: &str) -> Self {
        Self::new(key, key)
    }
}

impl From<&&str> for Choice {
    fn from(key: &&str) -> Self {
        Self::from(*key)
    }
}

impl From<String> for Choice {
    fn from(key: String) -> Self {
        Self::new(key.clone(), key)
    }
}

/// A field whose value is one of a list: implement this, and the type is a
/// [`Field`] too, through the blanket impl below, given to a path with
/// `field:` in `form!`.
///
/// ```ignore
/// #[derive(Clone, Debug, PartialEq)]
/// struct QuestionChoice {
///     choices: Vec<Choice>,
/// }
///
/// impl ChoiceField for QuestionChoice {
///     type Value = Ref<Question>;
///     fn options(&self) -> &[Choice] {
///         &self.choices
///     }
///     fn coerce(&self, key: &str) -> Result<Ref<Question>, ValidationMessage> {
///         Ok(Ref::from_key(key))
///     }
///     fn key(&self, value: &Ref<Question>) -> String {
///         value.key().to_string()
///     }
/// }
///
/// fn assignment_form(questions: Vec<Choice>) -> FormSpec<Assignment> {
///     form! { Assignment { questions[] => { field: QuestionChoice { choices: questions } } } }
/// }
/// ```
///
/// What the blanket impl does, so no implementation can get it wrong:
///
/// 1. **Membership first.** A key that is not one of [`options`](Self::options)
///    is refused with "not one of the available choices", and
///    [`coerce`](Self::coerce) never sees it. So `coerce` only ever reads a
///    key the form itself offered: a forged key never reaches it.
/// 2. **A value is written as its [`key`](Self::key),** so a form filled
///    from an existing model shows that value's option as chosen, and a
///    value whose key is no longer offered is refused when the form is
///    checked.
/// 3. **A `select` renders it** unless the path names another widget that
///    takes choices (`radio_group`).
///
/// **`coerce` runs on both sides.** The browser builds the model too, to run
/// the form's checks, so `coerce` must be quick, synchronous, and give the
/// same answer everywhere: no database. For a reference type that costs
/// nothing, since its value is its key.
///
/// **The server must not trust what the client sends as the inputs to its
/// list.** A submit server function builds its own list, from its own
/// session and data: a list built from a client's `course_id` without
/// checking that the user may use that course would widen the choices to
/// another course's.
pub trait ChoiceField: Clone + Debug + PartialEq + 'static {
    /// The type the model holds.
    type Value: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static;

    /// The kind of text the keys are, as [`Field::KIND`]: `Text` unless every
    /// key is a number or a bool and widgets should be judged by that.
    const KIND: FieldType = FieldType::Text;

    /// The options, in the order the widget shows them.
    fn options(&self) -> &[Choice];

    /// The value a key stands for. Only ever called with one of
    /// [`options`](Self::options)' keys.
    ///
    /// # Errors
    ///
    /// The message the user sees if a key cannot become a value, which for a
    /// list the field built itself should never happen.
    fn coerce(&self, key: &str) -> Result<Self::Value, ValidationMessage>;

    /// The key a value is written as: the reverse of [`coerce`](Self::coerce).
    fn key(&self, value: &Self::Value) -> String;
}

fn not_a_choice() -> ValidationMessage {
    ValidationMessage("not one of the available choices".to_string())
}

fn is_offered<C: ChoiceField>(field: &C, key: &str) -> bool {
    field.options().iter().any(|choice| choice.key == key)
}

/// Every [`ChoiceField`] is a [`Field`], written once here: see the trait's
/// docs for what each method does.
///
/// `do_not_recommend`, so a type that is no `Field` at all is told so
/// (`form_field_not_a_field`), not told it should implement `ChoiceField`,
/// which is what rustc says otherwise when this is the only impl it can see.
#[diagnostic::do_not_recommend]
impl<C: ChoiceField> Field for C {
    type Value = C::Value;
    type Context = ();

    const KIND: FieldType = <C as ChoiceField>::KIND;
    const HAS_CHOICES: bool = true;

    fn parse(&self, raw: &str) -> Result<Self::Value, ValidationMessage> {
        if is_offered(self, raw) {
            self.coerce(raw)
        } else {
            Err(not_a_choice())
        }
    }

    fn format(&self, value: &Self::Value) -> String {
        self.key(value)
    }

    /// Membership again, for a value that came from the model rather than
    /// from `parse`: a submission that skipped the browser, or a list that
    /// changed while the form was open, can still hold a value the list does
    /// not.
    fn check(&self, value: &Self::Value) -> Vec<ValidationMessage> {
        if is_offered(self, &self.key(value)) {
            Vec::new()
        } else {
            vec![not_a_choice()]
        }
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        Select.into()
    }

    #[allow(private_interfaces)]
    fn choices(&self, _: Sealed) -> Option<Vec<Choice>> {
        Some(self.options().to_vec())
    }
}

/// The [`ChoiceField`] `choices:` builds for a built-in type: a value that
/// must be one of a fixed list, typed, so a `ScalarChoices<u8>` offers `u8`s.
///
/// **The keys are derived, never written by hand.** Each value is written as
/// its type's own text, the same text a plain field of that type writes and
/// reads, so what a `<select>` submits parses back to the value it came from.
/// A hand-written key could say `"True"` for a `bool` and never match.
#[derive(Clone, Debug, PartialEq)]
pub struct ScalarChoices<T> {
    options: Vec<Choice>,
    value: std::marker::PhantomData<fn() -> T>,
}

impl<T> ScalarChoices<T>
where
    T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    /// # Panics
    ///
    /// If a value is written as `""`, which formoxus reads as no answer at
    /// all. The "no value" entry belongs to the widget, not to the list.
    pub fn new(choices: impl IntoIterator<Item = impl IntoChoice<T>>) -> Self {
        let options: Vec<Choice> = choices
            .into_iter()
            .map(IntoChoice::into_choice)
            .map(|(value, label)| Choice::new(Peek::new(&value).to_string(), label))
            .collect();
        for choice in &options {
            assert!(
                !choice.key.is_empty(),
                "the choice labelled {:?} has a value that is written as \"\", which formoxus \
                 reads as no answer at all",
                choice.display
            );
        }
        Self {
            options,
            value: std::marker::PhantomData,
        }
    }
}

impl<T> ChoiceField for ScalarChoices<T>
where
    T: Clone + Debug + PartialEq + for<'f> Facet<'f> + 'static,
{
    type Value = T;

    const KIND: FieldType = field_type_of(T::SHAPE);

    fn options(&self) -> &[Choice] {
        &self.options
    }

    fn coerce(&self, key: &str) -> Result<T, ValidationMessage> {
        parse_scalar(key)
    }

    fn key(&self, value: &T) -> String {
        Peek::new(value).to_string()
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

/// One entry of a built-in choice list, as written: a `(value, label)` pair,
/// or a reference to one, since iterating a `const` slice yields references.
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

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    fn sizes() -> ScalarChoices<u8> {
        ScalarChoices::new([(1, "Small"), (2, "Medium"), (3, "Large")])
    }

    #[gtest]
    fn a_listed_value_passes() {
        expect_that!(Field::check(&sizes(), &2), is_empty());
    }

    #[gtest]
    fn an_unlisted_value_is_refused() {
        expect_that!(
            Field::check(&sizes(), &4),
            elements_are![eq(&not_a_choice())]
        );
    }

    #[gtest]
    fn the_widget_gets_each_value_as_the_key_it_submits() {
        expect_that!(
            sizes().options(),
            elements_are![
                eq(&Choice::new("1", "Small")),
                eq(&Choice::new("2", "Medium")),
                eq(&Choice::new("3", "Large")),
            ]
        );
    }

    /// The point of deriving the keys: whatever a widget submits parses back
    /// to the value it came from. `bool` is the case a hand-written list got
    /// wrong (`"True"`).
    #[gtest]
    fn every_submitted_key_parses_back_to_its_value() {
        let answers = ScalarChoices::new([(true, "Yes"), (false, "No")]);
        for (choice, value) in answers.options().iter().zip([true, false]) {
            expect_that!(Field::parse(&answers, &choice.key), ok(eq(&value)));
        }
    }

    /// The bare integers in `sizes()` are inferred as the field's `u8`; a
    /// `const` table of `&str` pairs works for a `String` field, iterated by
    /// reference as a slice is.
    #[gtest]
    fn a_const_str_table_makes_a_string_field() {
        const STATES: &[(&str, &str)] = &[("AL", "Alabama"), ("AK", "Alaska")];
        let states = ScalarChoices::<String>::new(STATES);
        expect_that!(Field::check(&states, &"AK".to_string()), is_empty());
        expect_that!(Field::check(&states, &"TX".to_string()), len(eq(1)));
    }

    /// Membership before `coerce`: text that would parse as a `u8` but is
    /// not one of the keys is refused as a choice, not read as a number.
    #[gtest]
    fn a_key_that_is_not_offered_is_refused_before_it_is_coerced() {
        expect_that!(Field::parse(&sizes(), "4"), err(eq(&not_a_choice())));
        expect_that!(Field::parse(&sizes(), "01"), err(eq(&not_a_choice())));
        expect_that!(Field::parse(&sizes(), "2"), ok(eq(&2)));
    }

    #[gtest]
    #[should_panic(expected = "reads as no answer at all")]
    fn a_value_that_is_written_as_empty_is_refused() {
        let _ = ScalarChoices::new([(String::new(), "Nothing"), ("x".to_string(), "X")]);
    }

    #[gtest]
    fn the_default_widget_is_a_select_optional_or_not() {
        expect_that!(
            Field::default_widget(&sizes(), false),
            eq(&AnyWidget::from(Select))
        );
        expect_that!(
            Field::default_widget(&sizes(), true),
            eq(&AnyWidget::from(Select))
        );
    }

    #[gtest]
    fn a_choice_field_says_it_has_choices_and_hands_them_over() {
        expect_that!(<ScalarChoices<u8> as Field>::HAS_CHOICES, eq(true));
        expect_that!(Field::choices(&sizes(), Sealed::new()), some(len(eq(3))));
        expect_that!(<ScalarChoices<u8> as Field>::KIND, eq(FieldType::Int));
    }
}
