//! End-to-end tests for the `Form` derive.
//!
//! Three-tier strategy (mirroring `surreal-table`):
//!   1. in-crate unit tests of the codegen helpers,
//!   2. macro-output tests — derive a form, assert the generated `from_model` /
//!      `validate` behave (this file),
//!   3. `trybuild` UI goldens in `tests/ui/` for the compile-fail diagnostics
//!      (bad `#[form(...)]` attrs, non-struct input, etc.). `trybuild` is already
//!      a dev-dependency; add a `compile_fail` runner here once cases exist.
//!
//! The derive generates the `DummyFormState` struct, its `from_model` /
//! `validate`, and the `Form` / `ValidateForm` impls; these tests exercise that
//! generated behavior — no Dioxus runtime required, since a form is plain data.

use formoxus::*;
use googletest::prelude::*;

#[derive(Form, Debug)]
#[allow(dead_code)]
pub struct DummyForm {
    name: String,
    count: i32,
    opt: Option<bool>,
}

/// Compiling this file already proves the derive applies via the facade and
/// accepts its `#[form(...)]` helper attribute; keep a trivial case as a marker.
#[gtest]
fn derive_applies_cleanly() {
    let form = DummyFormState::default();
    expect_that!(form.errors, is_empty());
}

/// An empty form: `validate` yields nothing, each required field stamps its own
/// error, and the optional field stays clean.
#[gtest]
fn empty_form_flags_only_required_fields() {
    let mut form = DummyFormState::default();

    expect_that!(form.validate(), none());
    expect_that!(form.name.errors, len(eq(1)));
    expect_that!(form.count.errors, len(eq(1)));
    expect_that!(form.opt.errors, is_empty());
}

/// A form seeded from a model round-trips back to that model on `validate`:
/// required fields unwrap to their real types, the optional one passes through.
#[gtest]
fn valid_model_round_trips() {
    let mut form = DummyFormState::from_model(&DummyForm {
        name: "hi".to_string(),
        count: 3,
        opt: Some(true),
    });

    let model = form.validate();
    expect_that!(model, some(anything()));

    let model = model.expect("a fully-seeded form should validate");
    expect_that!(model.name, eq("hi"));
    expect_that!(model.count, eq(3));
    expect_that!(model.opt, eq(Some(true)));
}
