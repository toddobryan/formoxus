//! A spec that builds a leaf: `FormSpec::with_node`, what `field:` and a typed
//! field's settings (`min: 3`) will expand to.
//!
//! The shape walk builds the leaf from the constructor while it walks, so
//! nothing is mutated afterwards. Each test names a leaf inside a `Vec` item,
//! gives it a field the default walk would never build (`CapField`, with a
//! setting baked in, the way `min: 3` would be), and checks the field's own
//! `check` runs. An item's leaves come to exist three ways, so the spec has
//! three routes to reach: built by `form_for` from a value, added by `AddItem`,
//! and revealed by `ChooseVariant` inside an item.

use facet::Facet;
use formoxus::attrs::FieldType;
use formoxus::error::ValidationMessage;
use formoxus::field::Field;
use formoxus::field_kind::choice_field_for;
use formoxus::nodes::Edit;
use formoxus::nodes::NodeCtor;
use formoxus::path::NodePath;
use formoxus::prelude::*;
use formoxus::widgets::{AnyWidget, TextInput};
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

    const KIND: FieldType = FieldType::Int;

    fn check(&self, value: &u32) -> Vec<ValidationMessage> {
        if *value > self.max {
            vec![ValidationMessage(format!("at most {} (ctor)", self.max))]
        } else {
            Vec::new()
        }
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
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
    let values: Vec<(NodePath, String)> = values
        .iter()
        .map(|(k, v)| (NodePath::new(k.to_string()), v.to_string()))
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

/// Route 1: an item `form_for` builds from the value.
#[gtest]
fn a_constructor_reaches_an_item_built_from_the_value() {
    let trip = Trip {
        venues: vec![Venue {
            city: "Paris".into(),
            capacity: 5,
        }],
    };
    let spec = FormSpec::default().with_node(&path!(Trip.venues[].capacity), cap_at_100());
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

/// Route 2: an item added after the form exists, which `AddItem` builds through
/// the same walk, spec and all.
#[gtest]
fn a_constructor_reaches_an_item_added_later() {
    let spec = FormSpec::default().with_node(&path!(Trip.venues[].capacity), cap_at_100());
    let mut state = empty_form::<Trip>(spec);
    state
        .edit(&Edit::AddItem {
            path: NodePath::new("venues"),
            before: None,
            value: None,
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

/// Route 3a: a variant's field inside an item, revealed by choosing the variant
/// after the item was added.
#[gtest]
fn a_constructor_reaches_a_variant_chosen_inside_an_added_item() {
    let spec =
        FormSpec::default().with_node(&path!(Drawing.shapes[].$Shape::Circle.radius), cap_at_100());
    let mut state = empty_form::<Drawing>(spec);
    state
        .edit(&Edit::AddItem {
            path: NodePath::new("shapes"),
            before: None,
            value: None,
        })
        .unwrap();
    state
        .edit(&Edit::new_choose_variant(
            &NodePath::new("shapes.#0"),
            Some("Circle"),
        ))
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
    let spec =
        FormSpec::default().with_node(&path!(Drawing.shapes[].$Shape::Circle.radius), cap_at_100());
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
    let spec =
        FormSpec::default().with_node(&path!(Drawing.shapes[].$Shape::Circle.radius), cap_at_100());
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

// ── Choices: `choice_field_for` ──────────────────────────────────────────
//
// What `form!` emits for `choices:`. The model field's type picks what the
// field chooses among, and the field's own `check` enforces the list.

#[derive(Clone, Debug, PartialEq, Facet)]
struct Order {
    size: u8,
    gift_wrap: Option<u8>,
    state: String,
}

const STATES: &[(&str, &str)] = &[("AL", "Alabama"), ("AK", "Alaska")];

fn order() -> Order {
    Order {
        size: 1,
        gift_wrap: None,
        state: "AL".into(),
    }
}

fn order_values<'a>(size: &'a str, state: &'a str) -> [(&'a str, &'a str); 3] {
    [("size", size), ("gift_wrap", ""), ("state", state)]
}

/// `(1, "Small")` is a `u8` here, inferred from the model field.
#[gtest]
fn choices_over_a_number_field_are_that_number() {
    let spec = || {
        FormSpec::default().with_node(
            &path!(Order.size),
            choice_field_for(|m: &Order| &m.size, [(1, "Small"), (2, "Large")]),
        )
    };
    expect_that!(
        errors_at(form_for(&order(), spec()), &order_values("2", "AL"), "size"),
        is_empty()
    );
    expect_that!(
        errors_at(form_for(&order(), spec()), &order_values("3", "AL"), "size"),
        elements_are![eq("not one of the available choices")]
    );
}

/// An `Option<u8>` chooses among `u8`s; leaving it empty is still allowed.
#[gtest]
fn an_optional_field_chooses_among_what_it_holds() {
    let spec = || {
        FormSpec::default().with_node(
            &path!(Order.gift_wrap?),
            choice_field_for(|m: &Order| &m.gift_wrap, [(1, "Paper"), (2, "Box")]),
        )
    };
    let with_wrap = [("size", "1"), ("gift_wrap", "9"), ("state", "AL")];
    expect_that!(
        errors_at(form_for(&order(), spec()), &with_wrap, "gift_wrap"),
        elements_are![eq("not one of the available choices")]
    );
    expect_that!(
        errors_at(
            form_for(&order(), spec()),
            &order_values("1", "AL"),
            "gift_wrap"
        ),
        is_empty()
    );
}

/// A `const` table of `&str` pairs, the way choices are usually written,
/// works for a `String` field.
#[gtest]
fn a_const_str_table_serves_a_string_field() {
    let spec = || {
        FormSpec::default().with_node(
            &path!(Order.state),
            choice_field_for(|m: &Order| &m.state, STATES),
        )
    };
    expect_that!(
        errors_at(
            form_for(&order(), spec()),
            &order_values("1", "AK"),
            "state"
        ),
        is_empty()
    );
    expect_that!(
        errors_at(
            form_for(&order(), spec()),
            &order_values("1", "TX"),
            "state"
        ),
        elements_are![eq("not one of the available choices")]
    );
}

/// Through a `[]` item, as `form!` projects one.
#[gtest]
fn choices_reach_an_item() {
    let trip = Trip {
        venues: vec![Venue {
            city: "Paris".into(),
            capacity: 5,
        }],
    };
    let spec = FormSpec::default().with_node(
        &path!(Trip.venues[].city),
        choice_field_for(
            |m: &Trip| &formoxus::field_kind::item(m.venues.iter()).city,
            [("Paris", "Paris"), ("Rome", "Rome")],
        ),
    );
    expect_that!(
        errors_at(
            form_for(&trip, spec),
            &[("venues.#0.city", "Oslo"), ("venues.#0.capacity", "5")],
            "venues.#0.city",
        ),
        elements_are![eq("not one of the available choices")]
    );
}

// ── A named field wins over any shape ───────────────────────────────────
//
// `node_for_shape` asks the spec before it looks at the shape, so a
// constructor builds the leaf on any path, not only a scalar one: a struct
// becomes one input instead of a fieldset, and a newtype hands the field the
// newtype itself instead of the scalar inside it. These go through
// `with_node`, with no `form!`, so they prove the walk on its own.

/// A struct the default walk would build as a fieldset of two inputs.
#[derive(Clone, Debug, PartialEq, Facet)]
struct Span {
    start: u32,
    end: u32,
}

/// Reads and writes a `Span` as ONE text input, `3-7`.
#[derive(Clone, Debug, PartialEq)]
struct SpanField;

impl Field for SpanField {
    type Value = Span;
    type Context = ();

    const KIND: FieldType = FieldType::Text;

    fn parse(&self, raw: &str) -> Result<Span, ValidationMessage> {
        let not_a_span = || ValidationMessage::from("expected start-end, like 3-7");
        let (start, end) = raw.split_once('-').ok_or_else(not_a_span)?;
        Ok(Span {
            start: start.parse().map_err(|_| not_a_span())?,
            end: end.parse().map_err(|_| not_a_span())?,
        })
    }

    fn format(&self, span: &Span) -> String {
        format!("{}-{}", span.start, span.end)
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

fn span_field() -> NodeCtor {
    NodeCtor::new(|args| Box::new(args.node(SpanField)))
}

#[derive(Clone, Debug, PartialEq, Facet)]
struct Highlight {
    note: String,
    span: Span,
}

#[derive(Clone, Debug, PartialEq, Facet)]
struct MaybeHighlight {
    span: Option<Span>,
}

/// A plain newtype, which the default walk no longer unwraps (decision N of
/// `CUSTOM_FIELDS_PLAN.md`): without the constructor it would panic, asking for
/// a registered field.
#[derive(Clone, Debug, PartialEq, Facet)]
struct Minutes(u32);

/// Reads and writes `Minutes` as `15m`, so its `Value` is the newtype itself.
#[derive(Clone, Debug, PartialEq)]
struct MinutesField;

impl Field for MinutesField {
    type Value = Minutes;
    type Context = ();

    const KIND: FieldType = FieldType::Text;

    fn parse(&self, raw: &str) -> Result<Minutes, ValidationMessage> {
        raw.strip_suffix('m')
            .and_then(|n| n.parse().ok())
            .map(Minutes)
            .ok_or_else(|| ValidationMessage::from("expected minutes, like 15m"))
    }

    fn format(&self, minutes: &Minutes) -> String {
        format!("{}m", minutes.0)
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

#[derive(Clone, Debug, PartialEq, Facet)]
struct Break {
    length: Minutes,
}

/// Every path the form sends, with its raw text, in order.
fn values_of<T>(state: &FormState<T>) -> Vec<(String, String)>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    state
        .collect_values()
        .into_iter()
        .map(|(path, raw)| (path.to_string(), raw))
        .collect()
}

/// The model after submitting `values`, or `None` if it did not validate.
fn submit<T>(mut state: FormState<T>, values: &[(&str, &str)]) -> Option<T>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    let values: Vec<(NodePath, String)> = values
        .iter()
        .map(|(k, v)| (NodePath::new(k.to_string()), v.to_string()))
        .collect();
    state.distribute_form_values(&values);
    state.validate()
}

/// The control: with no constructor, a struct is a fieldset of its fields.
#[gtest]
fn without_a_constructor_a_struct_is_its_fields() {
    let highlight = Highlight {
        note: "x".into(),
        span: Span { start: 3, end: 7 },
    };
    let state = form_for(&highlight, FormSpec::default());
    expect_that!(
        values_of(&state),
        elements_are![
            eq(&("note".to_string(), "x".to_string())),
            eq(&("span.start".to_string(), "3".to_string())),
            eq(&("span.end".to_string(), "7".to_string())),
        ]
    );
}

/// On a struct-shaped path the constructor builds ONE leaf, filled with the
/// field's own `format`.
#[gtest]
fn a_constructor_on_a_struct_path_builds_one_leaf() {
    let highlight = Highlight {
        note: "x".into(),
        span: Span { start: 3, end: 7 },
    };
    let spec = FormSpec::default().with_node(&path!(Highlight.span), span_field());
    let state = form_for(&highlight, spec);
    expect_that!(
        values_of(&state),
        elements_are![
            eq(&("note".to_string(), "x".to_string())),
            eq(&("span".to_string(), "3-7".to_string())),
        ]
    );
}

/// And that one leaf reads the struct back through the field's own `parse`.
#[gtest]
fn a_struct_built_by_a_constructor_reads_back_through_its_parse() {
    let spec = FormSpec::default().with_node(&path!(Highlight.span), span_field());
    expect_that!(
        submit(
            empty_form::<Highlight>(spec),
            &[("note", "x"), ("span", "5-9")]
        ),
        some(eq(&Highlight {
            note: "x".into(),
            span: Span { start: 5, end: 9 },
        }))
    );
}

/// On an `Option<Span>` path the constructor sees what is inside the
/// `Option`: the walk peels it first, so the field's `Value` is `Span`.
#[gtest]
fn a_constructor_on_an_optional_path_sees_the_inner_value() {
    let spec = || FormSpec::default().with_node(&path!(MaybeHighlight.span?), span_field());
    let some_span = MaybeHighlight {
        span: Some(Span { start: 3, end: 7 }),
    };
    expect_that!(
        values_of(&form_for(&some_span, spec())),
        elements_are![eq(&("span".to_string(), "3-7".to_string()))]
    );
    expect_that!(
        submit(empty_form::<MaybeHighlight>(spec()), &[("span", "5-9")]),
        some(eq(&MaybeHighlight {
            span: Some(Span { start: 5, end: 9 }),
        }))
    );
}

/// An empty input on an optional path is `None`, the field never asked.
#[gtest]
fn an_empty_optional_path_with_a_constructor_is_none() {
    let spec = FormSpec::default().with_node(&path!(MaybeHighlight.span?), span_field());
    let none = MaybeHighlight { span: None };
    let state = form_for(&none, spec);
    expect_that!(
        values_of(&state),
        elements_are![eq(&("span".to_string(), String::new()))]
    );
    expect_that!(
        submit(state, &[("span", "")]),
        some(eq(&MaybeHighlight { span: None }))
    );
}

/// On a newtype path the constructor's field holds the newtype itself, not
/// the `u32` inside it, and writes it back without wrapping it a second time.
#[gtest]
fn a_constructor_on_a_newtype_path_sees_the_newtype() {
    let spec = || {
        FormSpec::default().with_node(&path!(Break.length), {
            NodeCtor::new(|args| Box::new(args.node(MinutesField)))
        })
    };
    let fifteen = Break {
        length: Minutes(15),
    };
    expect_that!(
        values_of(&form_for(&fifteen, spec())),
        elements_are![eq(&("length".to_string(), "15m".to_string()))]
    );
    expect_that!(
        submit(empty_form::<Break>(spec()), &[("length", "20m")]),
        some(eq(&Break {
            length: Minutes(20),
        }))
    );
}

/// A hand-built spec can name a field whose `Value` is not the path's type,
/// which `form!` would refuse at compile time. The walk refuses it as it builds
/// the leaf, naming the path and both types, even with no value to fill.
#[gtest]
#[should_panic(expected = "mismatched types at size, expected u8, given String")]
fn a_constructor_whose_value_is_not_the_paths_type_panics() {
    let spec = FormSpec::default().with_node(
        &path!(Order.size),
        NodeCtor::new(|args| Box::new(args.node(formoxus::fields::StringField::default()))),
    );
    let _ = empty_form::<Order>(spec);
}
