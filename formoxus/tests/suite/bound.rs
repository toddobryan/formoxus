//! The read-only views of the value and error stores (#22) and the bound
//! field every widget is handed (#23), `BOUND_PLAN.md` box 2.
//!
//! Written before the code, from the plan: `ValuesView` (`get`, `peek`,
//! `paths_under`), `ErrorsView` (`at`, `validity`), `BoundField<K>`'s common
//! getters, the leaf kind through `WidgetProps`'s `Deref`, and the edit kinds
//! through a dev's adder. The `Variant`, `List` and `Set` kinds' own buttons
//! (Remove, Edit, ↑/↓, the variant select) are covered by the tests that already
//! click them in `vecs.rs`, `sets.rs` and `enums.rs`, which go through the
//! bound fields once the built-in widgets do. What must NOT compile is pinned
//! in `tests/ui-pending/bound_*.rs`.

use std::cell::Cell;
use std::collections::BTreeSet;

use dioxus::prelude::*;
use facet::Facet;
use formoxus::nodes::ItemValue;
use formoxus::path::NodePath;
use formoxus::prelude::*;
use formoxus::widgets::{AddItemProps, LeafWidget, Widget, WidgetProps};
use googletest::prelude::*;

use super::{Harness, render_to_html};

thread_local! {
    static ECHO_RENDERS: Cell<usize> = const { Cell::new(0) };
    static PEEK_RENDERS: Cell<usize> = const { Cell::new(0) };
    static ADDER_RENDERS: Cell<usize> = const { Cell::new(0) };
}

fn bump(counter: &'static std::thread::LocalKey<Cell<usize>>) {
    counter.with(|c| c.set(c.get() + 1));
}

fn renders(counter: &'static std::thread::LocalKey<Cell<usize>>) -> usize {
    counter.with(Cell::get)
}

fn path(p: &str) -> NodePath {
    NodePath::new(p)
}

// ── ValuesView ──────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Person {
    name: String,
    nickname: String,
}

/// No value is normal, not an error: a path the store has never seen reads
/// as `""`, through `get` and `peek` alike.
#[gtest]
fn an_absent_key_reads_as_empty() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(FormSpec::<Person>::default()));
        let (got, peeked) = (
            form.values().get(&path("no.such.path")),
            form.values().peek(&path("no.such.path")),
        );
        rsx! { p { "[{got}] [{peeked}]" } }
    }
    expect_that!(render_to_html(App), contains_substring("[] []"));
}

#[gtest]
fn get_reads_what_was_written() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(FormSpec::<Person>::default()));
        form.write(&path!(Person.name), "Ada")
            .expect("name is a field of this form");
        let name = form.values().get(&path("name"));
        rsx! { p { "[{name}]" } }
    }
    expect_that!(render_to_html(App), contains_substring("[Ada]"));
}

/// A widget that shows ANOTHER field's value, `name`, read through its bound
/// field's `values()`, and counts its own renders. `PeekEcho` reads with
/// `peek` instead, which must subscribe nothing.
#[derive(Clone, Debug, Default, PartialEq)]
struct Echo;

impl Widget for Echo {
    const NAME: &'static str = "echo";
    const EXTENDS: formoxus::attrs::AttrSet = formoxus::attrs::AttrSet::EMPTY;
    const PARTS: &'static [formoxus::attrs::Tag] = &[];
}

impl LeafWidget for Echo {
    fn render(&self, props: WidgetProps) -> Element {
        rsx! { EchoView { field: props } }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct PeekEcho;

impl Widget for PeekEcho {
    const NAME: &'static str = "peek_echo";
    const EXTENDS: formoxus::attrs::AttrSet = formoxus::attrs::AttrSet::EMPTY;
    const PARTS: &'static [formoxus::attrs::Tag] = &[];
}

impl LeafWidget for PeekEcho {
    fn render(&self, props: WidgetProps) -> Element {
        rsx! { PeekView { field: props } }
    }
}

/// Its own component, so the count is this widget's scope, not the form's.
#[component]
fn EchoView(field: WidgetProps) -> Element {
    bump(&ECHO_RENDERS);
    let name = field.values().get(&path("name"));
    rsx! { output { class: "echo", "{name}" } }
}

#[component]
fn PeekView(field: WidgetProps) -> Element {
    bump(&PEEK_RENDERS);
    let name = field.values().peek(&path("name"));
    rsx! { output { class: "peek", "{name}" } }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trio {
    name: String,
    nickname: String,
    echo: String,
}

#[component]
fn EchoForm() -> Element {
    let form = use_form(|| empty_form(form! { Trio { echo => { widget: Echo } } }));
    form.render(using_fns! {})
}

#[component]
fn PeekForm() -> Element {
    let form = use_form(|| empty_form(form! { Trio { echo => { widget: PeekEcho } } }));
    form.render(using_fns! {})
}

/// `name` and `nickname`, in that order: the echo renders no input of its own.
fn name_and_nickname(app: &Harness) -> (dioxus::core::ElementId, dioxus::core::ElementId) {
    let ids = app.listeners("input");
    (ids[0], ids[1])
}

/// **`get` subscribes to one key.** Typing into `nickname` must not wake a
/// reader of `name`; typing into `name` must. A `get` that read the whole map
/// would fail the first half, one that read nothing the second.
#[gtest]
fn get_subscribes_to_the_key_it_reads_and_no_other() {
    let mut app = Harness::mount(EchoForm);
    let (name, nickname) = name_and_nickname(&app);
    let at_mount = renders(&ECHO_RENDERS);

    app.fire("input", nickname, "Bun");
    expect_that!(renders(&ECHO_RENDERS), eq(at_mount));

    app.fire("input", name, "Ada");
    expect_that!(renders(&ECHO_RENDERS), gt(at_mount));
    expect_that!(
        app.html(),
        contains_substring(r#"<output class="echo">Ada</output>"#)
    );
}

/// **`peek` subscribes to nothing,** for an event handler that reads a value:
/// typing into the very key it read does not wake it.
#[gtest]
fn peek_subscribes_to_nothing() {
    let mut app = Harness::mount(PeekForm);
    let (name, _) = name_and_nickname(&app);
    let at_mount = renders(&PEEK_RENDERS);

    app.fire("input", name, "Ada");
    expect_that!(renders(&PEEK_RENDERS), eq(at_mount));
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Answers {
    answers: Vec<String>,
    answers_note: String,
}

fn two_answers() -> Answers {
    Answers {
        answers: vec!["alpha".to_string(), "beta".to_string()],
        answers_note: "a note".to_string(),
    }
}

/// Every key under the list, and only those: `answers_note` shares the
/// list's name as a string prefix, but is not under it. A prefix test on the
/// raw string would count it.
#[gtest]
fn paths_under_a_list_are_its_items_and_nothing_beside_it() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| form_for(&two_answers(), FormSpec::default()));
        let mut under: Vec<String> = form
            .values()
            .paths_under(&path("answers"))
            .iter()
            .map(ToString::to_string)
            .collect();
        under.sort();
        let under = under.join(" ");
        rsx! { p { "[{under}]" } }
    }
    expect_that!(
        render_to_html(App),
        contains_substring("[answers.#0 answers.#1]")
    );
}

/// From the root, every key in the store.
#[gtest]
fn paths_under_the_root_are_every_path() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| form_for(&two_answers(), FormSpec::default()));
        let count = form.values().paths_under(&NodePath::root()).len();
        rsx! { p { "[{count}]" } }
    }
    expect_that!(render_to_html(App), contains_substring("[3]"));
}

/// An adder that counts the keys under its list and its own renders, and adds
/// a filled `"gamma"` through its bound field.
#[component]
fn CountingAdder(props: AddItemProps) -> Element {
    bump(&ADDER_RENDERS);
    let field = props.field;
    let stored = field.values().paths_under(field.path()).len();
    let label = "Add gamma";
    rsx! {
        button {
            r#type: "button",
            aria_label: "{label}",
            onclick: move |_| field.add(Some(ItemValue::new("gamma".to_string()))),
            "stored {stored}"
        }
    }
}

#[component]
fn CountingAdderForm() -> Element {
    let form = use_form(|| {
        form_for(
            &two_answers(),
            form! { Answers { answers => { adder: CountingAdder } } },
        )
    });
    form.render(using_fns! {})
}

/// **`paths_under` wakes on a new key, not on a keystroke.** Typing into an
/// item writes a key that is already there, which must not re-render a reader
/// of the list's keys, or every adder would re-render on every keystroke in
/// its list.
#[gtest]
fn paths_under_does_not_wake_on_a_keystroke_in_an_item() {
    let mut app = Harness::mount(CountingAdderForm);
    let first_item = app.listeners("input")[0];
    let at_mount = renders(&ADDER_RENDERS);

    app.fire("input", first_item, "alphabet");
    expect_that!(renders(&ADDER_RENDERS), eq(at_mount));
}

#[gtest]
fn a_devs_adder_adds_to_its_list_through_its_field() {
    let mut app = Harness::mount(CountingAdderForm);
    app.click(app.labelled("click", "Add gamma"));
    let html = app.html();
    expect_that!(html, contains_substring(r#"value="gamma""#));
    expect_that!(html, contains_substring("stored 3"));
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Tags {
    tags: BTreeSet<String>,
}

#[component]
fn CountingAdderSetForm() -> Element {
    let form = use_form(|| empty_form(form! { Tags { tags => { adder: CountingAdder } } }));
    form.render(using_fns! {})
}

/// The same adder on a set: one `ItemAdder` serves both, through the `Items`
/// kind, and its `add` reaches the set, not a list.
#[gtest]
fn the_same_adder_adds_to_a_set_through_its_field() {
    let mut app = Harness::mount(CountingAdderSetForm);
    app.click(app.labelled("click", "Add gamma"));
    let html = app.html();
    expect_that!(html, contains_substring("gamma"));
}

// ── ErrorsView ──────────────────────────────────────────────────────────

/// The three states, from a state seeded with them: `name` invalid,
/// `nickname` checked and valid, and a path nobody checked.
#[gtest]
fn the_errors_view_reads_all_three_states() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            let mut state = empty_form(FormSpec::<Person>::default());
            state.field_errors.insert(
                path("name"),
                vec![ValidationMessage("Too short.".to_string())],
            );
            state.field_errors.insert(path("nickname"), Vec::new());
            state
        });
        let errors = form.errors();
        let count = |p: &str| errors.at(&path(p)).map(|messages| messages.len());
        let says = format!(
            "name {:?} {:?} / nickname {:?} {:?} / other {:?} {:?}",
            count("name"),
            errors.validity(&path("name")),
            count("nickname"),
            errors.validity(&path("nickname")),
            count("other"),
            errors.validity(&path("other")),
        );
        rsx! { p { "{says}" } }
    }
    let html = render_to_html(App);
    expect_that!(html, contains_substring("name Some(1) Invalid"));
    expect_that!(html, contains_substring("nickname Some(0) Valid"));
    expect_that!(html, contains_substring("other None Unchecked"));
}

// ── The leaf kind, through `WidgetProps` ────────────────────────────────

/// A widget written only against the props' `Deref` to its bound field: its
/// label, value and validity read from the props, its input's handlers from
/// the props, and a Fill button whose handler owns `props.field()`.
#[derive(Clone, Debug, Default, PartialEq)]
struct Bare;

impl Widget for Bare {
    const NAME: &'static str = "bare";
    const EXTENDS: formoxus::attrs::AttrSet = formoxus::attrs::AttrSet::EMPTY;
    const PARTS: &'static [formoxus::attrs::Tag] = &[];
}

impl LeafWidget for Bare {
    fn render(&self, props: WidgetProps) -> Element {
        rsx! { BareView { props } }
    }
}

#[component]
fn BareView(props: WidgetProps) -> Element {
    let field = props.field();
    let label = props.label().unwrap_or("unlabelled").to_string();
    let fill = format!("Fill {label}");
    rsx! {
        span { class: "bare-label", "{label}" }
        input {
            id: "{props.id()}",
            name: "{props.path()}",
            value: props.current(),
            oninput: props.on_input(),
            onblur: props.on_blur(),
        }
        span { class: "bare-state", "invalid={props.is_invalid()}" }
        button {
            r#type: "button",
            aria_label: "{fill}",
            onclick: move |_| field.change("filled".to_string()),
            "Fill"
        }
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Handle {
    nickname: String,
}

#[component]
fn BareForm() -> Element {
    let form = use_form(|| {
        empty_form(form! {
            Handle {
                live_validation: on,
                nickname => {
                    widget: Bare,
                    label: "Nick",
                    validator: |v| if v.len() < 3 { vec!["Too short.".into()] } else { Vec::new() },
                },
            }
        })
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_bound_fields_getters_read_its_own_node() {
    let html = render_to_html(BareForm);
    expect_that!(
        html,
        contains_substring(r#"<span class="bare-label">Nick</span>"#)
    );
    expect_that!(html, contains_substring(r#"name="nickname""#));
    expect_that!(html, contains_regex(r#"id="[A-Za-z0-9]{8}-nickname""#));
}

/// The props' handlers are the bound field's, so a widget written only
/// against them is checked as the user goes: on leaving the field after
/// typing, then on every keystroke while it shows an error.
#[gtest]
fn a_widget_using_only_the_props_is_live_checked() {
    let mut app = Harness::mount(BareForm);
    let (input, blur) = (app.only_listener("input"), app.only_listener("blur"));

    app.fire("input", input, "ab");
    expect_that!(app.html(), contains_substring("invalid=false"));
    app.blur(blur);
    let html = app.html();
    expect_that!(html, contains_substring("invalid=true"));
    expect_that!(html, contains_substring("Too short."));

    app.fire("input", input, "abc");
    expect_that!(app.html(), contains_substring("invalid=false"));
}

/// `props.field()` is a bound field a handler can own: its `change` writes
/// the value and checks it at once, as a chooser's does.
#[gtest]
fn a_handler_owning_props_field_writes_and_checks() {
    let mut app = Harness::mount(BareForm);
    app.click(app.labelled("click", "Fill Nick"));
    let html = app.html();
    expect_that!(html, contains_substring(r#"value="filled""#));
    expect_that!(html, contains_substring("invalid=false"));
    expect_that!(html, not(contains_substring("Too short.")));
}
