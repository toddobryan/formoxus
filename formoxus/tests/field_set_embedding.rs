//! End-to-end test for `#[form(field_set)]` — a `Form` embedding a `FieldSet`
//! as one of its own fields, mirroring `derive.rs`'s coverage style. No Dioxus
//! runtime is exercised here (these tests never call `render`), but merely
//! compiling this file already proves the generic-lens plumbing in
//! `FieldSetState::render` type-checks: that generic body is elaborated at
//! definition time regardless of whether a test calls it, so a bad bound here
//! would fail to *compile*, not just fail at runtime.

use formoxus::prelude::*;
use googletest::prelude::*;

#[derive(FieldSet, Debug, Clone)]
#[allow(dead_code)]
pub struct QuestionCommon {
    name: String,
    source: String,
}

#[derive(Form, Debug)]
#[form(button(type = "submit", name = submit))]
#[allow(dead_code)]
pub struct QuestionForm {
    #[form(field_set)]
    common: QuestionCommon,
    text: String,
}

/// An empty form: the top-level required field AND the embedded field set's
/// own required fields all stamp their own errors, and the parent's
/// `has_errors` sees the embedded field set's errors too.
#[gtest]
fn empty_form_flags_required_fields_in_both_the_form_and_the_embedded_field_set() {
    let mut form = QuestionFormState::default();

    expect_that!(form.validate(), none());
    expect_that!(form.text.errors, len(eq(1)));
    expect_that!(form.common.name.errors, len(eq(1)));
    expect_that!(form.common.source.errors, len(eq(1)));
    expect_that!(form.has_errors(), eq(true));
}

/// A form seeded from a model — including the nested field set's own model —
/// round-trips back through `validate`.
#[gtest]
fn valid_model_round_trips_through_the_embedded_field_set() {
    let mut form = QuestionFormState::from_model(&QuestionForm {
        common: QuestionCommon {
            name: "Warm-up".to_string(),
            source: "unit-1-quiz".to_string(),
        },
        text: "What does CPU stand for?".to_string(),
    });

    let model = form.validate();
    expect_that!(model, some(anything()));

    let model = model.expect("a fully-seeded form should validate");
    expect_that!(model.text, eq("What does CPU stand for?"));
    expect_that!(model.common.name, eq("Warm-up"));
    expect_that!(model.common.source, eq("unit-1-quiz"));
}
