//! The widget boundary — signal-minted and fully uncontrolled.

// `dioxus::prelude` exports its own `Location`, so ours needs an explicit
// name to win the glob-import ambiguity.
use super::models::{EventForCreate, Location as ModelLocation};
use super::render_to_html;
use crate::reflect::*;
use dioxus::core::Mutation;
use dioxus_html::{
    PlatformEventData, SerializedFormData, SerializedHtmlEventConverter, set_event_converter,
};
use std::any::Any;
use std::rc::Rc;
use dioxus::prelude::*;
use facet::Facet;
use std::{collections::HashMap, fmt::Debug};
use googletest::prelude::*;

/// One signal per leaf input, keyed by qualified path.
///
/// The rules-of-hooks question this answers: the field count isn't known
/// until runtime, so we can't call `use_signal` in a loop. But `use_hook`
/// runs its initializer exactly once, inside a live runtime — so a single
/// hook call can mint N signals, and the *hook count* stays 1 no matter
/// how many fields the shape turned out to have.
fn use_field_signals<T: Clone + Debug + PartialEq + Facet<'static>>(
    form: &Form<T>,
) -> HashMap<String, Signal<String>> {
    let leaves = form.leaves();
    use_hook(|| {
        leaves
            .into_iter()
            .map(|(path, raw)| (path, Signal::new(raw)))
            .collect()
    })
}

#[component]
fn EventFormView() -> Element {
    let form = use_hook(|| {
        form_for(&EventForCreate {
            title: "Board Game Night".to_string(),
            location: ModelLocation {
                street: "123 Main St".to_string(),
                city: "Springfield".to_string(),
                zip: "12345".to_string(),
            },
        })
    });
    let signals = use_field_signals(&form);

    // Stable order so the rendered output is deterministic.
    let mut paths: Vec<String> = signals.keys().cloned().collect();
    paths.sort();

    rsx! {
        form {
            for path in paths {
                input {
                    r#type: "text",
                    name: "{path}",
                    value: "{signals[&path]}",
                }
            }
        }
    }
}

/// No signals at all: every input is uncontrolled, named by its qualified
/// path, and the browser holds the editing state. On submit, `values()`
/// hands the whole form back and we shuffle it into a `Form<T>` once.
#[component]
fn UncontrolledEventForm() -> Element {
    let form = use_hook(empty_form::<EventForCreate>);
    let leaves = form.leaves();

    rsx! {
        form {
            onsubmit: move |e: FormEvent| {
                let values: Vec<(String, String)> = e
                    .values()
                    .into_iter()
                    .filter_map(|(name, v)| match v {
                        FormValue::Text(text) => Some((name, text)),
                        // File inputs are a separate story — there's no
                        // text for a scalar widget to parse.
                        FormValue::File(_) => None,
                    })
                    .collect();
                let mut form = empty_form::<EventForCreate>();
                form.apply_form_values(&values);
                let _model = form.validate();
            },
            for (path, raw) in leaves {
                input { r#type: "text", name: "{path}", value: "{raw}" }
            }
            button { r#type: "submit", "Save" }
        }
    }
}

#[gtest]
fn uncontrolled_inputs_are_named_by_qualified_path() {
    // `FormData::values()` keys off the `name` attribute, so these names
    // are the entire contract between the DOM and `apply_form_values`.
    let html = render_to_html(UncontrolledEventForm);
    for path in ["title", "location.street", "location.city", "location.zip"] {
        expect_that!(html, contains_substring(format!(r#"name="{path}""#)));
    }
}

#[gtest]
fn submitted_values_shuffle_into_a_model() {
    // Exactly the shape `FormData::values()` produces, minus the DOM.
    let submitted = vec![
        ("title".to_string(), "Board Game Night".to_string()),
        ("location.street".to_string(), "123 Main St".to_string()),
        ("location.city".to_string(), "Springfield".to_string()),
        ("location.zip".to_string(), "12345".to_string()),
    ];

    let mut form = empty_form::<EventForCreate>();
    form.apply_form_values(&submitted);

    expect_that!(
        form.validate(),
        some(eq(&EventForCreate {
            title: "Board Game Night".to_string(),
            location: ModelLocation {
                street: "123 Main St".to_string(),
                city: "Springfield".to_string(),
                zip: "12345".to_string(),
            },
        }))
    );
}

#[gtest]
fn component_mints_one_signal_per_leaf() {
    let html = render_to_html(EventFormView);

    // Nested paths are qualified, so `location.street` can't collide with
    // a top-level `street` in some other field set.
    for path in ["title", "location.street", "location.city", "location.zip"] {
        expect_that!(html, contains_substring(format!(r#"name="{path}""#)));
    }
}

#[gtest]
fn signals_are_populated_from_the_model() {
    let html = render_to_html(EventFormView);
    expect_that!(html, contains_substring("Board Game Night"));
    expect_that!(html, contains_substring("123 Main St"));
}

// ── The store-bound widget boundary ──────────────────────────────────────
//
// These drive `ScalarInput` directly rather than through `FormMember::render`,
// so they stay meaningful regardless of how the members wire it up.

use crate::reflect::widgets::ScalarInput;

#[component]
fn PopulatedInput() -> Element {
    let values = use_store(|| {
        HashMap::from([("title".to_string(), "Board Game Night".to_string())])
    });
    rsx! {
        ScalarInput {
            path: "title".to_string(),
            label: Some("Title".to_string()),
            errors: Vec::new(),
            values,
        }
    }
}

#[component]
fn EmptyInput() -> Element {
    // A path the schema has but the value map has never seen — what a variant
    // chosen after mount produces.
    let values = use_store(HashMap::<String, String>::new);
    rsx! {
        ScalarInput {
            path: "shape.radius".to_string(),
            label: None,
            errors: Vec::new(),
            values,
        }
    }
}

#[gtest]
fn a_scalar_input_binds_to_its_path_in_the_value_map() {
    let html = render_to_html(PopulatedInput);
    expect_that!(html, contains_substring(r#"name="title""#));
    expect_that!(html, contains_substring("Board Game Night"));
    expect_that!(html, contains_substring("Title"));
}

#[gtest]
fn a_path_the_value_map_never_saw_renders_empty_rather_than_panicking() {
    // `get_unchecked` + `try_read` is what buys this: reading a missing key
    // yields `""` instead of the panic a plain `read()` would raise. It's the
    // same "absent IS empty" rule `apply_leaves` follows for submitted values.
    let html = render_to_html(EmptyInput);
    expect_that!(html, contains_substring(r#"name="shape.radius""#));
    expect_that!(html, contains_substring(r#"value="""#));
}

// ── Typing: the write half of the store binding ──────────────────────────
//
// Everything about the fine-grained write (`set` through the child store when
// the path is populated, `insert` only when it isn't) was argument until here.
// These drive a real `oninput` through a real `VirtualDom`.

/// Type `text` into the app's `oninput` listener and return the re-rendered HTML.
///
/// The listener's `ElementId` is read out of the rebuild's mutations rather than
/// hard-coded — `NewEventListener` carries the id it attached to, so this keeps
/// working if the markup around the input changes.
fn type_into(app: fn() -> Element, text: &str) -> String {
    // A platform (web, desktop) normally installs this; a bare `VirtualDom`
    // has none, and without it the listener's `PlatformEventData -> FormData`
    // conversion has nothing to convert with. Idempotent, so each test can call it.
    set_event_converter(Box::new(SerializedHtmlEventConverter));

    let mut dom = VirtualDom::new(app);
    let mutations = dom.rebuild_to_vec();
    let input_id = mutations
        .edits
        .iter()
        .find_map(|m| match m {
            Mutation::NewEventListener { name, id } if name == "input" => Some(*id),
            _ => None,
        })
        .expect("ScalarInput should have registered an `oninput` listener");

    // Listeners are registered against `PlatformEventData`, not `FormData` — the
    // `oninput` attribute macro does that conversion itself, inside the handler.
    // Handing it a `FormData` directly fails the downcast at dispatch.
    let payload = PlatformEventData::new(Box::new(SerializedFormData::new(
        text.to_string(),
        Vec::new(),
    )));
    let event: Event<dyn Any> = Event::new(Rc::new(payload), true);
    dom.runtime().handle_event("input", event, input_id);

    dom.render_immediate_to_vec();
    dioxus_ssr::render(&dom)
}

#[gtest]
fn typing_writes_through_to_the_value_map() {
    // The populated path: the key already exists, so the handler writes through
    // the child store rather than re-inserting.
    let html = type_into(PopulatedInput, "Trivia Night");
    expect_that!(html, contains_substring(r#"value="Trivia Night""#));
    expect_that!(html, not(contains_substring("Board Game Night")));
}

#[gtest]
fn typing_into_a_never_populated_path_inserts_it() {
    // The `insert` fallback — a leaf revealed after mount, whose path the value
    // map has never held. Without it the keystroke would be silently dropped.
    let html = type_into(EmptyInput, "3.5");
    expect_that!(html, contains_substring(r#"value="3.5""#));
}
