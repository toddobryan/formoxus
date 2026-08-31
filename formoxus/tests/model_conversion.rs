//! Minimal repro for `#[form(model = X)]` where `X`'s fields don't line up
//! 1:1 by name with the form's own fields — the simplest possible case that
//! forces the "Model != Self" question at the *form* level (no embedded
//! `FieldSet` at all, unlike `generic_form.rs`'s `Wrapper<T>`). `X`'s fields
//! are renamed *and* reordered relative to `XForm`'s, so the derive's usual
//! by-name shorthand construction (which only cares about names, not order)
//! genuinely can't produce `X` on its own — proving this needs the
//! `From<XForm> for X` / `From<X> for XForm` conversion to actually run, not
//! just compile.

use formoxus::fields::FieldValue;
use formoxus::prelude::*;
use googletest::prelude::*;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct X {
    pub last_name: String,
    pub given_name: String,
}

#[derive(Form, Debug)]
#[form(model = X, button(type = "submit", name = submit))]
#[allow(dead_code)]
pub struct XForm {
    pub given_name: String,
    pub last_name: String,
}

impl From<XForm> for X {
    fn from(form: XForm) -> Self {
        X {
            last_name: form.last_name,
            given_name: form.given_name,
        }
    }
}

impl From<X> for XForm {
    fn from(x: X) -> Self {
        XForm {
            given_name: x.given_name,
            last_name: x.last_name,
        }
    }
}

#[gtest]
fn seeding_from_a_model_maps_fields_by_name_not_position() {
    let x = X {
        last_name: "Lovelace".to_string(),
        given_name: "Ada".to_string(),
    };

    let state = XFormState::from_model(&x);

    expect_that!(
        state.given_name.value,
        matches_pattern!(FieldValue::Valid(eq(&"Ada".to_string())))
    );
    expect_that!(
        state.last_name.value,
        matches_pattern!(FieldValue::Valid(eq(&"Lovelace".to_string())))
    );
}

#[gtest]
fn validating_produces_the_model_not_the_form_struct() {
    let x = X {
        last_name: "Lovelace".to_string(),
        given_name: "Ada".to_string(),
    };

    let mut state = XFormState::from_model(&x);
    let validated = state.validate();

    expect_that!(validated, some(eq(&x)));
}

#[gtest]
fn a_full_round_trip_preserves_the_model() {
    let original = X {
        last_name: "Turing".to_string(),
        given_name: "Alan".to_string(),
    };

    let mut state = XFormState::from_model(&original);
    let round_tripped = state.validate().expect("fully seeded form should validate");

    expect_that!(round_tripped, eq(&original));
}
