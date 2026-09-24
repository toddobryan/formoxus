//! Buttons on the reflection path: declared in `form!`, handled at `render`.
//!
//! Written from a consumer's position (see `tests/reflect.rs`) because
//! `form!` and `using_fns!` both expand to `::formoxus::…` paths, which cannot
//! resolve inside formoxus itself.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::{Fns, FormSpec, empty_form, use_form};
use formoxus::{form, using_fns};
use googletest::prelude::*;

use super::render_to_html;

#[derive(Clone, Debug, PartialEq, Facet)]
struct FakeFormWithButtons {
    pub some_data: String,
}

fn fake_form() -> FormSpec<FakeFormWithButtons> {
    form!(
        FakeFormWithButtons {
            some_data => {
                widget: text,
            },
            buttons: {
                delete: { type: destructive, text: "Drop" },
                reload: { type: reset },
                update: { type: submit, text: "Save to Db"},
            }
        }
    )
}

/// Every button supplied, each with the arity its type implies: `destructive`
/// and `reset` default to running unconditionally, `submit` to validating
/// first.
#[component]
fn FakeForm() -> Element {
    let form = use_form(|| empty_form(fake_form()));
    form.render(using_fns! {
        delete: || async move {},
        reload: || async move {},
        update: |_m| async move {},
    })
}

#[gtest]
fn every_declared_button_is_rendered_in_order() {
    let html = render_to_html(FakeForm);
    let (drop_at, reload_at, save_at) = (
        html.find("Drop").expect("the destructive button"),
        html.find("Reload").expect("the reset button"),
        html.find("Save to Db").expect("the submit button"),
    );
    // Source order is display order — the one layout fact the author states.
    expect_that!(drop_at, lt(reload_at));
    expect_that!(reload_at, lt(save_at));
}

#[gtest]
fn a_button_without_text_falls_back_to_its_name_in_title_case() {
    // `reload` declares no `text`, so it reads as its own name — run through
    // the same title casing a field label gets, so a two-word name like
    // `sign_in` does not render with its underscore showing.
    expect_that!(render_to_html(FakeForm), contains_substring(">Reload<"));
}

#[gtest]
fn each_type_picks_its_html_type_and_class() {
    let html = render_to_html(FakeForm);
    // `destructive` is a plain `button`: letting it submit would save the form
    // it is meant to discard.
    expect_that!(html, contains_substring(r#"type="button" class="danger""#));
    expect_that!(
        html,
        contains_substring(r#"type="reset" class="outline danger""#)
    );
    expect_that!(html, contains_substring(r#"type="submit" class="primary""#));
}

#[gtest]
fn the_row_is_wrapped_for_the_existing_style_rule() {
    // The hook a consumer's stylesheet lays the button row out with — see the
    // wrapper in `Form::render_buttons`.
    expect_that!(
        render_to_html(FakeForm),
        contains_substring(r#"class="formoxus-buttons""#)
    );
}

#[gtest]
fn a_supplied_button_is_enabled() {
    expect_that!(
        render_to_html(FakeForm),
        not(contains_substring("disabled"))
    );
}

// ── When the handlers and the spec disagree ──────────────────────────────

#[component]
fn NoFnsAtAll() -> Element {
    let form = use_form(|| empty_form(fake_form()));
    form.render(Fns::new())
}

#[gtest]
fn a_button_with_no_fn_is_disabled_and_says_so() {
    let html = render_to_html(NoFnsAtAll);
    // Rendered, not omitted: a missing button hides the mistake, and a live
    // `submit` with nothing behind it would reload the page.
    expect_that!(html, contains_substring("Drop"));
    expect_that!(html, contains_substring("disabled"));
    expect_that!(
        html,
        contains_substring("button `delete` has no fn supplied")
    );
    expect_that!(
        html,
        contains_substring("button `update` has no fn supplied")
    );
}

#[component]
fn UnknownFn() -> Element {
    let form = use_form(|| empty_form(fake_form()));
    form.render(using_fns! {
        delete: || async move {},
        reload: || async move {},
        update: |_m| async move {},
        delete_all: || async move {},
    })
}

#[gtest]
fn a_fn_for_no_such_button_is_reported() {
    // The typo case, and the half-done-rename case. Nothing would ever call it.
    expect_that!(
        render_to_html(UnknownFn),
        contains_substring("`delete_all` is not a button this form declares")
    );
}

#[component]
fn WrongArity() -> Element {
    let form = use_form(|| empty_form(fake_form()));
    form.render(using_fns! {
        delete: || async move {},
        reload: || async move {},
        // `update` is a submit button, so it validates first and its fn must be
        // able to receive the model.
        update: || async move {},
    })
}

#[gtest]
fn a_fn_whose_arity_contradicts_the_spec_is_reported() {
    expect_that!(
        render_to_html(WrongArity),
        contains_substring("button `update` validates its model")
    );
}

// ── A form that declares none ────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Facet)]
struct Plain {
    note: String,
}

#[component]
fn NoButtons() -> Element {
    let form = use_form(|| empty_form(form! { Plain { note => { label: "Note" } } }));
    form.render(Fns::new())
}

#[gtest]
fn a_form_with_no_buttons_renders_no_row() {
    // The existing views hand-write their own buttons; nothing may appear
    // underneath them.
    let html = render_to_html(NoButtons);
    expect_that!(html, not(contains_substring("formoxus-buttons")));
    expect_that!(html, not(contains_substring("<button")));
}
