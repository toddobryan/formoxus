//! End-to-end test for `#[form(field_set)]` on a `Vec<T>`-shaped field — a
//! repeating group of `FieldSet`s (formoxus's analog of a Django formset).
//! No Dioxus runtime is exercised here (these tests never call `render`),
//! but merely compiling this file already proves the generic-lens plumbing
//! in `render_call`'s list branch type-checks.

use formoxus::prelude::*;
use googletest::prelude::*;

#[derive(FieldSet, Debug, Clone, Default, PartialEq)]
#[allow(dead_code)]
pub struct Choice {
    text: String,
}

#[derive(Form, Debug)]
#[form(button(type = "submit", name = submit))]
#[allow(dead_code)]
pub struct MultipleChoiceForm {
    #[form(field_set)]
    options: Vec<Choice>,
}

/// An empty list of rows is itself valid (no row to fail) — the list is
/// required as a `Vec`, not as "at least one row"; an empty `Vec` still
/// validates to `Some(vec![])`.
#[gtest]
fn empty_list_of_rows_validates_to_an_empty_vec() {
    let mut form = MultipleChoiceFormState::default();

    let model = form.validate();
    expect_that!(model, some(anything()));
    expect_that!(model.unwrap().options, is_empty());
}

/// Every row's own required fields get validated — including rows after an
/// earlier invalid one, proving the two-pass `assign_to_vars` codegen
/// doesn't short-circuit and skip validating (and erroring) later rows.
#[gtest]
fn every_row_is_validated_even_after_an_earlier_row_fails() {
    let mut form = MultipleChoiceFormState::default();
    form.options.push(ChoiceState::default()); // row 0: empty, invalid
    form.options.push(ChoiceState::default()); // row 1: also empty, invalid

    expect_that!(form.validate(), none());
    expect_that!(form.options[0].text.errors, len(eq(1)));
    expect_that!(form.options[1].text.errors, len(eq(1)));
}

/// A form seeded from a model with several rows round-trips back through
/// `validate`, in order.
#[gtest]
fn valid_rows_round_trip_in_order() {
    let mut form = MultipleChoiceFormState::from_model(&MultipleChoiceForm {
        options: vec![
            Choice {
                text: "First".to_string(),
            },
            Choice {
                text: "Second".to_string(),
            },
        ],
    });

    let model = form.validate();
    expect_that!(model, some(anything()));

    let options = model.expect("a fully-seeded form should validate").options;
    expect_that!(
        options,
        elements_are![
            eq(&Choice {
                text: "First".to_string(),
            }),
            eq(&Choice {
                text: "Second".to_string(),
            }),
        ]
    );
}
