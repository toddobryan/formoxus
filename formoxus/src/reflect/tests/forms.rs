//! End-to-end round trips through `FormState<T>`: populate, collect, apply, validate.

use super::render_to_html;
use crate::reflect::{widgets::InputKind, *};
use dioxus::prelude::*;
use facet::Facet;
use std::{collections::HashMap, marker::PhantomData};
use super::models::{Event, EventForCreate, Location};
use googletest::prelude::*;

fn text_field(name: &str, value: FieldValue<String>) -> Box<dyn FormMember> {
    Box::new(FormField {
        name: name.to_string(),
        label: None,
        input_kind: InputKind::Text,
        value,
        errors: Vec::new(),
    })
}

fn location_members(
    street: FieldValue<String>,
    city: FieldValue<String>,
    zip: FieldValue<String>,
) -> Vec<Box<dyn FormMember>> {
    vec![
        text_field("street", street),
        text_field("city", city),
        text_field("zip",  zip),
    ]
}

fn location_field_set(
    street: FieldValue<String>,
    city: FieldValue<String>,
    zip: FieldValue<String>,
) -> Box<dyn FormMember> {
    Box::new(FieldSet {
        name: "location".to_string(),
        label: Some("Location".to_string()),
        members: location_members(street, city, zip),
        errors: Vec::new(),
    })
}

/// Has an `Option` field, which none of the other models do — that's the
/// path `form_for` uses to decide `required`.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Rsvp {
    name: String,
    guests: u32,
    note: Option<String>,
}

/// The same struct type twice in one form — the case that decides whether
/// leaf paths can be keyed by struct name.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    origin: Location,
    destination: Location,
}

fn member_names(members: &[Box<dyn FormMember>]) -> Vec<String> {
    members.iter().map(|m| m.name()).collect()
}

#[gtest]
fn repeated_struct_types_get_distinct_paths() {
    let form = empty_form::<Trip>();
    let paths: Vec<String> = form.leaves().into_iter().map(|(p, _)| p).collect();

    expect_that!(
        paths,
        elements_are![
            eq("origin.street"),
            eq("origin.city"),
            eq("origin.zip"),
            eq("destination.street"),
            eq("destination.city"),
            eq("destination.zip"),
        ]
    );
}

#[component]
fn EmptyEventForm() -> Element {
    let form = use_form(empty_form::<EventForCreate>());
    form.render()
}

#[gtest]
fn form_for_none_walks_the_shape_into_empty_members() {
    let form = empty_form::<EventForCreate>();

    expect_that!(member_names(&form.members), elements_are![eq("title"), eq("location")]);

    // The nested struct field became a FieldSet with its own members,
    // discovered purely from `Location`'s shape. The inner names are now
    // QUALIFIED (`location.street`, not `street`) — that changed when `render`
    // started threading a prefix, and it's what stops two field sets in one
    // form from both claiming `street`.
    let rendered = render_to_html(EmptyEventForm);
    for name in ["title", "location.street", "location.city", "location.zip"] {
        expect_that!(rendered, contains_substring(format!(r#"name="{name}""#)));
    }
    // Nothing was populated, so every input is blank.
    expect_that!(rendered, not(contains_substring(r#"value="Board Game Night""#)));
}

#[gtest]
fn form_for_none_is_invalid_until_filled() {
    let mut form = empty_form::<EventForCreate>();
    expect_that!(form.validate(), none());
    expect_that!(form.has_errors(), eq(true));
}

#[gtest]
fn form_for_some_round_trips_the_model() {
    let event = EventForCreate {
        title: "Board Game Night".to_string(),
        location: Location {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            zip: "12345".to_string(),
        },
    };

    let mut form = form_for(&event);
    expect_that!(form.has_errors(), eq(false));
    expect_that!(form.validate(), some(eq(&event)));
}

#[gtest]
fn option_fields_are_not_required() {
    let mut form = empty_form::<Rsvp>();

    // `note: Option<String>` is optional, so an empty form only complains
    // about `name` and `guests`.
    for m in form.members.iter_mut() {
        m.validate();
    }
    let complaining: Vec<String> = form
        .members
        .iter()
        .filter(|m| m.has_errors())
        .map(|m| m.name())
        .collect();
    expect_that!(complaining, elements_are![eq("name"), eq("guests")]);
}

#[gtest]
fn option_fields_round_trip_both_ways() {
    let with_note = Rsvp {
        name: "Ada".to_string(),
        guests: 2,
        note: Some("bringing dessert".to_string()),
    };
    expect_that!(form_for(&with_note).validate(), some(eq(&with_note)));

    let without_note = Rsvp {
        name: "Ada".to_string(),
        guests: 2,
        note: None,
    };
    expect_that!(form_for(&without_note).validate(), some(eq(&without_note)));
}

fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[gtest]
fn applying_widget_values_round_trips_to_a_model() {
    // The full loop: shape-walk an empty form, take raw strings back in
    // the way a submit handler would, then validate into a model.
    let mut form = empty_form::<EventForCreate>();
    form.apply(&values(&[
        ("title", "Board Game Night"),
        ("location.street", "123 Main St"),
        ("location.city", "Springfield"),
        ("location.zip", "12345"),
    ]));

    expect_that!(
        form.validate(),
        some(eq(&EventForCreate {
            title: "Board Game Night".to_string(),
            location: Location {
                street: "123 Main St".to_string(),
                city: "Springfield".to_string(),
                zip: "12345".to_string(),
            },
        }))
    );
}

#[gtest]
fn non_string_scalars_parse_through_the_shape_vtable() {
    // `u32` here never touches `FromStr` — facet parses it from the shape.
    let mut form = empty_form::<Rsvp>();
    form.apply(&values(&[
        ("name", "Ada"),
        ("guests", "2"),
        ("note", "bringing dessert"),
    ]));

    expect_that!(
        form.validate(),
        some(eq(&Rsvp {
            name: "Ada".to_string(),
            guests: 2,
            note: Some("bringing dessert".to_string()),
        }))
    );
}

#[gtest]
fn unparseable_input_becomes_invalid_not_a_panic() {
    let mut form = empty_form::<Rsvp>();
    form.apply(&values(&[("name", "Ada"), ("guests", "not a number")]));

    expect_that!(form.validate(), none());
    expect_that!(form.has_errors(), eq(true));

    // The bad input is preserved so the widget can show it back.
    let guests = form
        .leaves()
        .into_iter()
        .find(|(p, _)| p == "guests")
        .map(|(_, raw)| raw);
    expect_that!(guests, some(eq(&"not a number".to_string())));
}

#[gtest]
fn blanking_a_field_makes_it_empty_again() {
    let mut form = form_for(&Rsvp {
        name: "Ada".to_string(),
        guests: 2,
        note: Some("bringing dessert".to_string()),
    });
    // Clearing an optional field is legal; clearing a required one isn't.
    form.apply(&values(&[("note", ""), ("name", "")]));

    expect_that!(form.validate(), none());
    let complaining: Vec<String> = form
        .members
        .iter()
        .filter(|m| m.has_errors())
        .map(|m| m.name())
        .collect();
    expect_that!(complaining, elements_are![eq("name")]);
}

#[gtest]
fn leaves_then_apply_is_an_identity_round_trip() {
    // The actual widget loop: populate a form from a model, hand the raw
    // strings to the widget layer, take them straight back, and validate.
    // Nothing edited in between, so this must land on the same model.
    let rsvp = Rsvp {
        name: "Ada".to_string(),
        guests: 2,
        note: Some("bringing dessert".to_string()),
    };

    let form = form_for(&rsvp);
    let round_tripped: HashMap<String, String> = form.leaves().into_iter().collect();

    let mut reloaded = empty_form::<Rsvp>();
    reloaded.apply(&round_tripped);

    expect_that!(reloaded.validate(), some(eq(&rsvp)));
}

#[gtest]
fn empty_event_form_is_invalid() {
    let mut form: FormState<Event> = FormState {
        title: Some("New Event".to_string()),
        members: vec![
            text_field("title", FieldValue::Empty),
            location_field_set(FieldValue::Empty, FieldValue::Empty, FieldValue::Empty),
        ],
        errors: Vec::new(),
        _type: PhantomData,
    };

    expect_that!(form.validate(), none());
    expect_that!(form.has_errors(), eq(true));
}

#[gtest]
fn location_form_round_trips_to_model() {
    // `Location` has no uncollected fields, so this exercises the core
    // `FormField::write_into` -> `Partial::build` -> `materialize` path
    // with nothing else in the way.
    let mut form: FormState<Location> = FormState {
        title: None,
        members: location_members(
            FieldValue::Valid("123 Main St".to_string()),
            FieldValue::Valid("Springfield".to_string()),
            FieldValue::Valid("12345".to_string()),
        ),
        errors: Vec::new(),
        _type: PhantomData,
    };

    let model = form.validate().expect("all required fields are filled");
    expect_that!(
        model,
        eq(&Location {
            street: "123 Main St".to_string(),
            city: "Springfield".to_string(),
            zip: "12345".to_string(),
        })
    );
}

#[gtest]
fn event_for_create_form_round_trips_to_model() {
    let mut form: FormState<EventForCreate> = FormState {
        title: Some("New Event".to_string()),
        members: vec![
            text_field(
                "title",
                FieldValue::Valid("Board Game Night".to_string()),
            ),
            location_field_set(
                FieldValue::Valid("123 Main St".to_string()),
                FieldValue::Valid("Springfield".to_string()),
                FieldValue::Valid("12345".to_string()),
            ),
        ],
        errors: Vec::new(),
        _type: PhantomData,
    };

    expect_that!(form.has_errors(), eq(false));
    let model = form.validate().expect("all required fields are filled");
    expect_that!(model.title, eq(&"Board Game Night"));
    expect_that!(model.location.street, eq(&"123 Main St"));
}

// ── Labels ───────────────────────────────────────────────────────────────

#[gtest]
fn a_label_defaults_to_the_humanized_field_name() {
    // Nothing sets labels today — the shape carries a field's name but no prose
    // for it — so without this every input renders bare.
    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Settings {
        can_shuffle: bool,
        name: String,
    }

    let form = empty_form::<Settings>();
    let labels: Vec<Option<String>> = form.members.iter().map(|m| m.label()).collect();
    expect_that!(labels, elements_are![some(eq("Can Shuffle")), some(eq("Name"))]);
}

#[gtest]
fn a_list_row_gets_no_label() {
    // A row's name is its index, and "0" is a position, not a label. The list
    // carries the prose; the rows are positional.
    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Quiz {
        answer_choices: Vec<String>,
    }

    let form = form_for(&Quiz {
        answer_choices: vec!["PNG".to_string(), "JPEG".to_string()],
    });
    let list = &form.members[0];
    expect_that!(list.label(), some(eq("Answer Choices")));

    // Rendered, the rows must not pick up "0"/"1" as labels. Note the list's own
    // label doesn't appear either — no container renders one yet, which is the
    // other half of why a form still reads bare.
    let html = super::render_to_html(QuizForm);
    expect_that!(html, not(contains_substring(">0<")));
    expect_that!(html, not(contains_substring(">1<")));
}

#[component]
fn QuizForm() -> Element {
    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Quiz {
        answer_choices: Vec<String>,
    }
    let form = use_form(form_for(&Quiz {
        answer_choices: vec!["PNG".to_string(), "JPEG".to_string()],
    }));
    form.render()
}

/// A model using a scalar with no built-in widget. `usize` is the realistic
/// case — it is excluded deliberately, for being a target-dependent width.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Unsupported {
    count: usize,
}

#[gtest]
#[should_panic(expected = "scalar type USize is not supported in FormField")]
fn an_unsupported_scalar_fails_loudly_at_construction() {
    // Loud rather than silent: the alternative to panicking is rendering the
    // field as something it isn't, and a form that quietly mis-handles a value
    // is worse than one that refuses to build. Construction is also the right
    // moment — it happens once, in a hook initialiser, not per render.
    let _ = empty_form::<Unsupported>();
}
