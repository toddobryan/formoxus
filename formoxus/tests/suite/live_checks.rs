//! A field checked as the user goes, driven through its widget's events
//! (`ERRORS_PLAN.md` 8b, 8c's store writes, 8e).
//!
//! `FormState::check_live`'s own inline tests cover what a check FINDS. These
//! cover when one runs and what it writes: the triggers (A: nothing while first
//! typing; E: leaving a field typed into, or changing a chooser), the store
//! writes `Form::check_live` makes, and the three `aria-invalid` states they
//! render (#7). What only a browser shows, focus and the caret, is 8d's.

use std::cell::Cell;

use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;
use formoxus::widgets::{Widget, WidgetProps};
use googletest::prelude::*;

use super::Harness;

const REQUIRED: &str = "This field is required.";
const TOO_SHORT: &str = "length must be at least 3";

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    name: String,
}

#[component]
fn NameForm() -> Element {
    let form = use_form(|| empty_form(form! { Signup { name => { min_length: 3 } } }));
    form.render(using_fns! {})
}

fn type_into(app: &mut Harness, value: &str) {
    app.fire("input", app.only_listener("input"), value);
}

fn leave(app: &mut Harness) {
    app.blur(app.only_listener("blur"));
}

// ── When a check runs (A, E) ─────────────────────────────────────────────

/// **A: typing into a field the first time shows nothing,** however wrong,
/// until the user leaves it.
#[gtest]
fn typing_a_first_time_shows_nothing() {
    let mut app = Harness::mount(NameForm);
    type_into(&mut app, "A");
    let html = app.html();
    expect_that!(html, not(contains_substring(TOO_SHORT)));
    expect_that!(html, not(contains_substring("aria-invalid")));
}

#[gtest]
fn leaving_a_field_typed_into_shows_its_error() {
    let mut app = Harness::mount(NameForm);
    type_into(&mut app, "A");
    leave(&mut app);
    let html = app.html();
    expect_that!(html, contains_substring(TOO_SHORT));
    expect_that!(html, contains_substring(r#"aria-invalid="true""#));
}

/// **E: tabbing through an empty required field says nothing.** Only a field
/// typed into since its last check is checked on leaving.
#[gtest]
fn leaving_a_field_not_typed_into_says_nothing() {
    let mut app = Harness::mount(NameForm);
    leave(&mut app);
    let html = app.html();
    expect_that!(html, not(contains_substring(REQUIRED)));
    expect_that!(html, not(contains_substring("aria-invalid")));
}

/// **Once it shows an error, each keystroke checks it,** without waiting for
/// a blur: the error goes on the keystroke that fixes it, and the field says
/// `"false"` (#7).
#[gtest]
fn the_keystroke_that_fixes_an_error_removes_it() {
    let mut app = Harness::mount(NameForm);
    type_into(&mut app, "A");
    leave(&mut app);
    type_into(&mut app, "Ada");
    let html = app.html();
    expect_that!(html, not(contains_substring(TOO_SHORT)));
    expect_that!(html, contains_substring(r#"aria-invalid="false""#));
}

#[gtest]
fn a_keystroke_that_leaves_it_wrong_keeps_the_error() {
    let mut app = Harness::mount(NameForm);
    type_into(&mut app, "A");
    leave(&mut app);
    type_into(&mut app, "Ad");
    expect_that!(app.html(), contains_substring(TOO_SHORT));
}

/// **The first keystroke after a clean check drops `"false"`,** since the
/// value it vouched for is gone, and the next error waits for a blur again,
/// as on a first visit (A).
#[gtest]
fn after_a_fix_the_next_error_waits_for_a_blur() {
    let mut app = Harness::mount(NameForm);
    type_into(&mut app, "A");
    leave(&mut app);
    type_into(&mut app, "Ada");
    type_into(&mut app, "Ad");
    let html = app.html();
    expect_that!(html, not(contains_substring(TOO_SHORT)));
    expect_that!(html, not(contains_substring("aria-invalid")));

    leave(&mut app);
    expect_that!(app.html(), contains_substring(TOO_SHORT));
}

// ── A server's error (E) ─────────────────────────────────────────────────

#[component]
fn TakenName() -> Element {
    let form = use_form(|| {
        form_for(
            &Signup {
                name: "Ada".to_string(),
            },
            form! { Signup {} },
        )
    });
    use_hook(|| {
        form.push_field_error(&path!(Signup.name), "That name is taken.")
            .expect("name is a field of this form");
    });
    form.render(using_fns! {})
}

/// **E: a click in and out does not wipe what only the server knows.**
/// Nothing was typed, so nothing is checked, and a local check could not
/// re-derive "taken" anyway.
#[gtest]
fn a_servers_error_survives_a_click_in_and_out() {
    let mut app = Harness::mount(TakenName);
    leave(&mut app);
    expect_that!(app.html(), contains_substring("That name is taken."));
}

/// Typing into it is a new answer, checked at once like any field showing an
/// error. "Taken" goes; whether the new name is taken is the next submit's.
#[gtest]
fn typing_into_a_field_with_a_servers_error_checks_it_again() {
    let mut app = Harness::mount(TakenName);
    type_into(&mut app, "Grace");
    let html = app.html();
    expect_that!(html, not(contains_substring("That name is taken.")));
    expect_that!(html, contains_substring(r#"aria-invalid="false""#));
}

// ── A chooser checks on change (E) ───────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Terms {
    agreed: bool,
}

#[component]
fn TermsForm() -> Element {
    let form = use_form(|| {
        form_for(
            &Terms { agreed: true },
            form! { Terms { agreed => { required_true } } },
        )
    });
    form.render(using_fns! {})
}

/// A checkbox has no typing to wait out and no `onblur`, so its change is
/// the blur: unticking a box that must be ticked says so at once.
#[gtest]
fn unticking_a_box_that_must_be_ticked_says_so_at_once() {
    let mut app = Harness::mount(TermsForm);
    app.fire("change", app.only_listener("change"), "false");
    expect_that!(app.html(), contains_substring(r#"aria-invalid="true""#));
}

// ── Siblings under an option the edit left absent ───────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    street: String,
    city: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Shipping {
    address: Option<Address>,
}

/// Street filled and city blank, then submitted: city is "required", and
/// street is checked and clean.
#[component]
fn HalfAnAddress() -> Element {
    let form = use_form(|| {
        form_for(
            &Shipping {
                address: Some(Address {
                    street: "1 Main St".to_string(),
                    city: String::new(),
                }),
            },
            form! { Shipping {} },
        )
    });
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

/// Street first in the markup, so first among a fresh mount's listeners.
fn street(app: &Harness, event: &str) -> dioxus::core::ElementId {
    app.listeners(event)[0]
}

#[gtest]
fn half_an_address_after_submit_has_cities_error() {
    let html = Harness::mount(HalfAnAddress).html();
    expect_that!(html, contains_substring(REQUIRED));
    expect_that!(html, contains_substring(r#"aria-invalid="false""#));
}

/// **Blanking the last filled field clears its siblings' errors.** With every
/// leaf blank the address is absent, which is valid, so city's "required" no
/// longer applies; and nothing under an absent option is checked, so neither
/// field claims `"false"` either.
#[gtest]
fn blanking_the_last_filled_field_clears_its_siblings_errors() {
    let mut app = Harness::mount(HalfAnAddress);
    app.fire("input", street(&app, "input"), "");
    app.blur(street(&app, "blur"));
    let html = app.html();
    expect_that!(html, not(contains_substring(REQUIRED)));
    expect_that!(html, not(contains_substring("aria-invalid")));
}

/// While the address stays present, a sibling's error stands: a live check
/// only ever clears under an absent option (B).
#[gtest]
fn a_siblings_error_survives_while_the_option_stays_present() {
    let mut app = Harness::mount(HalfAnAddress);
    app.fire("input", street(&app, "input"), "2 Main St");
    app.blur(street(&app, "blur"));
    expect_that!(app.html(), contains_substring(REQUIRED));
}

// ── aria-invalid="false" (#7) ────────────────────────────────────────────

#[component]
fn SeededEdit() -> Element {
    let form = use_form(|| {
        form_for(
            &Shipping {
                address: Some(Address {
                    street: "1 Main St".to_string(),
                    city: "Springfield".to_string(),
                }),
            },
            form! { Shipping {} },
        )
    });
    form.render(using_fns! {})
}

/// **A seeded edit form claims nothing on load.** Its values are the model's,
/// but nothing has checked them, and validating on load is not part of this.
#[gtest]
fn a_seeded_edit_form_claims_nothing_on_load() {
    expect_that!(
        Harness::mount(SeededEdit).html(),
        not(contains_substring("aria-invalid"))
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
#[allow(dead_code)]
enum Shape {
    Circle { radius: f64 },
    Square { side: f64 },
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Drawing {
    shape: Shape,
}

#[component]
fn ValidatedCircle() -> Element {
    let form = use_form(|| {
        form_for(
            &Drawing {
                shape: Shape::Circle { radius: 1.0 },
            },
            form! { Drawing {} },
        )
    });
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

/// The variant select's own tag, which `VariantNode` gives the errors'
/// attributes itself.
fn select_tag(html: &str) -> String {
    let start = html.find("<select").expect("a variant select");
    let end = start + html[start..].find('>').expect("the tag closes");
    html[start..=end].to_string()
}

/// **Choosing a variant drops the enum select's `"false"`.** The check vouched
/// for the variant chosen then, and choosing removes the enum's key.
#[gtest]
fn choosing_a_variant_drops_the_selects_false() {
    let mut app = Harness::mount(ValidatedCircle);
    expect_that!(
        select_tag(&app.html()),
        contains_substring(r#"aria-invalid="false""#)
    );
    app.fire("change", app.only_listener("change"), "Square");
    expect_that!(
        select_tag(&app.html()),
        not(contains_substring("aria-invalid"))
    );
}

// ── Only the field whose errors changed re-renders (8b) ──────────────────

thread_local! {
    static FORM_RENDERS: Cell<usize> = const { Cell::new(0) };
    static PROBE_RENDERS: Cell<usize> = const { Cell::new(0) };
}

fn bump(counter: &'static std::thread::LocalKey<Cell<usize>>) {
    counter.with(|c| c.set(c.get() + 1));
}

fn renders(counter: &'static std::thread::LocalKey<Cell<usize>>) -> usize {
    counter.with(Cell::get)
}

/// A custom widget that counts its own renders. It reads its errors and
/// nothing else, through an uncontrolled `<input>`, so the count moves only
/// when its errors are written: its own value's writes do not wake it.
#[derive(Clone, Debug, Default, PartialEq)]
struct Probe;

impl Widget for Probe {
    const NAME: &'static str = "probe";
    const EXTENDS: formoxus::attrs::AttrSet = formoxus::attrs::AttrSet::EMPTY;
    const PARTS: &'static [formoxus::attrs::Tag] = &[];

    fn render(&self, props: WidgetProps) -> Element {
        rsx! { ProbeView { field: props } }
    }
}

/// Its own component, as a widget reading the store must be, so the count is
/// this field's scope and not the form's.
#[component]
fn ProbeView(field: WidgetProps) -> Element {
    bump(&PROBE_RENDERS);
    let handle = field.field_handle();
    let validity = field.validity();
    rsx! {
        input { id: "{field.id}", oninput: handle.on_input(), onblur: handle.on_blur() }
        span { class: "probe", "{validity:?}" }
    }
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Pair {
    name: String,
    nickname: String,
}

#[component]
fn PairForm() -> Element {
    bump(&FORM_RENDERS);
    let form = use_form(|| {
        empty_form(form! {
            Pair {
                nickname => {
                    widget: Probe,
                    validator: |v| if v.len() < 3 { vec!["Too short.".into()] } else { Vec::new() },
                },
            }
        })
    });
    form.render(using_fns! {})
}

/// `name` first in the markup, so first among a fresh mount's listeners;
/// the probe, `nickname`, second.
fn name_then_probe(
    app: &Harness,
    event: &str,
) -> (dioxus::core::ElementId, dioxus::core::ElementId) {
    let ids = app.listeners(event);
    (ids[0], ids[1])
}

/// **One field's error wakes neither its sibling nor the form.** The probe
/// reads its errors by its own key, so `name`'s new key must not reach it; and
/// a live check writes only the store, so the form's scope stays asleep.
#[gtest]
fn one_fields_error_wakes_neither_a_sibling_nor_the_form() {
    let mut app = Harness::mount(PairForm);
    let (name_input, _) = name_then_probe(&app, "input");
    let (name_blur, _) = name_then_probe(&app, "blur");
    let (forms, probes) = (renders(&FORM_RENDERS), renders(&PROBE_RENDERS));

    app.fire("input", name_input, "");
    app.blur(name_blur);

    expect_that!(app.html(), contains_substring(REQUIRED));
    expect_that!(renders(&PROBE_RENDERS), eq(probes));
    expect_that!(renders(&FORM_RENDERS), eq(forms));
}

/// **A check that finds what was already there writes nothing,** so the
/// field does not wake on every keystroke while it stays wrong. It wakes when
/// the error first appears, and when it goes.
#[gtest]
fn an_unchanged_result_does_not_wake_the_field() {
    let mut app = Harness::mount(PairForm);
    let (_, probe_input) = name_then_probe(&app, "input");
    let (_, probe_blur) = name_then_probe(&app, "blur");
    let at_mount = renders(&PROBE_RENDERS);

    app.fire("input", probe_input, "a");
    expect_that!(
        renders(&PROBE_RENDERS),
        eq(at_mount),
        "typing alone checks nothing"
    );
    app.blur(probe_blur);
    expect_that!(
        renders(&PROBE_RENDERS),
        eq(at_mount + 1),
        "the error appears"
    );
    app.fire("input", probe_input, "ab");
    expect_that!(
        renders(&PROBE_RENDERS),
        eq(at_mount + 1),
        "the same error again"
    );
    app.fire("input", probe_input, "abc");
    expect_that!(renders(&PROBE_RENDERS), eq(at_mount + 2), "the error goes");
    expect_that!(
        app.html(),
        contains_substring(r#"<span class="probe">Valid</span>"#)
    );
}

// ── A live check and a submit agree (8b) ─────────────────────────────────

#[component]
fn ShortNameSubmitted() -> Element {
    let form = use_form(|| {
        form_for(
            &Signup {
                name: "A".to_string(),
            },
            form! { Signup { name => { min_length: 3 } } },
        )
    });
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

/// **A submit's errors arrive where a live check's would,** with the same
/// markup: one `validate` decides both, run on a clone of the node for the
/// live check (trap 5).
#[gtest]
fn a_submit_and_a_live_check_render_the_same_field() {
    let mut live = Harness::mount(NameForm);
    type_into(&mut live, "A");
    leave(&mut live);
    let submitted = Harness::mount(ShortNameSubmitted);
    expect_that!(
        super::without_form_id(&live.html()),
        eq(&super::without_form_id(&submitted.html()))
    );
}

// ── The wire and a seeded state (8b) ─────────────────────────────────────

/// Declared out of alphabetical order, so render order and a sorted (or
/// hashed) order disagree.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Backwards {
    zeta: String,
    alpha: String,
}

/// The form, and the error paths `to_wire` would send, read in render so a
/// store write shows.
fn with_wire_errors(form: &Form<Backwards>) -> Element {
    let paths = form.to_wire().errors().paths().join(" ");
    rsx! {
        {form.render(using_fns! {})}
        pre { id: "wire", "[{paths}]" }
    }
}

#[component]
fn BackwardsSubmitted() -> Element {
    let form = use_form(|| empty_form(form! { Backwards {} }));
    use_hook(|| form.validate());
    with_wire_errors(&form)
}

/// **`to_wire` keeps render order,** not the store's `HashMap` order.
#[gtest]
fn to_wire_lists_errors_in_render_order() {
    expect_that!(
        Harness::mount(BackwardsSubmitted).html(),
        contains_substring("[zeta alpha]")
    );
}

#[component]
fn BackwardsLive() -> Element {
    let form = use_form(|| empty_form(form! { Backwards {} }));
    with_wire_errors(&form)
}

/// **A live check's error reaches the wire,** though it was written only to
/// the store and never to the state: `to_wire` reads the store.
#[gtest]
fn a_live_checks_error_reaches_the_wire() {
    let mut app = Harness::mount(BackwardsLive);
    let alpha_input = app.listeners("input")[1];
    let alpha_blur = app.listeners("blur")[1];
    app.fire("input", alpha_input, "");
    app.blur(alpha_blur);
    expect_that!(app.html(), contains_substring("[alpha]"));
}

#[component]
fn SeededWithAnError() -> Element {
    let form = use_form(|| {
        let mut state = empty_form(form! { Signup {} });
        state
            .field_errors
            .insert("name".to_string(), vec!["Taken.".into()]);
        state
    });
    form.render(using_fns! {})
}

/// **A state handed to `use_form` with errors shows them,** as a server's
/// rejected submit, rebuilt on the client, would.
#[gtest]
fn a_seeded_states_errors_show() {
    let html = Harness::mount(SeededWithAnError).html();
    expect_that!(html, contains_substring("Taken."));
    expect_that!(html, contains_substring(r#"aria-invalid="true""#));
}
