//! `live_validation: on | off`, and the app-wide `Formoxus::with_live_validation`
//! it overrides (`ERRORS_PLAN.md` box 7).
//!
//! Only the knob. What a live check does once it runs (which leaves it checks,
//! what it writes) is 8c's, and what a user sees in a browser is 8d's. Every
//! test here takes the same two steps, type then leave, because that is the
//! smallest thing that checks a field when the knob is on: a keystroke alone
//! does not, until the field shows an error.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::{Formoxus, empty_form, form, form_for, provide_defaults, use_form, using_fns};
use googletest::prelude::*;

use super::Harness;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Contact {
    name: String,
}

const REQUIRED: &str = "This field is required.";

/// Type a blank name, then leave the field.
fn type_blank_and_leave(app: fn() -> Element) -> String {
    let mut harness = Harness::mount(app);
    harness.fire("input", harness.only_listener("input"), "");
    harness.blur(harness.only_listener("blur"));
    harness.html()
}

// ── The form's own key ───────────────────────────────────────────────────

#[component]
fn Silent() -> Element {
    let form = use_form(|| empty_form(form! { Contact {} }));
    form.render(using_fns! {})
}

/// **On by default:** leaving a field typed into checks it.
#[gtest]
fn by_default_leaving_a_field_typed_into_checks_it() {
    let html = type_blank_and_leave(Silent);
    expect_that!(html, contains_substring(REQUIRED));
    expect_that!(html, contains_substring(r#"aria-invalid="true""#));
}

#[component]
fn On() -> Element {
    let form = use_form(|| empty_form(form! { Contact { live_validation: on } }));
    form.render(using_fns! {})
}

#[gtest]
fn on_checks_a_field_left_after_typing() {
    expect_that!(type_blank_and_leave(On), contains_substring(REQUIRED));
}

#[component]
fn Off() -> Element {
    let form = use_form(|| empty_form(form! { Contact { live_validation: off } }));
    form.render(using_fns! {})
}

/// **Off leaves every check to submit:** the same steps show nothing.
#[gtest]
fn off_leaves_a_field_unchecked_until_submit() {
    let html = type_blank_and_leave(Off);
    expect_that!(html, not(contains_substring(REQUIRED)));
    expect_that!(html, not(contains_substring("aria-invalid")));
}

// ── The app's default ────────────────────────────────────────────────────

#[component]
fn AppOffSilentForm() -> Element {
    provide_defaults(Formoxus::new().with_live_validation(false));
    let form = use_form(|| empty_form(form! { Contact {} }));
    form.render(using_fns! {})
}

#[gtest]
fn an_app_default_reaches_a_silent_form() {
    expect_that!(
        type_blank_and_leave(AppOffSilentForm),
        not(contains_substring(REQUIRED))
    );
}

#[component]
fn AppOffFormOn() -> Element {
    provide_defaults(Formoxus::new().with_live_validation(false));
    let form = use_form(|| empty_form(form! { Contact { live_validation: on } }));
    form.render(using_fns! {})
}

#[gtest]
fn a_form_overrides_the_app_default() {
    expect_that!(
        type_blank_and_leave(AppOffFormOn),
        contains_substring(REQUIRED)
    );
}

// ── Off still keeps "false" honest (6b step 2) ───────────────────────────

#[component]
fn OffCheckedClean() -> Element {
    let form = use_form(|| {
        form_for(
            &Contact {
                name: "Ada".to_string(),
            },
            form! { Contact { live_validation: off } },
        )
    });
    use_hook(|| form.validate());
    form.render(using_fns! {})
}

/// **A clean result goes stale on a write, knob or no knob.** With it off,
/// nothing re-checks the field, so `aria-invalid="false"` would otherwise go
/// on claiming a value it never saw was valid.
#[gtest]
fn with_it_off_a_write_still_drops_a_clean_fields_false() {
    let mut app = Harness::mount(OffCheckedClean);
    expect_that!(app.html(), contains_substring(r#"aria-invalid="false""#));
    app.fire("input", app.only_listener("input"), "");
    let html = app.html();
    expect_that!(html, not(contains_substring("aria-invalid")));
    expect_that!(html, not(contains_substring(REQUIRED)));
}
