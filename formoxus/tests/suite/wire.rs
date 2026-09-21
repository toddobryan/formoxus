//! The return leg: `path!`, `Form::apply_errors`, `to_wire` and `absorb`.
//!
//! These are written from a consumer's position for a reason beyond the usual
//! one: `path!` expands to an absolute `::formoxus::path::Path` and a witness
//! borrow, and neither resolves inside the library itself. A test here is the
//! only place the expansion is exercised as a caller meets it.

use std::collections::HashMap;

use dioxus::prelude::*;
use facet::Facet;
use formoxus::{
    FieldError, FormError, FormErrors, FormSpec, Submission, WireForm, empty_form, form, form_for,
    path, use_form, using_fns,
};
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Location {
    city: String,
    zip: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Contact {
    name: String,
    email: String,
    location: Location,
}

fn seeded() -> Contact {
    Contact {
        name: "Ada".into(),
        email: "ada@example.com".into(),
        location: Location {
            city: "London".into(),
            zip: "NW1".into(),
        },
    }
}

fn render(app: fn() -> Element) -> String {
    super::render_to_html(app)
}

fn errors_at(path: &str, message: &str) -> FormErrors {
    FormErrors {
        form: Vec::new(),
        fields: vec![(path.to_string(), vec![FieldError(message.to_string())])],
    }
}

// ── path! ────────────────────────────────────────────────────────────────

#[gtest]
fn a_path_carries_its_wire_spelling() {
    expect_that!(path!(Contact.email).as_str(), eq("email"));
    expect_that!(path!(Contact.location.zip).as_str(), eq("location.zip"));
}

/// The type parameter is what stops a path for one model reaching another
/// model's form. Nothing here asserts that at runtime — it is a compile-time
/// property, and this test exists to pin the spelling that carries it.
#[gtest]
fn a_path_is_typed_by_its_model() {
    let p: formoxus::Path<Contact> = path!(Contact.name);
    expect_that!(format!("{p}"), eq("name"));
}

/// `Path` is `Copy` and comparable even though `Contact` is neither `Copy` nor
/// `Eq` — the impls are hand-written precisely so no bound leaks onto `T`.
#[gtest]
fn a_path_copies_without_its_model_being_copy() {
    let a = path!(Contact.email);
    let b = a;
    expect_that!(a, eq(b));
}

// ── Form::apply_errors ───────────────────────────────────────────────────

/// The loop every caller used to write by hand, and the thing that had no
/// consumer at all before: `collect_errors` produced a `FormErrors` and nothing
/// in the crate took one back.
#[gtest]
fn applied_field_errors_render_where_the_widget_shows_them() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact {} }));
        form.apply_errors(&errors_at("email", "That address is already registered."))
            .expect("email is a field of this form");
        form.render_fragment()
    }
    expect_that!(
        render(App),
        contains_substring("That address is already registered.")
    );
}

#[gtest]
fn applied_form_errors_reach_the_form_level_list() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact {} }));
        form.apply_errors(&FormErrors {
            form: vec![FormError("Those credentials do not match.".into())],
            fields: Vec::new(),
        })
        .expect("a form-level error names no path");
        form.render(using_fns! {})
    }
    expect_that!(
        render(App),
        contains_substring("Those credentials do not match.")
    );
}

/// A nested path is as good as a flat one — `owns()` walks the tree.
#[gtest]
fn an_applied_error_finds_a_nested_leaf() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact {} }));
        form.apply_errors(&errors_at("location.zip", "No such ZIP code."))
            .expect("a qualified leaf is a legal target");
        form.render_fragment()
    }
    expect_that!(render(App), contains_substring("No such ZIP code."));
}

/// A path the two sides disagree about is a spec mismatch, not user input —
/// so it surfaces rather than being dropped.
#[gtest]
fn an_error_at_an_unknown_path_is_reported() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact {} }));
        let outcome = form.apply_errors(&errors_at("nope", "..."));
        rsx! { "{outcome.is_err()}" }
    }
    expect_that!(render(App), contains_substring("true"));
}

// ── to_wire / absorb ─────────────────────────────────────────────────────

#[gtest]
fn to_wire_carries_the_live_values() {
    #[component]
    fn App() -> Element {
        let contact = seeded();
        let form = use_form(move || form_for(&contact, form! { Contact {} }));
        formoxus::widgets::write_value("name", form.values(), "Grace".to_string());

        let wire = form.to_wire();
        rsx! { "{wire.values().get(\"name\").cloned().unwrap_or_default()}" }
    }
    expect_that!(render(App), contains_substring("Grace"));
}

/// The server changed a value and sent it back — the field shows the server's
/// version, not what was typed.
#[gtest]
fn absorb_replaces_values_the_server_normalized() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact {} }));
        formoxus::widgets::write_value("email", form.values(), "  ADA@Example.COM ".to_string());

        let normalized: HashMap<String, String> =
            [("email".to_string(), "ada@example.com".to_string())]
                .into_iter()
                .collect();
        form.absorb(WireForm::new(normalized, FormErrors::default()))
            .expect("email is a field of this form");

        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"value="ada@example.com""#));
    expect_that!(html, not(contains_substring("ADA@Example.COM")));
}

/// Values and errors arrive together, which is the whole point of one call.
#[gtest]
fn absorb_applies_values_and_errors_in_one_pass() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact {} }));
        let values: HashMap<String, String> = [("name".to_string(), "Ada".to_string())]
            .into_iter()
            .collect();
        form.absorb(WireForm::new(
            values,
            errors_at("email", "Already registered."),
        ))
        .expect("both halves name fields of this form");
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"value="Ada""#));
    expect_that!(html, contains_substring("Already registered."));
}

/// A rejection normally sends no values, because the client still holds what it
/// submitted. Absorbing one must not wipe the form.
#[gtest]
fn absorbing_errors_alone_leaves_the_values_alone() {
    #[component]
    fn App() -> Element {
        let contact = seeded();
        let form = use_form(move || form_for(&contact, form! { Contact {} }));
        form.absorb(WireForm::from_errors(errors_at(
            "email",
            "Already registered.",
        )))
        .expect("email is a field of this form");
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"value="Ada""#));
    expect_that!(html, contains_substring("Already registered."));
}

// ── The whole round trip ─────────────────────────────────────────────────

/// A server handler, start to finish, with no Dioxus runtime — the property
/// that makes this testable at all.
fn handle(wire: &WireForm<Contact>) -> WireForm<Contact> {
    let spec = || FormSpec::<Contact>::default();
    match Submission::accept(spec(), wire.values()) {
        Err(errors) => WireForm::from_errors(errors),
        Ok(sub) => {
            if sub.model().email.contains("@example.com") {
                return sub.reject_field(path!(Contact.email), "Not a real address.");
            }
            let mut model = sub.into_model();
            model.email = model.email.trim().to_lowercase();
            WireForm::from_model(&model, spec())
        }
    }
}

#[gtest]
fn a_rejected_round_trip_comes_back_with_the_verdict() {
    let wire = WireForm::<Contact>::new(
        [
            ("name", "Ada"),
            ("email", "ada@example.com"),
            ("location.city", "London"),
            ("location.zip", "NW1"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
        FormErrors::default(),
    );

    let back = handle(&wire);
    expect_that!(back.is_clean(), eq(false));
    expect_that!(
        back.errors()
            .fields
            .iter()
            .map(|(p, _)| p.clone())
            .collect::<Vec<_>>(),
        elements_are![eq("email")]
    );
}

#[gtest]
fn an_accepted_round_trip_comes_back_normalized() {
    let wire = WireForm::<Contact>::new(
        [
            ("name", "Ada"),
            ("email", "  ADA@Lovelace.ORG "),
            ("location.city", "London"),
            ("location.zip", "NW1"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect(),
        FormErrors::default(),
    );

    let back = handle(&wire);
    expect_that!(back.is_clean(), eq(true));
    expect_that!(back.values().get("email"), some(eq("ada@lovelace.org")));
    // Untouched fields ride along, because the leaves are regenerated whole.
    expect_that!(back.values().get("location.zip"), some(eq("NW1")));
}

/// `WireForm` is what actually crosses, so it has to survive serde in both
/// directions — including the `PhantomData`, which is why the derive carries
/// `#[serde(bound = "")]`.
#[gtest]
fn a_wire_form_round_trips_through_json() {
    let wire = WireForm::<Contact>::new(
        [("name".to_string(), "Ada".to_string())]
            .into_iter()
            .collect(),
        errors_at("email", "Already registered."),
    );

    let json = serde_json::to_string(&wire).expect("a wire form serializes");
    let back: WireForm<Contact> = serde_json::from_str(&json).expect("and comes back");

    expect_that!(back.values().get("name"), some(eq("Ada")));
    expect_that!(back.errors().fields.len(), eq(1));
}
