//! End-to-end round trips through `FormState<T>`: populate, collect, apply, validate.

use super::models::{Event, EventForCreate, Location};
use super::render_to_html;
use dioxus::core::Mutation;
use dioxus::prelude::*;
use dioxus_html::{
    PlatformEventData, SerializedHtmlEventConverter, SerializedMouseData, set_event_converter,
};
use facet::Facet;
use formoxus::fields::Constraints;
use formoxus::label_case::LabelCase;
use formoxus::*;
use googletest::prelude::*;
use std::any::Any;
use std::rc::Rc;
use std::{collections::HashMap, marker::PhantomData};

fn text_field(name: &str, value: FieldValue<String>) -> Box<dyn FormMember> {
    Box::new(FormField {
        name: name.to_string(),
        label: None,
        optional: false,
        constraints: Constraints::default(),
        custom_widget: None,
        choices: None,
        wrapper: None,
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
        text_field("zip", zip),
    ]
}

fn location_field_set(
    street: FieldValue<String>,
    city: FieldValue<String>,
    zip: FieldValue<String>,
) -> Box<dyn FormMember> {
    Box::new(FieldSet {
        name: "location".to_string(),
        optional: false,
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
    let form = empty_form::<Trip>(FormSpec::default());
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
    let form = use_form(|| empty_form::<EventForCreate>(FormSpec::default()));
    form.render_fragment()
}

#[gtest]
fn form_for_none_walks_the_shape_into_empty_members() {
    let form = empty_form::<EventForCreate>(FormSpec::default());

    expect_that!(
        member_names(&form.members),
        elements_are![eq("title"), eq("location")]
    );

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
    expect_that!(
        rendered,
        not(contains_substring(r#"value="Board Game Night""#))
    );
}

#[component]
fn TitledEventForm() -> Element {
    let form =
        use_form(|| empty_form::<EventForCreate>(FormSpec::default().with_title("New Event")));
    form.render_fragment()
}

#[gtest]
fn a_specs_title_reaches_the_rendered_form() {
    // Until `FormSpec` existed there was no way to set a title at all, so the
    // `h2.form-title` in `FormState::render` was unreachable — rendered by code
    // no caller could trigger. This is the first test that gets there.
    let rendered = render_to_html(TitledEventForm);
    expect_that!(rendered, contains_substring("New Event"));
    expect_that!(rendered, contains_substring(r#"class="form-title""#));
}

#[gtest]
fn a_form_with_no_title_renders_no_heading() {
    // The other half, and the one that would catch a stray default: an absent
    // title must produce no element at all, not an empty heading that still
    // takes vertical space and gets announced by a screen reader.
    let rendered = render_to_html(EmptyEventForm);
    expect_that!(rendered, not(contains_substring("form-title")));
}

#[gtest]
fn form_for_none_is_invalid_until_filled() {
    let mut form = empty_form::<EventForCreate>(FormSpec::default());
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

    let mut form = form_for(&event, FormSpec::default());
    expect_that!(form.has_errors(), eq(false));
    expect_that!(form.validate(), some(eq(&event)));
}

#[gtest]
fn option_fields_are_not_required() {
    let mut form = empty_form::<Rsvp>(FormSpec::default());

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
    expect_that!(
        form_for(&with_note, FormSpec::default()).validate(),
        some(eq(&with_note))
    );

    let without_note = Rsvp {
        name: "Ada".to_string(),
        guests: 2,
        note: None,
    };
    expect_that!(
        form_for(&without_note, FormSpec::default()).validate(),
        some(eq(&without_note))
    );
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
    let mut form = empty_form::<EventForCreate>(FormSpec::default());
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
    let mut form = empty_form::<Rsvp>(FormSpec::default());
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
    let mut form = empty_form::<Rsvp>(FormSpec::default());
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
    let mut form = form_for(
        &Rsvp {
            name: "Ada".to_string(),
            guests: 2,
            note: Some("bringing dessert".to_string()),
        },
        FormSpec::default(),
    );
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

    let form = form_for(&rsvp, FormSpec::default());
    let round_tripped: HashMap<String, String> = form.leaves().into_iter().collect();

    let mut reloaded = empty_form::<Rsvp>(FormSpec::default());
    reloaded.apply(&round_tripped);

    expect_that!(reloaded.validate(), some(eq(&rsvp)));
}

#[gtest]
fn empty_event_form_is_invalid() {
    let mut form: FormState<Event> = FormState {
        spec: FormSpec::new().with_title("New Event"),
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
        spec: FormSpec::default(),
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
        spec: FormSpec::new().with_title("New Event"),
        members: vec![
            text_field("title", FieldValue::Valid("Board Game Night".to_string())),
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

    let form = empty_form::<Settings>(FormSpec::default());
    let labels: Vec<Option<String>> = form
        .members
        .iter()
        .map(|m| m.label(LabelCase::Title))
        .collect();
    expect_that!(
        labels,
        elements_are![some(eq("Can Shuffle")), some(eq("Name"))]
    );
}

#[gtest]
fn a_list_row_gets_no_label() {
    // A row's name is its index, and "0" is a position, not a label. The list
    // carries the prose; the rows are positional.
    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Quiz {
        answer_choices: Vec<String>,
    }

    let form = form_for(
        &Quiz {
            answer_choices: vec!["PNG".to_string(), "JPEG".to_string()],
        },
        FormSpec::default(),
    );
    let list = &form.members[0];
    expect_that!(list.label(LabelCase::Title), some(eq("Answer Choices")));

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
    let form = use_form(|| {
        form_for(
            &Quiz {
                answer_choices: vec!["PNG".to_string(), "JPEG".to_string()],
            },
            FormSpec::default(),
        )
    });
    form.render_fragment()
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
    let _ = empty_form::<Unsupported>(FormSpec::default());
}

// ══ Form-level errors ════════════════════════════════════════════════════
//
// An error that belongs to the form rather than to a field: "invalid
// credentials", "that name is taken" — answers only the server has, which no
// field validator can produce because nothing local is wrong.

#[derive(Facet, Clone, Debug, PartialEq)]
struct Credentials {
    username: String,
}

/// Click the app's first `click` listener and return the re-rendered HTML.
///
/// The sibling of `type_into` in the `widgets` module: the listener's `ElementId`
/// comes out of the rebuild's mutations rather than being hard-coded, so the
/// markup around the button can change freely.
fn click_button(app: fn() -> Element) -> String {
    set_event_converter(Box::new(SerializedHtmlEventConverter));

    let mut dom = VirtualDom::new(app);
    let mutations = dom.rebuild_to_vec();
    let button_id = mutations
        .edits
        .iter()
        .find_map(|m| match m {
            Mutation::NewEventListener { name, id } if name == "click" => Some(*id),
            _ => None,
        })
        .expect("the test component should have registered an `onclick` listener");

    let payload = PlatformEventData::new(Box::new(SerializedMouseData::default()));
    // Fully qualified: this module imports `models::Event`, which shadows the
    // Dioxus one.
    let event: dioxus::prelude::Event<dyn Any> =
        dioxus::prelude::Event::new(Rc::new(payload), true);
    dom.runtime().handle_event("click", event, button_id);

    dom.render_immediate_to_vec();
    dioxus_ssr::render(&dom)
}

#[component]
fn PushesAnErrorOnClick() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Credentials>::default()));
    rsx! {
        { form.render_fragment() }
        button {
            onclick: move |_| form.push_error(FormError("invalid credentials".to_string())),
            "Sign in"
        }
    }
}

#[gtest]
fn a_pushed_error_renders() {
    // Through a real event handler, because that is the only way a page can push
    // one — and because it is what proves `push_error` can take `&self` on a
    // `Copy` handle inside a closure.
    let html = click_button(PushesAnErrorOnClick);
    expect_that!(html, contains_substring("invalid credentials"));
    expect_that!(html, contains_substring("form-error"));
}

#[gtest]
fn a_form_starts_with_no_error_markup() {
    // The widget for the test above: without it, `contains_substring` proves
    // only that the string appears somewhere, not that the click put it there.
    #[component]
    fn Untouched() -> Element {
        use_form(|| empty_form(FormSpec::<Credentials>::default())).render_fragment()
    }
    expect_that!(
        render_to_html(Untouched),
        not(contains_substring("form-error"))
    );
}

// ── The split render methods ─────────────────────────────────────────────
//
// `render_fragment` is title -> fields -> errors, all three. These pin that
// each of the three pieces `Form` now exposes on its own — `render_fields`,
// `render_title`, `render_errors` — renders exactly its own slice and nothing
// from the other two, so a page can recombine them (e.g. put the buttons
// between fields and errors) without any slice smuggling in markup that
// belongs to another.

#[component]
fn TitledFieldsOnly() -> Element {
    let form =
        use_form(|| empty_form::<EventForCreate>(FormSpec::default().with_title("New Event")));
    form.render_fields()
}

#[gtest]
fn render_fields_omits_the_title() {
    let rendered = render_to_html(TitledFieldsOnly);
    expect_that!(rendered, not(contains_substring("form-title")));
    expect_that!(rendered, not(contains_substring("New Event")));
    // The fields themselves are still there.
    expect_that!(rendered, contains_substring(r#"name="title""#));
    expect_that!(rendered, contains_substring(r#"name="location.street""#));
}

#[component]
fn FieldsOnlyWithPushedError() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Credentials>::default()));
    rsx! {
        { form.render_fields() }
        button {
            onclick: move |_| form.push_error(FormError("invalid credentials".to_string())),
            "Sign in"
        }
    }
}

#[gtest]
fn render_fields_omits_a_pushed_error() {
    let html = click_button(FieldsOnlyWithPushedError);
    expect_that!(html, not(contains_substring("invalid credentials")));
    expect_that!(html, not(contains_substring("form-error")));
    expect_that!(html, contains_substring(r#"name="username""#));
}

#[component]
fn TitleOnly() -> Element {
    let form =
        use_form(|| empty_form::<EventForCreate>(FormSpec::default().with_title("New Event")));
    form.render_title()
}

#[gtest]
fn render_title_renders_just_the_heading() {
    let rendered = render_to_html(TitleOnly);
    expect_that!(rendered, contains_substring("New Event"));
    expect_that!(rendered, contains_substring(r#"class="form-title""#));
    // None of the fields came along for the ride.
    expect_that!(rendered, not(contains_substring("name=")));
}

#[component]
fn TitleOnlyAbsent() -> Element {
    let form = use_form(|| empty_form::<EventForCreate>(FormSpec::default()));
    form.render_title()
}

#[gtest]
fn render_title_is_empty_when_there_is_none() {
    // The widget for the test above: a titleless form's `render_title` slice
    // is nothing at all, not an empty heading that still occupies a DOM node.
    let rendered = render_to_html(TitleOnlyAbsent);
    expect_that!(rendered.trim(), eq(""));
}

#[component]
fn ErrorsOnlyWithPushedError() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Credentials>::default()));
    rsx! {
        { form.render_errors() }
        button {
            onclick: move |_| form.push_error(FormError("invalid credentials".to_string())),
            "Sign in"
        }
    }
}

#[gtest]
fn render_errors_shows_a_pushed_error() {
    let html = click_button(ErrorsOnlyWithPushedError);
    expect_that!(html, contains_substring("invalid credentials"));
    expect_that!(html, contains_substring("form-error"));
    // No field markup leaked in from the same form.
    expect_that!(html, not(contains_substring(r#"name="username""#)));
}

#[gtest]
fn render_errors_is_empty_before_any_push() {
    #[component]
    fn Untouched() -> Element {
        use_form(|| empty_form(FormSpec::<Credentials>::default())).render_errors()
    }
    let rendered = render_to_html(Untouched);
    expect_that!(rendered.trim(), eq(""));
}

// ── `render` owns the `<div class="form">` / `<form>` wrapper ───────────────
//
// `render_fragment` never gained a wrapper — that's the escape hatch, and a
// regression here would silently start wrapping every one of the 46 existing
// call sites that rely on it staying bare. `render` is the new, opt-in
// method that owns the div/form/title placement.

#[component]
fn WrappedTitledEventForm() -> Element {
    let form =
        use_form(|| empty_form::<EventForCreate>(FormSpec::default().with_title("New Event")));
    form.render(Fns::new())
}

#[gtest]
fn render_wraps_the_form_in_a_div_with_class_form() {
    let rendered = render_to_html(WrappedTitledEventForm);
    expect_that!(rendered, contains_substring(r#"class="form""#));
    expect_that!(rendered, contains_substring("<form"));
}

#[gtest]
fn render_places_the_title_before_the_form_element_not_inside_it() {
    let rendered = render_to_html(WrappedTitledEventForm);
    let title_pos = rendered
        .find("form-title")
        .expect("the title should render");
    let form_open_pos = rendered
        .find("<form")
        .expect("a form element should render");
    expect_that!(title_pos, lt(form_open_pos));
}

#[gtest]
fn render_fragment_stays_unwrapped() {
    // The widget: `render_fragment` must NOT pick up `render`'s div/form
    // wrapper, since every existing call site depends on getting back exactly
    // today's bare fragment. `TitledEventForm` (defined above) renders via
    // `render_fragment`, with the same model and title as `WrappedTitledEventForm`.
    let rendered = render_to_html(TitledEventForm);
    expect_that!(rendered, not(contains_substring(r#"class="form""#)));
    expect_that!(rendered, not(contains_substring("<form")));
}

#[gtest]
fn validate_clears_a_pushed_error() {
    // The intended lifetime, and the reason `push_error` needs no `clear`: an
    // error from the last round trip must not outlive the next submit. It also
    // means a pushed error never BLOCKS validate, since the clear happens first —
    // which this proves by getting a model back out.
    let mut state = empty_form(FormSpec::<Credentials>::default());
    state
        .errors
        .push(FormError("invalid credentials".to_string()));
    expect_that!(state.has_errors(), eq(true));

    state.apply(&HashMap::from([(
        "username".to_string(),
        "bob".to_string(),
    )]));
    let model = state.validate();

    expect_that!(
        model,
        some(eq(&Credentials {
            username: "bob".to_string()
        }))
    );
    expect_that!(state.has_errors(), eq(false));
}
