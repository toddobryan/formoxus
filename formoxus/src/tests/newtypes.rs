//! Newtype wrappers as leaves.
//!
//! `struct Markdown(String)` is one input, not a fieldset around a field called
//! `"0"`. The wrapper can't be the `FormField`'s own type parameter — a
//! concrete `T` is unrecoverable from a runtime `&'static Shape` — so the field
//! carries the INNER scalar and remembers what to re-wrap it in. These pin both
//! halves of that: the string-facing side behaves exactly like the bare scalar,
//! and the value still rebuilds as the wrapper.

use crate::*;
use facet::Facet;
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Note(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Count(u32);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Doc {
    title: String,
    body: Note,
}

/// A one-field NAMED struct — deliberately NOT flattened.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Config {
    name: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Settings {
    config: Config,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Tally {
    label: String,
    count: Count,
}

/// A newtype around something that ISN'T a scalar — no single input could
/// carry it, so it stays a struct.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Pair(Config);

#[derive(Facet, Clone, Debug, PartialEq)]
struct HasPair {
    pair: Pair,
}

fn paths_of(form: &FormState<impl Clone + std::fmt::Debug + PartialEq + Facet<'static>>) -> Vec<String> {
    form.leaves().into_iter().map(|(p, _)| p).collect()
}

// ── The leaf, not a fieldset ─────────────────────────────────────────────

#[gtest]
fn a_newtype_is_one_leaf_at_the_fields_own_path() {
    let form = empty_form::<Doc>(FormSpec::default());
    expect_that!(
        paths_of(&form),
        elements_are![eq("title"), eq("body")],
        "a newtype must NOT produce `body.0`"
    );
}

#[gtest]
fn a_one_field_named_struct_is_still_a_struct() {
    // The whole reason the rule is `TupleStruct`, not "one field": flattening
    // this would rename the leaf, and leaf paths are the wire format.
    let form = empty_form::<Settings>(FormSpec::default());
    expect_that!(paths_of(&form), elements_are![eq("config.name")]);
}

#[gtest]
fn a_newtype_around_a_non_scalar_is_still_a_struct() {
    let form = empty_form::<HasPair>(FormSpec::default());
    expect_that!(paths_of(&form), elements_are![eq("pair.0.name")]);
}

// ── Round trips ──────────────────────────────────────────────────────────

#[gtest]
fn a_newtype_round_trips_through_edit_mode() {
    // Populating reads THROUGH the wrapper, and writing rebuilds it.
    let doc = Doc { title: "Unit 1".to_string(), body: Note("hello **there**".to_string()) };
    let mut form = form_for(&doc, FormSpec::default());
    expect_that!(form.validate(), some(eq(&doc)));
}

#[gtest]
fn a_populated_newtype_shows_its_inner_value_in_the_input() {
    let doc = Doc { title: "Unit 1".to_string(), body: Note("hello".to_string()) };
    let form = form_for(&doc, FormSpec::default());
    let body: Vec<String> = form
        .leaves()
        .into_iter()
        .filter(|(p, _)| p == "body")
        .map(|(_, v)| v)
        .collect();
    expect_that!(
        body,
        elements_are![eq("hello")],
        "the raw value is the INNER string, not a Debug of the wrapper"
    );
}

#[gtest]
fn a_newtype_builds_from_submitted_values() {
    let mut form = empty_form::<Doc>(FormSpec::default());
    form.apply_form_values(&[
        ("title".to_string(), "Unit 1".to_string()),
        ("body".to_string(), "typed by hand".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&Doc {
            title: "Unit 1".to_string(),
            body: Note("typed by hand".to_string()),
        }))
    );
}

// ── The inner scalar's own rules still apply ─────────────────────────────

#[gtest]
fn a_numeric_newtype_parses_through_the_inner_vtable() {
    let mut form = empty_form::<Tally>(FormSpec::default());
    form.apply_form_values(&[
        ("label".to_string(), "Votes".to_string()),
        ("count".to_string(), "42".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&Tally { label: "Votes".to_string(), count: Count(42) }))
    );
}

#[gtest]
fn a_bad_inner_value_is_rejected_rather_than_defaulted() {
    // The failure mode worth pinning: parsing goes through the INNER shape's
    // vtable, so `Count("abc")` fails exactly as a bare `u32` would instead of
    // quietly becoming `Count(0)`.
    let mut form = empty_form::<Tally>(FormSpec::default());
    form.apply_form_values(&[
        ("label".to_string(), "Votes".to_string()),
        ("count".to_string(), "not-a-number".to_string()),
    ]);
    expect_that!(form.validate(), none());
    expect_that!(
        form.collect_errors().fields.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>(),
        elements_are![eq("count")]
    );
}

#[gtest]
fn an_empty_newtype_field_is_required_like_any_other() {
    let mut form = empty_form::<Doc>(FormSpec::default());
    form.apply_form_values(&[("title".to_string(), "Unit 1".to_string())]);
    expect_that!(form.validate(), none());
    expect_that!(
        form.collect_errors().fields.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>(),
        elements_are![eq("body")]
    );
}

// ── Optional ─────────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct MaybeDoc {
    title: String,
    body: Option<Note>,
}

#[gtest]
fn an_absent_optional_newtype_round_trips_as_none() {
    let mut form = empty_form::<MaybeDoc>(FormSpec::default());
    form.apply_form_values(&[("title".to_string(), "Unit 1".to_string())]);
    expect_that!(
        form.validate(),
        some(eq(&MaybeDoc { title: "Unit 1".to_string(), body: None }))
    );
}

#[gtest]
fn a_present_optional_newtype_round_trips_as_some() {
    let doc = MaybeDoc {
        title: "Unit 1".to_string(),
        body: Some(Note("here".to_string())),
    };
    let mut form = form_for(&doc, FormSpec::default());
    expect_that!(form.validate(), some(eq(&doc)));
}
