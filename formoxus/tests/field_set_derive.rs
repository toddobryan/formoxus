//! End-to-end tests for the `FieldSet` derive — mirrors `derive.rs`'s coverage
//! for `Form`, minus everything button/handler-related (a `FieldSet` has none
//! of its own; it's meant to be embedded inside a `Form`).
//!
//! The derive generates the `DummyFieldSetState` struct, its `from_model` /
//! `validate`, and the `FieldSet` / `ValidateForm` impls — no Dioxus runtime
//! required, since a field set is plain data.

use formoxus::prelude::*;
use googletest::prelude::*;

#[derive(FieldSet, Debug)]
#[allow(dead_code)]
pub struct DummyFieldSet {
    name: String,
    count: i32,
    opt: Option<bool>,
}

/// Compiling this file already proves the derive applies via the facade and
/// accepts its `#[field_set(...)]` helper attribute; keep a trivial case as a
/// marker.
#[gtest]
fn derive_applies_cleanly() {
    let field_set = DummyFieldSetState::default();
    expect_that!(field_set.errors, is_empty());
}

/// An empty field set: `validate` yields nothing, each required field stamps
/// its own error, and the optional field stays clean.
#[gtest]
fn empty_field_set_flags_only_required_fields() {
    let mut field_set = DummyFieldSetState::default();

    expect_that!(field_set.validate(), none());
    expect_that!(field_set.name.errors, len(eq(1)));
    expect_that!(field_set.count.errors, len(eq(1)));
    expect_that!(field_set.opt.errors, is_empty());
}

/// A field set seeded from a model round-trips back to that model on
/// `validate`: required fields unwrap to their real types, the optional one
/// passes through.
#[gtest]
fn valid_model_round_trips() {
    let mut field_set = DummyFieldSetState::from_model(&DummyFieldSet {
        name: "hi".to_string(),
        count: 3,
        opt: Some(true),
    });

    let model = field_set.validate();
    expect_that!(model, some(anything()));

    let model = model.expect("a fully-seeded field set should validate");
    expect_that!(model.name, eq("hi"));
    expect_that!(model.count, eq(3));
    expect_that!(model.opt, eq(Some(true)));
}
