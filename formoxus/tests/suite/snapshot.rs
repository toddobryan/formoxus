//! Whole-form markup snapshots: every class slot formoxus emits, pinned byte
//! for byte, so that P3 (configurable classes, `formoxus/ATTRIBUTES_PLAN.md`)
//! can replace each literal with a slot lookup and PROVE `Classes::fx()`
//! renders exactly what the literals did.
//!
//! One model touches every widget part and fixed slot in the plan's inventory,
//! rendered three ways: clean (seeded, so a list has a row and its buttons),
//! with field errors (empty and validated, so every required field is
//! invalid), and with form-level errors (seeded and valid, so the validator
//! runs, plus a button whose fn has the wrong arity, which is what puts the
//! button-problems list on the page).
//!
//! The goldens live in `tests/snapshots/`, one tag per line so a change reads
//! as a diff. To regenerate after an INTENTIONAL change:
//!
//! ```text
//! SNAPSHOT=overwrite cargo test -p formoxus --test suite snapshot
//! ```
//!
//! then read the diff before committing, as with the trybuild goldens.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::{FormSpec, ValidationError, empty_form, form, form_for, use_form, using_fns};
use googletest::prelude::*;

use super::render_to_html;

const SIZES: &[(&str, &str)] = &[("s", "Small"), ("l", "Large")];
const STATES: &[(&str, &str)] = &[("AL", "Alabama"), ("AK", "Alaska")];

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Contact {
    Email,
    Phone,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    city: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Pet {
    name: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one field per bool widget is the point of the model"
)]
struct Everything {
    // Every `<input>` widget but the checkbox, each required, so each has a
    // marker; and one optional, which has none.
    text: String,
    password: String,
    hidden: String,
    number: i32,
    email: String,
    tel: String,
    url: String,
    search: String,
    color: String,
    date: String,
    time: String,
    datetime_local: String,
    month: String,
    week: String,
    nickname: Option<String>,
    // Checkboxes: plain, and one that must be ticked (it gets the marker).
    subscribed: bool,
    agreed: bool,
    // The other widgets, and the two bool-only defaults.
    bio: String,
    state: String,
    size: String,
    flag: bool,
    yes: bool,
    maybe: Option<bool>,
    // The three groups.
    contact: Contact,
    address: Address,
    pets: Vec<Pet>,
}

fn spec() -> FormSpec<Everything> {
    form! {
        Everything {
            title: "Everything",
            password => { widget: password },
            hidden => { widget: hidden },
            number => { widget: number },
            email => { widget: email },
            tel => { widget: tel },
            url => { widget: url },
            search => { widget: search },
            color => { widget: color },
            date => { widget: date },
            time => { widget: time },
            datetime_local => { widget: datetime_local },
            month => { widget: month },
            week => { widget: week },
            agreed => { required_true },
            bio => { widget: textarea },
            state => { widget: select { choices: STATES } },
            size => { widget: radio_group { choices: SIZES } },
            flag => { widget: radio_group },
            yes => { widget: select },
            buttons: {
                save: { type: submit },
                undo: { type: reset },
                back: { type: cancel },
                wipe: { type: destructive },
                ping: { type: action },
            }
        }
    }
}

fn seeded() -> Everything {
    Everything {
        text: "t".into(),
        password: "p".into(),
        hidden: "h".into(),
        number: 7,
        email: "a@b.c".into(),
        tel: "555".into(),
        url: "https://example.com".into(),
        search: "q".into(),
        color: "#ff0000".into(),
        date: "2026-10-14".into(),
        time: "09:30".into(),
        datetime_local: "2026-10-14T09:30".into(),
        month: "2026-10".into(),
        week: "2026-W42".into(),
        nickname: None,
        subscribed: false,
        agreed: true,
        bio: "b".into(),
        state: "AK".into(),
        size: "l".into(),
        flag: true,
        yes: false,
        maybe: None,
        contact: Contact::Phone,
        address: Address { city: "c".into() },
        pets: vec![Pet { name: "Rex".into() }],
    }
}

fn the_whole_form_is_wrong(_: &Everything) -> Vec<ValidationError<Everything>> {
    vec![ValidationError::form("The whole form is wrong.")]
}

#[component]
fn Clean() -> Element {
    let form = use_form(|| form_for(&seeded(), spec()));
    render_with_buttons(form)
}

#[component]
fn FieldErrors() -> Element {
    let mut state = empty_form(spec());
    // Validating BEFORE mounting renders the errors without an interaction.
    let _ = state.validate();
    let form = use_form(|| state);
    render_with_buttons(form)
}

#[component]
fn FormError() -> Element {
    let mut state = form_for(&seeded(), spec().with_validator(the_whole_form_is_wrong));
    let _ = state.validate();
    let form = use_form(|| state);
    // `action` validates by default, so its fn must take the model. This one
    // does not, ON PURPOSE: it is what renders the button-problems list.
    form.render(using_fns! {
        save: |_m| async move {},
        undo: || async move {},
        back: || async move {},
        wipe: || async move {},
        ping: || async move {},
    })
}

fn render_with_buttons(form: formoxus::Form<Everything>) -> Element {
    form.render(using_fns! {
        save: |_m| async move {},
        undo: || async move {},
        back: || async move {},
        wipe: || async move {},
        ping: |_m| async move {},
    })
}

#[gtest]
fn a_clean_form_matches_its_snapshot() {
    check("clean", &render_to_html(Clean));
}

#[gtest]
fn a_form_with_field_errors_matches_its_snapshot() {
    check("field_errors", &render_to_html(FieldErrors));
}

#[gtest]
fn a_form_with_a_form_error_matches_its_snapshot() {
    check("form_error", &render_to_html(FormError));
}

/// Compares `html` with `tests/snapshots/{name}.html`, or writes it there
/// when `SNAPSHOT=overwrite`.
fn check(name: &str, html: &str) {
    let actual = normalized(html);
    let path = format!("{}/tests/snapshots/{name}.html", env!("CARGO_MANIFEST_DIR"));
    if std::env::var("SNAPSHOT").as_deref() == Ok("overwrite") {
        std::fs::write(&path, &actual).expect("write the snapshot");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("no snapshot at {path}; run with SNAPSHOT=overwrite to create it")
    });
    assert_that!(actual, eq(&expected));
}

/// The form's random id replaced by `FORMID`, so the snapshot is the same on
/// every run, and a newline before every tag, so a change is a line diff.
///
/// The id is read off the first `id="…"` that starts with one: 8 characters
/// and a `-`, as `RenderCtx::id` builds them. Replacing that exact string,
/// rather than anything id-shaped, cannot touch a class or a value.
fn normalized(html: &str) -> String {
    let form_id = html.split("id=\"").skip(1).find_map(|rest| {
        let candidate = rest.get(..9)?;
        let (id, dash) = candidate.split_at(8);
        let mut chars = id.chars();
        let looks_like_one = dash == "-"
            && chars.next().is_some_and(|c| c.is_ascii_alphabetic())
            && chars.all(|c| c.is_ascii_alphanumeric());
        looks_like_one.then(|| id.to_string())
    });
    let html = match form_id {
        Some(id) => html.replace(&id, "FORMID"),
        None => html.to_string(),
    };
    format!("{}\n", html.replace('<', "\n<").trim_start())
}
