//! `collect_errors` — turning a validated tree into the flat, serializable
//! `FormErrors` that crosses a server fn.
//!
//! The counterpart to `roundtrip`/`forms`, which cover values going out and
//! back. These cover *verdicts* coming out. The contract under test is stated
//! on [`FormMember::collect_errors`](formoxus::FormMember::collect_errors):
//! only members with errors appear, and a container that can be *pushed* an
//! error at its own path must be able to hand it back from there.

use super::models::{EventForCreate, Location, Shape};
use facet::Facet;
use formoxus::*;
use googletest::prelude::*;

/// A struct with an enum field. `enums` has its own `Drawing` of the same
/// shape; this one is local rather than shared because the two modules assert
/// on different things and neither should constrain the other's fixture.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Drawing {
    name: String,
    shape: Shape,
}

/// Just the paths, in collected order — most assertions here don't care about
/// the message, only about where it landed.
fn paths(errors: &FormErrors) -> Vec<String> {
    errors.fields.iter().map(|(p, _)| p.clone()).collect()
}

/// The messages recorded at exactly `path`, or an empty vec if it's absent.
fn messages_at(errors: &FormErrors, path: &str) -> Vec<String> {
    errors
        .fields
        .iter()
        .filter(|(p, _)| p == path)
        .flat_map(|(_, errs)| errs.iter().map(|e| e.0.clone()))
        .collect()
}

fn filled_event() -> EventForCreate {
    EventForCreate {
        title: "Recital".to_string(),
        location: Location {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            zip: "12345".to_string(),
        },
    }
}

// ── The emptiness contract ───────────────────────────────────────────────

#[gtest]
fn a_passing_form_collects_nothing_at_all() {
    // The load-bearing case for the whole shape: a caller decides "did the
    // server accept it?" by asking whether `fields` is empty, so a clean form
    // must not report `(path, [])` for every field it happens to contain.
    let mut form = form_for(&filled_event(), FormSpec::default());
    expect_that!(form.validate(), some(eq(&filled_event())));

    let errors = form.collect_errors();
    expect_that!(errors.fields, is_empty());
    expect_that!(errors.form, is_empty());
}

#[gtest]
fn only_the_failing_fields_appear() {
    // One blank field among four filled ones. The three that passed stay out,
    // which is the same rule as above seen from the other side.
    let mut form = empty_form::<EventForCreate>(FormSpec::default());
    form.apply_form_values(&[
        ("title".to_string(), "Recital".to_string()),
        ("location.street".to_string(), "123 Main St".to_string()),
        ("location.city".to_string(), String::new()),
        ("location.zip".to_string(), "12345".to_string()),
    ]);
    expect_that!(form.validate(), none());

    let errors = form.collect_errors();
    expect_that!(paths(&errors), elements_are![eq("location.city")]);
    expect_that!(
        messages_at(&errors, "location.city"),
        elements_are![eq("This field is required.")]
    );
}

#[gtest]
fn leaf_paths_are_qualified_through_nested_field_sets() {
    // `FieldSet` contributes its name as a prefix and nothing of its own —
    // there is no `location` entry, only the leaves beneath it.
    let mut form = empty_form::<EventForCreate>(FormSpec::default());
    expect_that!(form.validate(), none());

    expect_that!(
        paths(&form.collect_errors()),
        elements_are![
            eq("title"),
            eq("location.street"),
            eq("location.city"),
            eq("location.zip"),
        ]
    );
}

// ── VariantSet: the member with errors of its own ────────────────────────

#[gtest]
fn an_unchosen_enum_reports_at_its_own_path() {
    // The regression test for `collect_errors` having been written as a copy of
    // `collect_leaves`, which early-returns when unchosen. `validate` pushes
    // "you must choose a variant" PRECISELY in that state, so the early return
    // dropped the one error this member reliably produces.
    let mut form = empty_form::<Drawing>(FormSpec::default());
    form.apply_form_values(&[("name".to_string(), "Sketch".to_string())]);
    expect_that!(form.validate(), none());

    let errors = form.collect_errors();
    expect_that!(paths(&errors), elements_are![eq("shape")]);
    expect_that!(
        messages_at(&errors, "shape"),
        elements_are![eq("You must choose a variant for this field.")],
        "the enum's own error belongs at `shape`, NOT under a `$Variant` segment"
    );
}

#[gtest]
fn a_chosen_variants_fields_report_under_the_variant_segment() {
    // Chosen but unfilled: now the error is a leaf's, one segment deeper, and
    // the enum itself has nothing to say.
    let mut form = empty_form::<Drawing>(FormSpec::default());
    form.apply_form_values(&[("name".to_string(), "Sketch".to_string())]);
    form.edit(&Edit::new_choose_variant("shape", Some("Circle")))
        .expect("Circle is a variant of Shape");
    expect_that!(form.validate(), none());

    expect_that!(
        paths(&form.collect_errors()),
        elements_are![eq("shape.$Circle.radius")]
    );
}

#[gtest]
fn choosing_a_variant_clears_the_enums_own_error() {
    // Both halves of the previous two tests in sequence, to pin that the
    // shallow path and the deep one are alternatives rather than additive —
    // a form that reported both would be telling the user to choose a variant
    // they had already chosen.
    let mut form = empty_form::<Drawing>(FormSpec::default());
    form.apply_form_values(&[("name".to_string(), "Sketch".to_string())]);
    expect_that!(form.validate(), none());
    expect_that!(paths(&form.collect_errors()), contains(eq("shape")));

    form.edit(&Edit::new_choose_variant("shape", Some("Circle")))
        .expect("Circle is a variant of Shape");
    form.apply_form_values(&[
        ("name".to_string(), "Sketch".to_string()),
        ("shape.$Circle.radius".to_string(), "1.5".to_string()),
    ]);
    expect_that!(form.validate(), some(anything()));
    expect_that!(form.collect_errors().fields, is_empty());
}

// ── push_field_error / collect_errors round trip ─────────────────────────

#[gtest]
fn a_server_verdict_about_the_choice_itself_comes_back_out() {
    // The rule `collect_errors` follows: whatever `push_field_error` accepts at
    // a path has to be collectable from that path. `VariantSet` is the one
    // member where the two could disagree, because it accepts its OWN path.
    let mut form = empty_form::<Drawing>(FormSpec::default());
    form.push_field_error("shape", "That shape is not available on this plan.")
        .expect("the enum's own path is a legal target");

    expect_that!(
        messages_at(&form.collect_errors(), "shape"),
        elements_are![eq("That shape is not available on this plan.")]
    );
}

#[gtest]
fn a_server_verdict_survives_the_enum_being_unchosen() {
    // Same push, made explicit about the state that used to swallow it: nothing
    // has been chosen, so there is no child subtree to recurse into at all.
    let mut form = empty_form::<Drawing>(FormSpec::default());
    form.push_field_error("shape", "Pick a shape first.")
        .expect("the enum's own path is a legal target while unchosen");

    let errors = form.collect_errors();
    expect_that!(paths(&errors), elements_are![eq("shape")]);
    expect_that!(
        messages_at(&errors, "shape"),
        elements_are![eq("Pick a shape first.")]
    );
}

#[gtest]
fn a_server_verdict_on_a_leaf_round_trips_at_its_own_path() {
    let mut form = form_for(&filled_event(), FormSpec::default());
    form.push_field_error("location.zip", "No such ZIP code.")
        .expect("a qualified leaf path is a legal target");

    expect_that!(
        paths(&form.collect_errors()),
        elements_are![eq("location.zip")],
        "a pushed error must not drag its clean siblings along"
    );
}

// ── Lists and deeper nesting ─────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Tally {
    label: String,
    counts: Vec<u32>,
}

#[gtest]
fn list_rows_report_their_own_row_keys() {
    // Row identity, not position — `#1` is the key `ListSet::rows` assigned,
    // which is what makes an error survive a sibling row being removed.
    let tally = Tally {
        label: "Votes".to_string(),
        counts: vec![1, 2, 3],
    };
    let mut form = form_for(&tally, FormSpec::default());
    form.apply_form_values(&[("counts.#1".to_string(), "not a number".to_string())]);
    expect_that!(form.validate(), none());

    expect_that!(
        paths(&form.collect_errors()),
        elements_are![eq("counts.#1")]
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Sketch {
    name: String,
    shape: Option<Shape>,
}

#[gtest]
fn an_absent_optional_enum_reports_nothing() {
    // `OptionMember` is a pass-through decorator, but `validate` clears the
    // inner member's errors when absent — so the "choose a variant" error the
    // required case produces must NOT appear here.
    let mut form = empty_form::<Sketch>(FormSpec::default());
    form.apply_form_values(&[("name".to_string(), "Doodle".to_string())]);
    expect_that!(form.validate(), some(anything()));

    expect_that!(form.collect_errors().fields, is_empty());
}

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Inner {
    A { x: f64 },
}

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Outer {
    First { inner: Inner },
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Doc {
    outer: Outer,
}

#[gtest]
fn an_enum_inside_a_variant_reports_at_its_own_nested_path() {
    // Two `$Variant` segments deep. The inner enum starts `Unchosen` like any
    // other, so its own error lands at `outer.$First.inner` — the prefix it
    // inherited, plus its name, and no segment of its own.
    let mut form = empty_form::<Doc>(FormSpec::default());
    form.edit(&Edit::new_choose_variant("outer", Some("First")))
        .expect("First is a variant of Outer");
    expect_that!(form.validate(), none());

    expect_that!(
        paths(&form.collect_errors()),
        elements_are![eq("outer.$First.inner")]
    );
}

#[gtest]
fn a_nested_enums_leaf_error_carries_both_variant_segments() {
    let mut form = empty_form::<Doc>(FormSpec::default());
    form.edit(&Edit::new_choose_variant("outer", Some("First")))
        .expect("First is a variant of Outer");
    form.edit(&Edit::new_choose_variant("outer.$First.inner", Some("A")))
        .expect("A is a variant of Inner");
    expect_that!(form.validate(), none());

    expect_that!(
        paths(&form.collect_errors()),
        elements_are![eq("outer.$First.inner.$A.x")]
    );
}

// ── Form-level errors ────────────────────────────────────────────────────

fn label_must_not_be_shouted(tally: &Tally) -> Vec<FormError> {
    if tally.label.chars().all(|c| !c.is_lowercase()) {
        vec![FormError("Don't shout the label.".to_string())]
    } else {
        Vec::new()
    }
}

#[gtest]
fn the_specs_validator_lands_in_form_not_fields() {
    // The cross-field verdict has no path — it is a statement about the whole
    // model — so it belongs in `FormErrors.form`, and `fields` stays empty even
    // though the form failed. A caller that branched on `fields.is_empty()`
    // ALONE would call this a pass.
    let spec = FormSpec::default().with_validator(label_must_not_be_shouted);
    let tally = Tally {
        label: "VOTES".to_string(),
        counts: vec![1],
    };
    let mut form = form_for(&tally, spec);
    expect_that!(form.validate(), none());

    let errors = form.collect_errors();
    expect_that!(errors.fields, is_empty());
    expect_that!(
        errors.form.iter().map(|e| e.0.clone()).collect::<Vec<_>>(),
        elements_are![eq("Don't shout the label.")]
    );
}
