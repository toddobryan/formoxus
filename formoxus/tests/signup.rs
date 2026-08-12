//! RED TARGET — behavior spec for the `FieldValue` migration + the `validate()`
//! `has_errors()` gate.
//!
//! `Signup` covers the three cases the three-state `FieldValue`
//! (`Empty` | `Valid` | `Invalid`) introduces:
//!   - a required text field (`username`),
//!   - a required *parsed* field (`age: i32`) that can be `Invalid`,
//!   - an optional *parsed* field (`referral: Option<i32>`) — the case where an
//!     `Invalid` value must fail the form even though it contributes `None` to the
//!     model and the model therefore *builds*.
//!
//! This will be red until the codegen emits the `has_errors()` gate in `validate()`
//! and a generated `pub fn has_errors(&self) -> bool`. (The current epilogue only
//! checks form-level errors, so `optional_invalid_fails_the_form` will wrongly get
//! `Some(model)` until the gate lands — that's the target.)
//!
//! API assumptions, all matching `fields.rs` today:
//!   - `FormField` fields are public, so a test can drop a field into `Invalid`
//!     directly (as a widget would after a failed parse) — see `invalid()`.
//!   - the generated state is `SignupState`, its fields public, `has_errors()` public.
//!
//! A follow-on target for the `#[form(min_len = …)]` validator is sketched at the end.

use std::fmt::Debug;
use std::str::FromStr;

use formoxus::fields::FieldValue;
use formoxus::prelude::*;
use googletest::prelude::*;

#[derive(Form, Debug, Clone, PartialEq)]
struct Signup {
    username: String,      // required text
    age: i32,              // required, parsed → can be Invalid
    referral: Option<i32>, // optional, parsed → Invalid must still fail the form
}

/// An unparseable value, as a widget leaves it after a failed parse: the raw text
/// plus a parse error, living in the `Invalid` branch (not the `errors` vec).
fn invalid<T: Clone + Debug + FromStr>(raw: &str) -> FieldValue<T> {
    FieldValue::Invalid {
        raw: raw.to_string(),
        error: FieldError(format!("`{raw}` is not valid")),
    }
}

/// Seed a valid form, then let each test perturb one field.
fn valid_form() -> SignupState {
    SignupState::from_model(&Signup {
        username: "ada".to_string(),
        age: 30,
        referral: Some(7),
    })
}

#[gtest]
fn empty_form_is_invalid_and_flags_only_required_fields() {
    let mut form = SignupState::default();

    expect_that!(form.validate(), none());
    // Required fields stamp a presence error; the optional one does not.
    expect_that!(form.username.errors, len(eq(1)));
    expect_that!(form.age.errors, len(eq(1)));
    expect_that!(form.referral.errors, is_empty());
    expect_that!(form.has_errors(), eq(true));
}

#[gtest]
fn fully_valid_form_yields_the_model() {
    let mut form = valid_form();

    let model = form.validate();
    assert_that!(model, some(anything()));
    let model = model.unwrap();
    expect_that!(model.username, eq("ada"));
    expect_that!(model.age, eq(30));
    expect_that!(model.referral, eq(Some(7)));

    expect_that!(form.has_errors(), eq(false));
}

#[gtest]
fn optional_absent_is_valid_and_maps_to_none() {
    let mut form = valid_form();
    form.referral.value = FieldValue::Empty;

    let model = form.validate();
    assert_that!(model, some(anything()));
    expect_that!(model.unwrap().referral, none());
}

#[gtest]
fn required_invalid_fails_without_a_spurious_required_error() {
    let mut form = valid_form();
    // Widget left an unparseable value on the required `age`.
    form.age.value = invalid::<i32>("thirty");

    expect_that!(form.validate(), none());
    // The `Invalid` branch carries its OWN parse error, so `required()` must NOT
    // stamp a second "required" error on top — the errors vec stays empty.
    expect_that!(form.age.errors, is_empty());
    // …yet the field still counts as errored, via the `Invalid` state.
    expect_that!(form.age.has_errors(), eq(true));
}

#[gtest]
fn optional_invalid_fails_the_form_even_though_it_builds() {
    let mut form = valid_form();
    // An unparseable *optional* field contributes `None` to the model, so the model
    // builds fine. The `has_errors()` gate is the ONLY thing that fails the form —
    // this is the case the old form-errors-only epilogue let through.
    form.referral.value = invalid::<i32>("seven");

    expect_that!(form.validate(), none());
    expect_that!(form.referral.errors, is_empty());
    expect_that!(form.referral.has_errors(), eq(true));
}

// ── Follow-on target: the `#[form(min_len = …)]` validator ───────────────────────
//
// Once field validators are wired, add `#[form(min_len = 3)]` to `username` and a
// test like this — it exercises the OTHER gate path: a field that is `Valid` (so it
// passes the required-destructure) but carries a *validator* error, which only the
// `has_errors()` gate catches:
//
//   #[gtest]
//   fn valid_but_too_short_fails_via_the_gate() {
//       let mut form = valid_form();
//       form.username.value = FieldValue::Valid("ab".to_string()); // 2 < 3
//       expect_that!(form.validate(), none());
//       expect_that!(form.username.errors, len(eq(1)));            // the min_len error
//   }
