//! A spec that builds a leaf: `FormSpec::with_node`, what `field:` and a typed
//! field's settings (`min: 3`) will expand to.
//!
//! The shape walk builds the leaf from the constructor while it walks, so
//! nothing is mutated afterwards. Each test names a leaf inside a `Vec` row,
//! gives it a field the default walk would never build (`CapField`, with a
//! setting baked in, the way `min: 3` would be), and checks the field's own
//! `check` runs. A row's leaves come to exist three ways, so the spec has
//! three routes to reach: built by `form_for` from a value, added by `AddRow`,
//! and revealed by `ChooseVariant` inside a row.

use facet::Facet;
use formoxus::error::ValidationMessage;
use formoxus::field::{Field, Widget};
use formoxus::field_node::NodeCtor;
use formoxus::nodes::Edit;
use formoxus::prelude::*;
use googletest::prelude::*;

#[derive(Clone, Debug, PartialEq, Facet)]
struct Venue {
    city: String,
    capacity: u32,
}

#[derive(Clone, Debug, PartialEq, Facet)]
struct Trip {
    venues: Vec<Venue>,
}

#[derive(Clone, Debug, PartialEq, Facet)]
#[repr(u8)]
#[allow(dead_code)]
enum Shape {
    Circle { radius: u32 },
    Square { side: u32 },
}

#[derive(Clone, Debug, PartialEq, Facet)]
struct Drawing {
    shapes: Vec<Shape>,
}

/// A field from outside formoxus, with a setting of its own. The message says
/// `(ctor)` so a test can tell this check from any built-in one.
#[derive(Clone, Debug, PartialEq)]
struct CapField {
    max: u32,
}

impl Field for CapField {
    type Value = u32;
    type Context = ();

    fn check(&self, value: &u32) -> Vec<ValidationMessage> {
        if *value > self.max {
            vec![ValidationMessage(format!("at most {} (ctor)", self.max))]
        } else {
            Vec::new()
        }
    }

    #[expect(clippy::todo, reason = "no `Widget` can be built until Step 5")]
    fn default_widget(&self, _optional: bool) -> Box<dyn Widget> {
        todo!()
    }
}

/// What `form!` would emit for `capacity => { field: CapField { max: 100 } }`.
fn cap_at_100() -> NodeCtor {
    NodeCtor::new(|args| Box::new(args.node(CapField { max: 100 })))
}

/// The errors at `path` after submitting `values`.
fn errors_at<T>(mut state: FormState<T>, values: &[(&str, &str)], path: &str) -> Vec<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    let values: Vec<(String, String)> = values
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    state.distribute_form_values(&values);
    let _ = state.validate();
    state
        .collect_errors()
        .fields
        .get(path)
        .map(|errors| errors.iter().map(|e| e.0.clone()).collect())
        .unwrap_or_default()
}

/// The control: without the spec, 500 is a perfectly good `u32`.
#[gtest]
fn without_a_constructor_the_default_field_takes_any_u32() {
    let trip = Trip {
        venues: vec![Venue {
            city: "Paris".into(),
            capacity: 5,
        }],
    };
    let state = form_for(&trip, FormSpec::default());
    expect_that!(
        errors_at(
            state,
            &[("venues.#0.city", "Paris"), ("venues.#0.capacity", "500")],
            "venues.#0.capacity",
        ),
        is_empty()
    );
}

/// Route 1: a row `form_for` builds from the value.
#[gtest]
fn a_constructor_reaches_a_row_built_from_the_value() {
    let trip = Trip {
        venues: vec![Venue {
            city: "Paris".into(),
            capacity: 5,
        }],
    };
    let spec = FormSpec::default().with_node("venues[].capacity", cap_at_100());
    let state = form_for(&trip, spec);
    expect_that!(
        errors_at(
            state,
            &[("venues.#0.city", "Paris"), ("venues.#0.capacity", "500")],
            "venues.#0.capacity",
        ),
        elements_are![eq("at most 100 (ctor)")]
    );
}

/// Route 2: a row added after the form exists, which `AddRow` builds through
/// the same walk, spec and all.
#[gtest]
fn a_constructor_reaches_a_row_added_later() {
    let spec = FormSpec::default().with_node("venues[].capacity", cap_at_100());
    let mut state = empty_form::<Trip>(spec);
    state
        .edit(&Edit::AddRow {
            path: "venues".into(),
            before: None,
        })
        .unwrap();
    expect_that!(
        errors_at(
            state,
            &[("venues.#0.city", "Paris"), ("venues.#0.capacity", "500")],
            "venues.#0.capacity",
        ),
        elements_are![eq("at most 100 (ctor)")]
    );
}

/// Route 3a: a variant's field inside a row, revealed by choosing the variant
/// after the row was added.
#[gtest]
fn a_constructor_reaches_a_variant_chosen_inside_an_added_row() {
    let spec = FormSpec::default().with_node("shapes[].$Circle.radius", cap_at_100());
    let mut state = empty_form::<Drawing>(spec);
    state
        .edit(&Edit::AddRow {
            path: "shapes".into(),
            before: None,
        })
        .unwrap();
    state
        .edit(&Edit::new_choose_variant("shapes.#0", Some("Circle")))
        .unwrap();
    expect_that!(
        errors_at(
            state,
            &[("shapes.#0", "Circle"), ("shapes.#0.$Circle.radius", "500")],
            "shapes.#0.$Circle.radius",
        ),
        elements_are![eq("at most 100 (ctor)")]
    );
}

/// Route 3b: the same leaf built by `form_for` from a value. This is the one
/// that needed `enum_node` to build a variant's fields under `$Circle`, the
/// path they really live at.
#[gtest]
fn a_constructor_reaches_a_variant_built_from_the_value() {
    let drawing = Drawing {
        shapes: vec![Shape::Circle { radius: 5 }],
    };
    let spec = FormSpec::default().with_node("shapes[].$Circle.radius", cap_at_100());
    let state = form_for(&drawing, spec);
    expect_that!(
        errors_at(
            state,
            &[("shapes.#0", "Circle"), ("shapes.#0.$Circle.radius", "500")],
            "shapes.#0.$Circle.radius",
        ),
        elements_are![eq("at most 100 (ctor)")]
    );
}

/// The spec names only `Circle`'s radius, so `Square`'s side keeps the default.
#[gtest]
fn a_constructor_for_one_variant_leaves_the_other_alone() {
    let drawing = Drawing {
        shapes: vec![Shape::Square { side: 5 }],
    };
    let spec = FormSpec::default().with_node("shapes[].$Circle.radius", cap_at_100());
    let state = form_for(&drawing, spec);
    expect_that!(
        errors_at(
            state,
            &[("shapes.#0", "Square"), ("shapes.#0.$Square.side", "500")],
            "shapes.#0.$Square.side",
        ),
        is_empty()
    );
}
