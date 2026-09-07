//! Enum fields and optional enums.
//!
//! **Rewritten 2026-09-05** when the variant became an in-form answer rather
//! than a construction parameter. Ten tests were deleted outright, not ported:
//! they exercised `required_variants` / `missing_variants` / the iterative
//! disclosure loop, machinery that exists only if construction has to be *told*
//! the answers up front. Their subject matter didn't vanish, it moved:
//!
//! | deleted test | what now covers it |
//! |---|---|
//! | variants listed in declaration order | the `<select>`'s options (widget layer) |
//! | an unknown variant name reports the real options | `choose_variant` returning `Err` |
//! | `Absent` satisfies an optional enum | `an_untouched_optional_enum_validates_as_none` |
//! | `Absent` rejected where not optional | `an_unchosen_required_enum_is_a_validation_error` |
//! | choosing a variant reveals the enums inside it | `choosing_a_variant_leaves_a_nested_enum_unchosen` |
//!
//! The disclosure *loop* has no successor because it no longer exists: a nested
//! enum simply starts `Unchosen` like any other, and the user answers it in the
//! form. That is the whole point of the change.
//!
//! The RED set that drove this rewrite — `validate()` reporting `Unchosen`, and
//! `Form::choose_variant` — all passes as of the same commit, and now stands as
//! the regression net for both.

use super::render_to_html;
use crate::reflect::*;
use std::collections::HashMap;
use dioxus::prelude::*;
use facet::Facet;
use super::models::{Mode, Shape};
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
pub struct Drawing {
    pub name: String,
    pub shape: Shape,
}

#[derive(Facet, Clone, Debug, PartialEq)]
pub struct Config {
    pub shape: Shape,
    pub mode: Mode,
}

#[derive(Facet, Clone, Debug, PartialEq)]
pub struct Outer {
    pub title: String,
    pub drawing: Drawing,
}

// An enum reachable only *through* another enum's variant — the shape that
// makes variant discovery iterative rather than one-shot.
#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Inner {
    A { x: f64 },
    B { y: f64 },
}

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum Outer2 {
    First { inner: Inner },
    Second { n: u32 },
}

#[derive(Facet, Clone, Debug, PartialEq)]
pub struct Doc {
    pub outer: Outer2,
}

/// An enum behind an `Option` — the case nothing covered until now.
#[derive(Facet, Clone, Debug, PartialEq)]
pub struct Sketch {
    pub name: String,
    pub shape: Option<Shape>,
}

// ── Edit mode: the value pins the variant, so nothing is ever unchosen ──

#[gtest]
fn form_for_round_trips_an_enum_field() {
    let drawing = Drawing {
        name: "Rect".to_string(),
        shape: Shape::Rectangle { width: 2.0, height: 4.0 },
    };
    let mut form = form_for(&drawing);
    expect_that!(form.validate(), some(eq(&drawing)));
}

#[gtest]
fn nested_enum_field_round_trips() {
    // The enum lives one struct deep — exercises the qualified path
    // (`drawing.shape.…`) through both construction and write_into.
    let outer = Outer {
        title: "T".to_string(),
        drawing: Drawing {
            name: "N".to_string(),
            shape: Shape::Circle { radius: 1.0 },
        },
    };
    let mut form = form_for(&outer);
    expect_that!(form.validate(), some(eq(&outer)));
}

#[gtest]
fn edit_mode_round_trips_an_optional_enum() {
    let sketch = Sketch {
        name: "Doodle".to_string(),
        shape: Some(Shape::Circle { radius: 1.5 }),
    };
    let mut form = form_for(&sketch);
    expect_that!(form.validate(), some(eq(&sketch)));
}

#[gtest]
fn edit_mode_round_trips_an_absent_optional_enum() {
    let sketch = Sketch { name: "Doodle".to_string(), shape: None };
    let mut form = form_for(&sketch);
    expect_that!(form.validate(), some(eq(&sketch)));
}

// ── Blank mode: everything starts Unchosen ──

#[gtest]
fn a_blank_form_starts_every_enum_unchosen() {
    // No caller supplies choices any more, so this is now infallible — the
    // single biggest consequence of the change.
    let form = empty_form::<Drawing>();
    let paths: Vec<String> = form.leaves().into_iter().map(|(p, _)| p).collect();
    expect_that!(
        paths,
        elements_are![eq("name")],
        "an unchosen variant has no fields to contribute: {paths:?}"
    );
}

#[gtest]
fn an_untouched_optional_enum_validates_as_none() {
    // Unchosen behind an `Option` is a legal, complete answer — `OptionMember`
    // suppresses the inner error exactly as it does for an empty scalar. This is
    // why two states suffice and `Absent` was not needed.
    let mut form = empty_form::<Sketch>();
    form.apply_form_values(&[("name".to_string(), "Doodle".to_string())]);
    expect_that!(
        form.validate(),
        some(eq(&Sketch { name: "Doodle".to_string(), shape: None }))
    );
}

/// A form has to be rendered inside a live runtime now — see
/// [`super::render_to_html`]. Each render test owns a tiny component like this
/// one because `Form<T>` isn't `PartialEq` and so can't be a component prop.
#[component]
fn UnchosenSketchForm() -> Element {
    let form = use_hook(empty_form::<Sketch>);
    let values = use_form_values(&form);
    form.render(values)
}

#[gtest]
fn unchosen_renders_a_visible_placeholder() {
    // Visible but inert, so leaving a value out is something the user can see
    // rather than a field silently vanishing. `disabled` also means the browser
    // won't submit it, so the placeholder text never comes back as a value.
    // (This is the spot the reactive `<select>` eventually takes over.)
    let html = render_to_html(UnchosenSketchForm);
    expect_that!(html, contains_substring(ABSENT_DISPLAY));
    expect_that!(html, contains_substring("disabled"));
}

#[component]
fn DocWithChosenOuter() -> Element {
    let form = use_hook(|| {
        let mut form = empty_form::<Doc>();
        form.choose_variant("outer", Some("First")).expect("First is a variant of Outer2");
        form
    });
    let values = use_form_values(&form);
    form.render(values)
}

#[gtest]
fn a_nested_unchosen_placeholder_is_named_by_its_qualified_path() {
    // Guards the fix that qualified this input's `name`. It has to be a NESTED
    // enum to mean anything: `Sketch.shape` sits at the root, where
    // `qualify("", "shape")` and the bare name are the same string, so it would
    // pass with the bug still in. `Doc` gives us `outer.inner`, which differs.
    //
    // Cosmetic today (the input is `disabled`, so it never submits) and a real
    // bug the moment this becomes a live `<select>`: a nested enum posting
    // under `inner` would never be found by `apply_leaves`.
    let html = render_to_html(DocWithChosenOuter);
    expect_that!(html, contains_substring(r#"name="outer.$First.inner""#));
    expect_that!(html, not(contains_substring(r#"name="inner""#)));
}

// ── RED: validate() must report an unchosen REQUIRED enum ──

#[gtest]
fn an_unchosen_required_enum_is_a_validation_error() {
    // The other half of "two states suffice": unchosen is an ordinary validation
    // failure, exactly as `Empty` is for a required scalar. Without this, a blank
    // form silently validates into a model with no variant selected — which it
    // can't, so it would panic in `write_value_into` instead.
    let mut form = empty_form::<Drawing>();
    form.apply_form_values(&[("name".to_string(), "My Drawing".to_string())]);
    expect_that!(form.validate(), none(), "an unchosen `shape` must not validate");
    expect_that!(form.has_errors(), eq(true));
}

// ── RED: choose_variant, the schema rebuild the select will trigger ──

#[gtest]
fn choosing_a_variant_reveals_its_fields() {
    let mut form = empty_form::<Drawing>();
    form.choose_variant("shape", Some("Circle")).expect("Circle is a variant of Shape");

    let paths: Vec<String> = form.leaves().into_iter().map(|(p, _)| p).collect();
    expect_that!(paths, contains(eq("shape.$Circle.radius")));

    form.apply_form_values(&[
        ("name".to_string(), "My Drawing".to_string()),
        ("shape.$Circle.radius".to_string(), "3.5".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&Drawing {
            name: "My Drawing".to_string(),
            shape: Shape::Circle { radius: 3.5 },
        }))
    );
}

#[gtest]
fn choosing_a_variant_behind_an_option_builds_a_some() {
    // The `begin_some` frame still has to happen, but `OptionMember` owns it now
    // rather than `VariantSet` knowing it is optional.
    let mut form = empty_form::<Sketch>();
    form.choose_variant("shape", Some("Circle")).expect("Circle is a variant of Shape");

    form.apply_form_values(&[
        ("name".to_string(), "Doodle".to_string()),
        ("shape.$Circle.radius".to_string(), "2.5".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&Sketch {
            name: "Doodle".to_string(),
            shape: Some(Shape::Circle { radius: 2.5 }),
        }))
    );
}

#[gtest]
fn choosing_an_unknown_variant_is_an_error() {
    // The successor to `an_unknown_variant_name_reports_the_real_options`. It has
    // to be an error rather than a panic: with a reactive select the name can
    // arrive from a stale client, not just from our own bug.
    let mut form = empty_form::<Drawing>();
    expect_that!(form.choose_variant("shape", Some("Hexagon")), err(anything()));
}

#[gtest]
fn choosing_a_variant_leaves_a_nested_enum_unchosen() {
    // What the disclosure loop used to test, minus the loop. `outer.inner` does
    // not exist until `outer` is answered — but now it simply appears, unchosen,
    // and the user answers it in the form. No pre-flight, no second round trip.
    let mut form = empty_form::<Doc>();
    form.choose_variant("outer", Some("First")).expect("First is a variant of Outer2");

    // `inner` is now reachable and unanswered, so it contributes no leaves yet…
    let paths: Vec<String> = form.leaves().into_iter().map(|(p, _)| p).collect();
    expect_that!(paths, not(contains(starts_with("outer.$First.inner."))));

    // …and answering it reveals its fields.
    form
        .choose_variant("outer.$First.inner", Some("A"))
        .expect("A is a variant of Inner");
    form.apply_form_values(&[("outer.$First.inner.$A.x".to_string(), "1.5".to_string())]);
    expect_that!(
        form.validate(),
        some(eq(&Doc { outer: Outer2::First { inner: Inner::A { x: 1.5 } } }))
    );
}

#[gtest]
fn switching_a_variant_replaces_the_subtree() {
    // The destructive switch, which is now a UX rule rather than something the
    // type system prevents: the old variant's fields are gone, not merged.
    let mut form = empty_form::<Drawing>();
    form.choose_variant("shape", Some("Circle")).expect("Circle is a variant");
    form.apply_form_values(&[("shape.$Circle.radius".to_string(), "3.5".to_string())]);

    form.choose_variant("shape", Some("Rectangle")).expect("Rectangle is a variant");
    let paths: Vec<String> = form.leaves().into_iter().map(|(p, _)| p).collect();
    expect_that!(paths, not(contains(eq("shape.$Circle.radius"))));
    expect_that!(paths, contains(eq("shape.$Rectangle.width")));
}

// ── Reaching the enum through other containers ──
//
// The tests above all reach a `VariantSet` that is either a direct member of the
// `Form` or nested in another `VariantSet`. These cover the two remaining
// dispatchers, which are otherwise never exercised.

#[derive(Facet, Clone, Debug, PartialEq)]
struct Gallery {
    shapes: Vec<Shape>,
}

#[gtest]
fn choosing_a_variant_through_a_field_set() {
    // `drawing.shape` — the path crosses a `FieldSet` on its way down.
    let mut form = empty_form::<Outer>();
    form.choose_variant("drawing.shape", Some("Circle"))
        .expect("Circle is a variant of Shape");

    form.apply_form_values(&[
        ("title".to_string(), "T".to_string()),
        ("drawing.name".to_string(), "N".to_string()),
        ("drawing.shape.$Circle.radius".to_string(), "1.0".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&Outer {
            title: "T".to_string(),
            drawing: Drawing {
                name: "N".to_string(),
                shape: Shape::Circle { radius: 1.0 },
            },
        }))
    );
}

#[gtest]
fn choosing_a_variant_on_one_list_row_leaves_the_others_alone() {
    // `shapes.1` — the path crosses a `ListSet`, whose rows are named by index.
    // Edit mode, because a blank form has no rows until create-mode lengths land.
    let gallery = Gallery {
        shapes: vec![
            Shape::Circle { radius: 1.0 },
            Shape::Circle { radius: 2.0 },
        ],
    };
    let mut form = form_for(&gallery);
    form.choose_variant("shapes.1", Some("Rectangle"))
        .expect("Rectangle is a variant of Shape");

    let paths: Vec<String> = form.leaves().into_iter().map(|(p, _)| p).collect();
    expect_that!(
        paths,
        elements_are![
            eq("shapes.0.$Circle.radius"),
            eq("shapes.1.$Rectangle.width"),
            eq("shapes.1.$Rectangle.height"),
        ],
        "row 0 keeps its variant and its value; only row 1 was rebuilt"
    );

    form.apply_form_values(&[
        ("shapes.1.$Rectangle.width".to_string(), "3.0".to_string()),
        ("shapes.1.$Rectangle.height".to_string(), "4.0".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&Gallery {
            shapes: vec![
                Shape::Circle { radius: 1.0 },
                Shape::Rectangle { width: 3.0, height: 4.0 },
            ],
        }))
    );
}

#[gtest]
fn a_bad_path_is_an_error_not_a_panic() {
    let mut form = empty_form::<Drawing>();
    expect_that!(form.choose_variant("nope", Some("Circle")), err(anything()));
    // A real field, but not an enum — worth distinguishing, since it means the
    // caller's path was right and its expectation wasn't.
    expect_that!(form.choose_variant("name", Some("Circle")), err(anything()));
}

// ── Variants that share a field name ─────────────────────────────────────

/// Two variants with a field of the same name. Without a variant segment in the
/// path they would both own `footprint.size`, and the value map — which
/// deliberately survives a structural edit — would hand the Circle's number to
/// the Square.
#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Footprint {
    Circle { size: f64 },
    Square { size: f64 },
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Plot {
    footprint: Footprint,
}

#[gtest]
fn a_variants_fields_are_namespaced_under_a_variant_segment() {
    // `$` can't begin a Rust identifier, so a descriptor segment can never
    // collide with a field name. Strip the `$` segments and the path mirrors
    // the model again: `footprint.size`.
    let mut form = empty_form::<Plot>();
    form
        .choose_variant("footprint",Some("Circle"))
        .expect("Circle is a variant of Footprint");

    let paths: Vec<String> = form.leaves().into_iter().map(|(p, _)| p).collect();
    expect_that!(paths, elements_are![eq("footprint.$Circle.size")]);
}

#[gtest]
fn switching_variants_does_not_inherit_a_same_named_field() {
    // Deliberately written without hard-coded paths, so it states the *bug*
    // rather than the fix: whatever the paths are, a value typed into Circle
    // must not reappear in Square.
    let mut form = empty_form::<Plot>();
    form
        .choose_variant("footprint", Some("Circle"))
        .expect("Circle is a variant of Footprint");

    // Type into every leaf Circle offers, then snapshot the value map.
    let typed: HashMap<String, String> = form
        .leaves()
        .into_iter()
        .map(|(p, _)| (p, "5".to_string()))
        .collect();
    form.apply(&typed);
    let store: HashMap<String, String> = form.leaves().into_iter().collect();

    // Switch. The store is keyed by path and survives structural edits by
    // design, so the Circle's entry is still sitting in it.
    form
        .choose_variant("footprint", Some("Square"))
        .expect("Square is a variant of Footprint");
    form.apply(&store);

    // Nothing was ever typed into Square's `size`, so the form must not build.
    expect_that!(form.validate(), none());
}

// ── Paths still mirror the model, modulo `$` segments ────────────────────

#[gtest]
fn model_path_strips_variant_descriptors() {
    // The rule that keeps namespacing from costing us path/model mirroring:
    // drop every `$` segment and what's left is the path through the model.
    expect_that!(model_path("footprint.$Circle.size"), eq("footprint.size"));
    expect_that!(model_path("outer.$First.inner.$A.x"), eq("outer.inner.x"));

    // A list index is part of the model path and must survive.
    expect_that!(model_path("shapes.0.$Circle.radius"), eq("shapes.0.radius"));

    // Nothing to strip.
    expect_that!(model_path("location.street"), eq("location.street"));
    expect_that!(model_path("title"), eq("title"));
}

#[gtest]
fn every_leaf_path_maps_back_onto_the_model() {
    // The property the unit cases above are examples of, checked against a real
    // form: no model path retains a descriptor, and each one names a chain of
    // real fields — here `shape.radius`, which is exactly how you'd reach the
    // value in `Drawing` itself.
    let mut form = empty_form::<Drawing>();
    form.choose_variant("shape", Some("Circle")).expect("Circle is a variant");

    let model: Vec<String> = form
        .leaves()
        .into_iter()
        .map(|(p, _)| model_path(&p))
        .collect();

    expect_that!(model, each(not(contains_substring("$"))));
    expect_that!(model, contains(eq("shape.radius")));
}

// ── Edits aimed at the wrong kind of member ──────────────────────────────

#[gtest]
fn a_row_edit_aimed_at_an_enum_is_rejected() {
    // New with `Edit`: previously `choose_variant` was the only edit, so
    // "right path, wrong kind of member" couldn't be expressed at all.
    let mut form = empty_form::<Drawing>();
    let error = form
        .edit(&Edit::AddRow { path: "shape".to_string() })
        .expect_err("an enum has no rows");
    expect_that!(error.0, contains_substring("shape is an enum, not a list"));
}

#[gtest]
fn a_variant_edit_aimed_at_a_scalar_is_rejected() {
    // Reaching a real field means the caller's path was right and its
    // *expectation* was wrong — worth saying differently from "no such path".
    let mut form = empty_form::<Drawing>();
    let error = form
        .choose_variant("name", Some("Circle"))
        .expect_err("a String field is not an enum");
    expect_that!(error.0, contains_substring("name is a field, not an enum"));
}

// ── Unsetting a variant ──────────────────────────────────────────────────

#[gtest]
fn an_optional_enum_can_be_unset_after_being_chosen() {
    // `choose_variant(path, None)` is the `--none--` option of an optional
    // enum's `<select>`. Without it there was no way back to `Unchosen` — you
    // could answer the question but never un-answer it.
    let mut form = empty_form::<Sketch>();
    form.choose_variant("shape", Some("Circle")).expect("Circle is a variant of Shape");
    form.apply_form_values(&[
        ("name".to_string(), "Doodle".to_string()),
        ("shape.$Circle.radius".to_string(), "2.5".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&Sketch {
            name: "Doodle".to_string(),
            shape: Some(Shape::Circle { radius: 2.5 }),
        }))
    );

    form.choose_variant("shape", None).expect("an optional enum can be cleared");
    expect_that!(
        form.validate(),
        some(eq(&Sketch {
            name: "Doodle".to_string(),
            shape: None,
        }))
    );
}

#[gtest]
fn unsetting_drops_the_subtree_from_the_form() {
    // The members go, so the enum contributes no leaves and `is_present` reads
    // false — which is what lets `OptionMember` write a `None`.
    let mut form = empty_form::<Sketch>();
    form.choose_variant("shape", Some("Circle")).expect("Circle is a variant of Shape");
    let chosen: Vec<String> = form.leaves().into_iter().map(|(p, _)| p).collect();
    expect_that!(chosen, contains(eq("shape.$Circle.radius")));

    form.choose_variant("shape", None).expect("an optional enum can be cleared");
    let cleared: Vec<String> = form.leaves().into_iter().map(|(p, _)| p).collect();
    expect_that!(cleared, elements_are![eq("name")]);
}

#[gtest]
fn unsetting_a_required_enum_is_structurally_legal_but_fails_validation() {
    // Clearing is always allowed on the way through — `validate()` is what
    // decides whether leaving it unanswered is an error, exactly as it does for
    // an enum that was never answered at all. `Drawing.shape` is a bare `Shape`,
    // with no `Option` around it.
    let mut form = empty_form::<Drawing>();
    form.choose_variant("shape", Some("Circle")).expect("Circle is a variant");
    form.apply_form_values(&[
        ("name".to_string(), "My Drawing".to_string()),
        ("shape.$Circle.radius".to_string(), "3.5".to_string()),
    ]);
    // Fully answered first, so the `none()` below can only be the clearing —
    // otherwise this would pass just as well against an unset that did nothing.
    expect_that!(form.validate(), some(anything()));

    form.choose_variant("shape", None).expect("clearing is structurally legal");
    expect_that!(form.validate(), none());
}

#[gtest]
fn unsetting_then_rechoosing_restores_what_was_typed() {
    // The payoff of the `$Variant` segment. The value map survives structural
    // edits, and clearing doesn't touch it, so the radius is still sitting under
    // `shape.$Circle.radius` when the user changes their mind.
    let mut form = empty_form::<Sketch>();
    form.choose_variant("shape", Some("Circle")).expect("Circle is a variant of Shape");
    form.apply_form_values(&[
        ("name".to_string(), "Doodle".to_string()),
        ("shape.$Circle.radius".to_string(), "2.5".to_string()),
    ]);
    let store: HashMap<String, String> = form.leaves().into_iter().collect();

    form.choose_variant("shape", None).expect("an optional enum can be cleared");
    // Pin the intermediate state, so this can't pass against an unset that
    // silently did nothing.
    expect_that!(form.validate(), some(eq(&Sketch { name: "Doodle".to_string(), shape: None })));

    form.choose_variant("shape", Some("Circle")).expect("and chosen again");
    form.apply(&store);

    expect_that!(
        form.validate(),
        some(eq(&Sketch {
            name: "Doodle".to_string(),
            shape: Some(Shape::Circle { radius: 2.5 }),
        }))
    );
}
