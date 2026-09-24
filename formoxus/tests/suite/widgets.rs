//! The widget boundary — signal-minted and fully uncontrolled.

// `dioxus::prelude` exports its own `Location`, so ours needs an explicit
// name to win the glob-import ambiguity.
use super::models::{EventForCreate, Location as ModelLocation};
use super::render_to_html;
use dioxus::core::Mutation;
use dioxus::prelude::*;
use dioxus_html::{
    PlatformEventData, SerializedFormData, SerializedHtmlEventConverter, set_event_converter,
};
use facet::Facet;
use formoxus::*;
use googletest::prelude::*;
use std::any::Any;
use std::rc::Rc;
use std::{collections::HashMap, fmt::Debug};

/// One signal per leaf input, keyed by qualified path.
///
/// The rules-of-hooks question this answers: the field count isn't known
/// until runtime, so we can't call `use_signal` in a loop. But `use_hook`
/// runs its initializer exactly once, inside a live runtime — so a single
/// hook call can mint N signals, and the *hook count* stays 1 no matter
/// how many fields the shape turned out to have.
fn use_field_signals<T: Clone + Debug + PartialEq + Facet<'static>>(
    form: &FormState<T>,
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
        form_for(
            &EventForCreate {
                title: "Board Game Night".to_string(),
                location: ModelLocation {
                    street: "123 Main St".to_string(),
                    city: "Springfield".to_string(),
                    zip: "12345".to_string(),
                },
            },
            FormSpec::default(),
        )
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
/// hands the whole form back and we shuffle it into a `FormState<T>` once.
#[component]
fn UncontrolledEventForm() -> Element {
    let form = use_hook(|| empty_form::<EventForCreate>(FormSpec::default()));
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
                let mut form = empty_form::<EventForCreate>(FormSpec::default());
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

    let mut form = empty_form::<EventForCreate>(FormSpec::default());
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
// These drive `ScalarWidget` directly rather than through `FormMember::render`,
// so they stay meaningful regardless of how the members wire it up.

use formoxus::fields::ValueKind;
use formoxus::widgets::{FieldProps, InputType, ScalarWidget, WidgetType};

/// What a `String` field derives. Spelled out because these tests drive
/// `ScalarWidget` directly rather than through `FormField::render`, so nothing
/// upstream is computing it for them — which is the point: they pin the widget
/// boundary independently of how the members happen to wire it up.
fn text_kind() -> ValueKind {
    ValueKind::Text {
        min_length: None,
        max_length: None,
        pattern: None,
    }
}

#[component]
fn PopulatedInput() -> Element {
    let values =
        use_store(|| HashMap::from([("title".to_string(), "Board Game Night".to_string())]));
    rsx! {
        ScalarWidget {
            value_kind: text_kind(),
            widget: WidgetType::Input(InputType::Text),
            values,
            props: FieldProps {
                path: "title".to_string(),
                label: Some("Title".to_string()),
                required: true,
                errors: Vec::new(),
            },
        }
    }
}

#[component]
fn EmptyInput() -> Element {
    // A path the schema has but the value map has never seen — what a variant
    // chosen after mount produces.
    let values = use_store(HashMap::<String, String>::new);
    rsx! {
        ScalarWidget {
            value_kind: text_kind(),
            widget: WidgetType::Input(InputType::Text),
            values,
            props: FieldProps {
                path: "shape.radius".to_string(),
                label: None,
                required: true,
                errors: Vec::new(),
            },
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
        .expect("ScalarWidget should have registered an `oninput` listener");

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

// ── An unparseable value has to SAY so ───────────────────────────────────

/// One `f64`, so a single unparseable entry is the whole story. `Grading::
/// AllowTwoChances { second_try_credit }` on `/form-demo` is where this was
/// spotted: typing "abc" refused the form with nothing shown under the field.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Score {
    credit: f64,
}

#[component]
fn ScoreFormWithBadInput() -> Element {
    let mut state = empty_form::<Score>(FormSpec::default());
    state.apply_form_values(&[("credit".to_string(), "abc".to_string())]);
    // Validating BEFORE mounting is what makes this a pure render assertion —
    // errors are populated by `validate`, and blur-time validation doesn't
    // exist yet, so there is no interaction to drive here.
    let _ = state.validate();
    let form = use_form(|| state);
    form.render_fragment()
}

#[gtest]
fn an_unparseable_value_renders_its_error() {
    // The regression that motivated this: `FieldValue::Invalid` carries the parse
    // error INSIDE the value, `has_errors` reads it there, and `validate` used to
    // leave `self.errors` empty. So the form correctly refused to submit using
    // evidence the widget could not see, and the field rendered clean.
    //
    // Asserted on the wrapper class rather than the message text, because the
    // wording is deliberately still the raw shape name and is expected to change.
    let html = render_to_html(ScoreFormWithBadInput);
    expect_that!(
        html,
        contains_substring("field-error"),
        "an invalid field must render an error, not just fail the form:\n{html}"
    );
    // What the user typed has to survive, or the error names a value that is no
    // longer on screen.
    expect_that!(html, contains_substring(r#"value="abc""#));
}

#[gtest]
fn an_unparseable_value_does_not_also_claim_to_be_required() {
    // `Empty` and `Invalid` are different failures and only one message belongs
    // on the field. The tempting shortcut is to leave `errors` empty when the
    // value is unparseable — which satisfies "not also required" by making the
    // field report nothing at all, so the real error never renders either.
    let mut form = empty_form::<Score>(FormSpec::default());
    form.apply_form_values(&[("credit".to_string(), "abc".to_string())]);
    expect_that!(form.validate(), none());

    let html = render_to_html(ScoreFormWithBadInput);
    expect_that!(html, not(contains_substring("This field is required.")));
    expect_that!(html.matches("field-error\"").count(), eq(1));
}

// ── The error state is markup, not a class we invented ───────────────────

#[gtest]
fn an_errored_field_marks_its_widget_aria_invalid() {
    // The widget itself carries the error state, not just the message beside
    // it — this is the half a screen reader announces. Rendering the message
    // alone would serve the sighted case and leave the other unserved.
    let html = render_to_html(ScoreFormWithBadInput);
    expect_that!(html, contains_substring(r#"aria-invalid="true""#));
    // Immediately after the input, with no wrapper in between, so a plain
    // `input[aria-invalid="true"] + *` sibling selector can reach it. Nesting
    // it any deeper would force every consumer to invent a class on the input
    // to style the error state.
    expect_that!(html, contains_substring(r#"/><ul class="field-errors">"#));
}

#[gtest]
fn a_clean_field_has_no_aria_invalid_attribute_at_all() {
    // NOT `aria-invalid="false"`: per ARIA that asserts "checked and passed",
    // so an untouched form would claim to have validated every field. Absent
    // is the only neutral value.
    let html = render_to_html(EmptyInput);
    expect_that!(html, not(contains_substring("aria-invalid")));
}

// ── `type=` reaching the rendered input ──────────────────────────────────
//
// `Input` renders every `<input type=…>` there is, so the type has to travel
// from the spec's `WidgetType` all the way to the attribute. Before these, both
// leaves hardcoded `type="text"` and a `password` override rendered a visible
// text box.

/// A one-`String` model, so a rendered `input` is unambiguous.
#[derive(Facet, Clone, Debug, PartialEq)]
struct OneString {
    secret: String,
}

/// One `u32`, to pin that a numeric still renders as text.
#[derive(Facet, Clone, Debug, PartialEq)]
struct OneNumber {
    count: u32,
}

fn rendered_with(widget: WidgetType) -> String {
    // A thread-local rather than a prop, because `render_to_html` takes a plain
    // `fn() -> Element` — there is nowhere to thread an argument through.
    WIDGET.replace(Some(widget));
    render_to_html(WithWidget)
}

thread_local! {
    static WIDGET: std::cell::RefCell<Option<WidgetType>> =
        const { std::cell::RefCell::new(None) };
}

#[component]
fn WithWidget() -> Element {
    let widget = WIDGET.with_borrow(|c| c.clone().expect("set by rendered_with"));
    let form = use_form(|| {
        empty_form(FormSpec::<OneString>::default().with_custom_widget("secret", widget))
    });
    form.render_fragment()
}

#[gtest]
fn every_input_type_reaches_the_type_attribute() {
    // Every variant that is selectable, with the `type=` HTML actually wants.
    // `tel` and `datetime-local` are the two where the variant name and the
    // attribute diverge, which is the whole reason `html_type` exists.
    let cases = [
        (InputType::Text, "text"),
        (InputType::Password, "password"),
        (InputType::Number, "number"),
        (InputType::Email, "email"),
        (InputType::Telephone, "tel"),
        (InputType::Url, "url"),
        (InputType::Search, "search"),
        (InputType::Color, "color"),
        (InputType::Date, "date"),
        (InputType::Time, "time"),
        (InputType::DatetimeLocal, "datetime-local"),
        (InputType::Month, "month"),
        (InputType::Week, "week"),
    ];
    for (input_type, expected) in cases {
        let html = rendered_with(WidgetType::Input(input_type.clone()));
        let wanted = format!("type=\"{expected}\"");
        expect_that!(
            html,
            contains_substring(wanted.as_str()),
            "for {input_type:?}"
        );
    }
}

#[gtest]
fn a_numeric_field_still_renders_as_text() {
    // Deliberate, and easy to "fix" by accident: `type="number"` hands back `""`
    // for anything the browser dislikes, so a half-typed value vanishes
    // mid-keystroke. `ValueKind::Int` is what parses the string back.
    #[component]
    fn NumberForm() -> Element {
        let form = use_form(|| empty_form(FormSpec::<OneNumber>::default()));
        form.render_fragment()
    }
    let html = render_to_html(NumberForm);
    expect_that!(html, contains_substring("type=\"text\""));
    expect_that!(html, not(contains_substring("type=\"number\"")));
}

#[gtest]
fn a_numeric_field_can_still_ask_for_a_number_input() {
    // The override half of `a_numeric_field_still_renders_as_text`: text is the
    // DEFAULT, not the only option. Someone who wants the spinner and the mobile
    // numeric keypad, and accepts that the browser may hand back `""`, can say so.
    let html = rendered_with(WidgetType::Input(InputType::Number));
    expect_that!(html, contains_substring("type=\"number\""));
}

// ── A password is an ordinary controlled input ───────────────────────────
//
// It WAS special — `value` withheld, per Django's `render_value=False`. That
// convention is about an HTTP response body carrying the secret into proxy logs
// and view-source, which is a server-rendering concern with no analogue here: the
// value goes keystroke -> DOM -> a wasm-side store in the same browser. Withholding
// it made the field impossible to clear programmatically and impossible to type
// into, twice, in two different ways. See `a_keystroke_writes_the_value_attribute`
// for the mechanism that made "just omit it" fail.

#[gtest]
fn a_password_round_trips_its_value_like_any_other_field() {
    #[component]
    fn FilledPassword() -> Element {
        let form = use_form(|| {
            form_for(
                &OneString {
                    secret: "hunter2".to_string(),
                },
                FormSpec::<OneString>::default()
                    .with_custom_widget("secret", WidgetType::Input(InputType::Password)),
            )
        });
        form.render_fragment()
    }
    let html = render_to_html(FilledPassword);
    expect_that!(html, contains_substring("type=\"password\""));
    expect_that!(html, contains_substring("value=\"hunter2\""));
}

#[gtest]
fn typing_into_a_password_reaches_the_value_map() {
    let html = type_into(PasswordWithEcho, "hunter2");
    expect_that!(html, contains_substring("<p class=\"echo\">hunter2</p>"));
}

// ── A hidden input has no chrome ──────────────────────────────────────────

#[gtest]
fn a_hidden_input_renders_without_a_label_or_marker() {
    // The wrapper every other leaf uses is a `label` with a caption and a
    // required marker. Around `type="hidden"` that puts visible text and an
    // asterisk on screen beside a widget nobody can see.
    let html = rendered_with(WidgetType::Input(InputType::Hidden));
    expect_that!(html, contains_substring("type=\"hidden\""));
    expect_that!(html, not(contains_substring("<label")));
    expect_that!(html, not(contains_substring("field-label")));
    expect_that!(html, not(contains_substring("required")));
}

/// A password input beside a plain readout of what the store holds for its path.
///
/// The echo is the only way to observe the write: the input itself must NOT show
/// the value, so the rendered `value=` attribute cannot be the evidence.
#[component]
fn PasswordWithEcho() -> Element {
    let values = use_store(HashMap::<String, String>::new);
    let stored = values.read().get("secret").cloned().unwrap_or_default();
    rsx! {
        ScalarWidget {
            value_kind: text_kind(),
            widget: WidgetType::Input(InputType::Password),
            values,
            props: FieldProps {
                path: "secret".to_string(),
                label: Some("Secret".to_string()),
                required: true,
                errors: Vec::new(),
            },
        }
        p { class: "echo", "{stored}" }
    }
}

/// The mutations a keystroke produces, for asserting on what Dioxus TOUCHES
/// rather than on what the HTML happens to say.
fn mutations_after_typing(app: fn() -> Element, text: &str) -> String {
    set_event_converter(Box::new(SerializedHtmlEventConverter));
    let mut dom = VirtualDom::new(app);
    let mounted = dom.rebuild_to_vec();
    let input_id = mounted
        .edits
        .iter()
        .find_map(|e| match e {
            Mutation::NewEventListener { name, id } if name == "input" => Some(*id),
            _ => None,
        })
        .expect("an `oninput` listener");
    let payload = PlatformEventData::new(Box::new(SerializedFormData::new(
        text.to_string(),
        Vec::new(),
    )));
    let event: Event<dyn Any> = Event::new(Rc::new(payload), true);
    dom.runtime().handle_event("input", event, input_id);
    format!("{:?}", dom.render_immediate_to_vec().edits)
}

#[gtest]
fn a_keystroke_writes_the_value_attribute() {
    // Every input here is CONTROLLED: the store is the source of truth and `value`
    // is reasserted from it on each re-render. Worth pinning, because the mechanism
    // is what defeated two attempts to withhold a password's value.
    //
    // `value=""` failed the obvious way — the empty attribute was reasserted and
    // wiped each keystroke. `value: None` failed less obviously: the rendered HTML
    // omitted the attribute, but Dioxus RE-EMITS an unchanged `None` attribute on
    // every re-render, and `None` for `value` is not "leave it alone" — the
    // interpreter runs `node.value = ""; node.removeAttribute("value")`. Same wipe,
    // one layer down. `aria_invalid` survives `None` only because no DOM state
    // hangs off it.
    let edits = mutations_after_typing(PopulatedInput, "Trivia Night");
    expect_that!(edits, contains_substring("SetAttribute"));
    expect_that!(edits, contains_substring("Trivia Night"));
}
