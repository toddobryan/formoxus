//! `FormSpec` overrides reaching the built tree, via `FormMember::apply_specs`.
//!
//! These go through `render_to_html` rather than inspecting members, because the
//! question is whether an override *arrives* — and the DOM is the only place that
//! answer is unambiguous. Nothing here constructs a spec through `form2!`; the
//! builder is the contract between the macro and the tree, so testing the builder
//! keeps these honest about which half is being exercised.

use super::models::{EventForCreate, Location};
use super::render_to_html;
use crate::reflect::widgets::{ControlType, InputType};
use crate::reflect::*;
use dioxus::prelude::*;
use facet::Facet;
use googletest::prelude::*;

/// A `bool` and an `Option<bool>` side by side — the two shapes whose *derived*
/// controls differ, which is what makes them the useful subjects for testing
/// precedence. A plain `bool` derives a checkbox, an `Option<bool>` a tri-state
/// select.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Flags {
    enabled: bool,
    subscribed: Option<bool>,
}

// ── Labels ───────────────────────────────────────────────────────────────

#[component]
fn RelabelledField() -> Element {
    let form = use_form(empty_form(
        FormSpec::<EventForCreate>::default().with_label("title", "Event name"),
    ));
    form.render()
}

#[gtest]
fn a_label_override_beats_the_humanized_field_name() {
    // `default_label` would make this "Title"; the spec has to win.
    let html = render_to_html(RelabelledField);
    expect_that!(html, contains_substring("Event name"));
    expect_that!(html, not(contains_substring(">Title<")));
}

#[component]
fn RelabelledNestedField() -> Element {
    let form = use_form(empty_form(
        FormSpec::<EventForCreate>::default().with_label("location.city", "Town"),
    ));
    form.render()
}

#[gtest]
fn a_nested_path_reaches_exactly_one_field() {
    // The case that fails outright if a container doesn't recurse, and smears if
    // it recurses with the wrong prefix. Asserting the siblings kept their
    // defaults is what separates those two failures from success.
    let html = render_to_html(RelabelledNestedField);
    expect_that!(html, contains_substring("Town"));
    expect_that!(html, contains_substring("Street"));
    expect_that!(html, contains_substring("Zip"));
    expect_that!(html, not(contains_substring(">City<")));
}

#[component]
fn RelabelledFieldSet() -> Element {
    let form = use_form(empty_form(
        FormSpec::<EventForCreate>::default().with_label("location", "Where"),
    ));
    form.render()
}

#[gtest]
fn a_field_sets_own_label_becomes_its_legend() {
    // A container is addressable in its own right, not just as a prefix — so
    // `apply_specs` has to consult the map about itself before recursing.
    let html = render_to_html(RelabelledFieldSet);
    expect_that!(html, contains_substring("<legend>Where</legend>"));
}

// ── Controls ─────────────────────────────────────────────────────────────

#[component]
fn UntouchedFlags() -> Element {
    let form = use_form(empty_form(FormSpec::<Flags>::default()));
    form.render()
}

#[gtest]
fn the_derived_controls_are_what_the_override_has_to_beat() {
    // Baseline, so the two tests below are measuring a change rather than a
    // coincidence: `enabled` is a checkbox and `subscribed` a select.
    let html = render_to_html(UntouchedFlags);
    expect_that!(html.matches(r#"type="checkbox""#).count(), eq(1));
    expect_that!(html.matches("<select").count(), eq(1));
}

#[component]
fn OptionalBoolForcedToCheckbox() -> Element {
    let form = use_form(empty_form(
        FormSpec::<Flags>::default().with_custom_control("subscribed", ControlType::Checkbox),
    ));
    form.render()
}

#[gtest]
fn an_override_beats_a_derived_control() {
    // `Option<bool>` derives `Select`. Forcing a checkbox is the strongest form
    // of the precedence rule: the override has to displace a control the shape
    // actively chose, not merely fill a gap.
    //
    // (Whether this is a GOOD idea is another matter — a checkbox can't express
    // the third state, so `None` becomes unreachable. The point is that the spec
    // is obeyed, which is what makes the author responsible for the choice.)
    let html = render_to_html(OptionalBoolForcedToCheckbox);
    expect_that!(html.matches(r#"type="checkbox""#).count(), eq(2));
    expect_that!(html.matches("<select").count(), eq(0));
}

#[component]
fn BoolForcedToSelect() -> Element {
    let form = use_form(empty_form(
        FormSpec::<Flags>::default().with_custom_control("enabled", ControlType::Select),
    ));
    form.render()
}

#[gtest]
fn an_override_works_in_the_other_direction_too() {
    let html = render_to_html(BoolForcedToSelect);
    expect_that!(html.matches("<select").count(), eq(2));
    expect_that!(html.matches(r#"type="checkbox""#).count(), eq(0));
}

// ── Rejections and no-ops ────────────────────────────────────────────────

#[gtest]
fn an_unmatched_path_is_a_no_op() {
    // The witness fn makes this unreachable through `form2!`, but the builder is
    // public and `apply_specs` must not panic on a path it simply doesn't find —
    // a spec is a set of statements about members, not an assertion that each
    // exists.
    let form = empty_form(
        FormSpec::<EventForCreate>::default().with_label("nowhere.at.all", "Ignored"),
    );
    expect_that!(form.leaves().len(), gt(0));
}

#[gtest]
#[should_panic(expected = "location is a field set")]
fn a_control_on_a_field_set_is_rejected() {
    // A field set has no single control to be, and silently ignoring the entry
    // would leave the author with no thread to pull. The message names the path
    // because a nested form has more than one candidate.
    let _ = empty_form(
        FormSpec::<EventForCreate>::default()
            .with_custom_control("location", ControlType::Textarea),
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct WithRows {
    venues: Vec<Location>,
}

#[gtest]
#[should_panic(expected = "venues is a list")]
fn a_control_on_a_list_is_rejected() {
    let _ = empty_form(FormSpec::<WithRows>::default().with_custom_control("venues", ControlType::Select));
}

/// The chooser has to be a FIELD, not the whole model: a bare-enum `T` has no
/// variant chosen in blank mode, and `fields_from_enum` refuses to build one.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Drawing {
    shape: super::models::Shape,
}

#[component]
fn ChooserWithAControl() -> Element {
    let form = use_form(empty_form(
        FormSpec::<Drawing>::default().with_custom_control("shape", ControlType::RadioGroup),
    ));
    form.render()
}

#[gtest]
fn a_control_on_a_variant_chooser_is_accepted() {
    // The one container that DOES own a control — the `<select>` that picks the
    // variant. The asymmetry with the two rejections above is deliberate.
    //
    // Nothing renders a radio group yet, so what this pins is that the override
    // is STORED and ignored rather than refused: construction succeeds and the
    // select still renders. It starts failing the day `RadioGroup` is honoured,
    // which is the right moment to be told.
    let html = render_to_html(ChooserWithAControl);
    expect_that!(html.matches("<select").count(), eq(1));
}

// ── `[]` — selecting every row of a list ─────────────────────────────────
//
// All of these go through `form_for` rather than `empty_form`: row count isn't in
// the shape, so a blank form has no rows to select and the assertions would hold
// vacuously.

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    answers: Vec<String>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    venues: Vec<Location>,
}

fn quiz() -> Quiz {
    Quiz { answers: vec!["PNG".to_string(), "JPEG".to_string(), "GIF".to_string()] }
}

fn trip() -> Trip {
    Trip {
        venues: vec![
            Location { street: "1 A St".to_string(), city: "Springfield".to_string(), zip: "11111".to_string() },
            Location { street: "2 B St".to_string(), city: "Shelbyville".to_string(), zip: "22222".to_string() },
        ],
    }
}

#[component]
fn RowsWithLabels() -> Element {
    let form = use_form(form_for(
        &quiz(),
        FormSpec::<Quiz>::default().with_label("answers[]", "Answer"),
    ));
    form.render()
}

#[gtest]
fn a_bracket_selector_reaches_every_row() {
    // Rows normally get NO label — `default_label` returns `None` for a `#`-keyed
    // name — so every occurrence here is one the selector put there. Counting is
    // what distinguishes "reached every row" from "reached the first one".
    //
    // Delimited rather than bare: the list's OWN derived legend is "Answers",
    // which contains "Answer", so a substring count would find four.
    let html = render_to_html(RowsWithLabels);
    expect_that!(html.matches(">Answer<").count(), eq(3));
}

#[component]
fn RowFieldsWithLabels() -> Element {
    let form = use_form(form_for(
        &trip(),
        FormSpec::<Trip>::default().with_label("venues[].city", "Town"),
    ));
    form.render()
}

#[gtest]
fn a_bracket_selector_composes_with_a_field_below_it() {
    // The case `[]` exists for. Two rows, so two "Town"s — and the siblings must
    // keep their derived labels, which is what separates a correct substitution
    // from one that smears across the whole row.
    let html = render_to_html(RowFieldsWithLabels);
    expect_that!(html.matches("Town").count(), eq(2));
    expect_that!(html.matches("Street").count(), eq(2));
    expect_that!(html, not(contains_substring(">City<")));
}

#[component]
fn ListWithItsOwnLabel() -> Element {
    let form = use_form(form_for(
        &trip(),
        FormSpec::<Trip>::default().with_label("venues", "Where we went"),
    ));
    form.render()
}

#[gtest]
fn a_bare_list_path_labels_the_list_not_the_rows() {
    // The distinction the bracket buys: without it, `venues` is the `ListSet`
    // itself, so exactly one legend and nothing per row.
    let html = render_to_html(ListWithItsOwnLabel);
    expect_that!(html.matches("Where we went").count(), eq(1));
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Answers {
    correct: Vec<bool>,
}

#[component]
fn RowControlsOverridden() -> Element {
    let form = use_form(form_for(
        &Answers { correct: vec![true, false] },
        FormSpec::<Answers>::default().with_custom_control("correct[]", ControlType::Select),
    ));
    form.render()
}

#[gtest]
fn a_bracket_selector_can_set_each_rows_control() {
    // A `Vec<bool>` derives a checkbox per row; the selector replaces all of
    // them. This is the half Todd's reading of `ListSet` predicted: a list has no
    // control of its own, but it has N rows that each do.
    let html = render_to_html(RowControlsOverridden);
    expect_that!(html.matches("<select").count(), eq(2));
    expect_that!(html.matches(r#"type="checkbox""#).count(), eq(0));
}

#[gtest]
#[should_panic(expected = "write `venues[]` to give every ROW a control")]
fn a_control_on_the_list_itself_now_suggests_the_bracket() {
    // The rejection still stands, but it can name the fix now.
    let _ = form_for(
        &trip(),
        FormSpec::<Trip>::default().with_custom_control("venues", ControlType::Select),
    );
}

// ── Members that arrive AFTER the spec was applied ───────────────────────

#[component]
fn RowAddedAfterTheSpec() -> Element {
    // The edit runs on the state before `use_form` takes it, which is the same
    // funnel a button click goes through — `FormState::edit`.
    let form = use_form({
        let mut state = form_for(
            &quiz(),
            FormSpec::<Quiz>::default().with_label("answers[]", "Answer"),
        );
        state
            .edit(&Edit::AddRow { path: "answers".to_string(), before: None })
            .expect("appending to `answers` should succeed");
        state
    });
    form.render()
}

#[gtest]
fn a_row_added_after_mount_still_gets_the_spec() {
    // `add_row` builds the new row from the shape alone, so without re-applying
    // it would come up with a derived label — none, for a `#`-keyed row — while
    // the three original rows carry "Answer". Four rows, four labels.
    //
    // This is the case that makes re-application load-bearing, and it only became
    // reachable when `[]` made a row's fields addressable at all.
    let html = render_to_html(RowAddedAfterTheSpec);
    expect_that!(html.matches(">Answer<").count(), eq(4));
}

#[component]
fn RowFieldAddedAfterTheSpec() -> Element {
    let form = use_form({
        let mut state = form_for(
            &trip(),
            FormSpec::<Trip>::default().with_label("venues[].city", "Town"),
        );
        state
            .edit(&Edit::AddRow { path: "venues".to_string(), before: None })
            .expect("appending to `venues` should succeed");
        state
    });
    form.render()
}

#[gtest]
fn a_new_rows_nested_field_gets_the_spec_too() {
    // One level deeper: the new row is a whole `FieldSet`, and the selector has to
    // reach inside it. Three "Town"s for two original rows plus the new one.
    let html = render_to_html(RowFieldAddedAfterTheSpec);
    expect_that!(html.matches("Town").count(), eq(3));
}
