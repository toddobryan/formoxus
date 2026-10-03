//! Author attributes (C6): `FieldSpec::attrs`, through `FormField::render` and
//! `ScalarWidget`, onto the element each widget spreads them on.
//!
//! These go through `FormSpec::with_attrs` rather than `form!`, so they pin
//! the runtime path on its own. The conversion itself is unit-tested beside
//! `FieldAttrs::to_attributes` in `fields.rs`. Which element is valid for which
//! attribute is in `.claude/memory/html_attributes_reference.md`.
//!
//! **An author attribute is always text**, so it renders quoted (`rows="4"`),
//! unlike a constraint's `maxlength=10`.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::fields::{Constraints, FieldAttrs};
use formoxus::members::ValuesByPath;
use formoxus::prelude::*;
use formoxus::widgets::{InputType, WidgetType};
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Person {
    name: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Terms {
    agreed: bool,
}

const STATES: &[(&str, &str)] = &[("AL", "Alabama"), ("AK", "Alaska")];

/// Renders `empty_form(spec)` to HTML. A macro rather than a fn, because
/// `render_to_html` takes a `fn() -> Element`, which cannot capture a spec.
/// `$spec` must only name items, never a local.
macro_rules! render {
    ($spec:expr) => {{
        #[component]
        fn App() -> Element {
            let form = use_form(|| empty_form($spec));
            form.render_fragment()
        }
        super::render_to_html(App)
    }};
}

fn attrs(pairs: &[(&'static str, &str)]) -> FieldAttrs {
    let mut out = FieldAttrs::default();
    for (name, value) in pairs {
        out.insert(name, (*value).to_string());
    }
    out
}

/// The opening tag of the first `<{element}` in `html`, up to its `>`.
fn tag<'h>(html: &'h str, element: &str) -> &'h str {
    let start = html
        .find(&format!("<{element}"))
        .unwrap_or_else(|| panic!("no <{element}> in:\n{html}"));
    let rest = &html[start..];
    &rest[..=rest.find('>').expect("an opening tag ends with >")]
}

// ── Each widget's element ────────────────────────────────────────────────

#[gtest]
fn author_attributes_reach_the_input() {
    let html = render!(FormSpec::<Person>::new().with_attrs(
        "name",
        attrs(&[("placeholder", "Ada"), ("autocomplete", "name")])
    ));
    let input = tag(&html, "input");
    expect_that!(input, contains_substring(r#"placeholder="Ada""#));
    expect_that!(input, contains_substring(r#"autocomplete="name""#));
}

/// The `From` impl is the other way in, and the one a literal table uses.
#[gtest]
fn an_array_of_pairs_converts_into_author_attributes() {
    let html = render!(FormSpec::<Person>::new().with_attrs(
        "name",
        FieldAttrs::from([("placeholder", "Ada".to_string())])
    ));
    expect_that!(
        tag(&html, "input"),
        contains_substring(r#"placeholder="Ada""#)
    );
}

#[gtest]
fn author_attributes_reach_the_textarea() {
    let html = render!(
        FormSpec::<Person>::new()
            .with_custom_widget("name", WidgetType::Textarea)
            .with_attrs("name", attrs(&[("rows", "4")]))
    );
    expect_that!(tag(&html, "textarea"), contains_substring(r#"rows="4""#));
}

#[gtest]
fn author_attributes_reach_the_checkbox() {
    let html =
        render!(FormSpec::<Terms>::new().with_attrs("agreed", attrs(&[("data-terms", "v2")])));
    let input = tag(&html, "input");
    expect_that!(input, contains_substring(r#"type="checkbox""#));
    expect_that!(input, contains_substring(r#"data-terms="v2""#));
}

#[gtest]
fn author_attributes_reach_the_select() {
    let html = render!(
        FormSpec::<Person>::new()
            .with_custom_widget("name", WidgetType::Select)
            .with_choices("name", STATES)
            .with_attrs("name", attrs(&[("autocomplete", "address-level1")]))
    );
    expect_that!(
        tag(&html, "select"),
        contains_substring(r#"autocomplete="address-level1""#)
    );
}

/// **On a radio group they land on the `<fieldset>`, not on the radios.**
/// That is where `RadioGroup` spreads, and it is the thing `class:` has to
/// decide about. If this changes, it should change on purpose.
#[gtest]
fn on_a_radio_group_author_attributes_land_on_the_fieldset() {
    let html = render!(
        FormSpec::<Person>::new()
            .with_custom_widget("name", WidgetType::RadioGroup)
            .with_choices("name", STATES)
            .with_attrs("name", attrs(&[("data-group", "states")]))
    );
    expect_that!(
        tag(&html, "fieldset"),
        contains_substring(r#"data-group="states""#)
    );
    expect_that!(html.matches("data-group").count(), eq(1));
}

/// A hidden input has no spread at all, so author attributes go nowhere.
/// Pinned so that a `form!` check refusing them has something to point at.
#[gtest]
fn a_hidden_input_gets_no_author_attributes() {
    let html = render!(
        FormSpec::<Person>::new()
            .with_custom_widget("name", WidgetType::Input(InputType::Hidden))
            .with_attrs("name", attrs(&[("data-x", "1")]))
    );
    expect_that!(html, not(contains_substring("data-x")));
}

// ── Against formoxus's own attributes ────────────────────────────────────

/// **An author attribute overrides a constraint attribute of the same name.**
/// `ScalarWidget` merges constraint attributes first and caller attributes
/// last. So a raw `maxlength` replaces `max_length`'s, and the browser then
/// allows 99 characters while `check` still rejects more than 10.
///
/// This pins the hazard rather than endorsing it. `form!` should refuse an
/// attribute that has a constraint key, and this is what that refusal
/// prevents. Only a hand-built `FormSpec` can still reach it.
#[gtest]
fn an_author_attribute_overrides_a_constraint_attribute() {
    let html = render!(
        FormSpec::<Person>::new()
            .with_constraints(
                "name",
                Constraints {
                    max_length: Some(10),
                    ..Default::default()
                }
            )
            .with_attrs("name", attrs(&[("maxlength", "99")]))
    );
    let input = tag(&html, "input");
    expect_that!(input, contains_substring(r#"maxlength="99""#));
    expect_that!(input, not(contains_substring("maxlength=10")));
}

/// **Author attributes are presentation only: the server never reads them.**
/// A raw `maxlength` limits nothing in `validate`, which is why the constraint
/// keys exist and why a raw attribute must not stand in for one.
#[gtest]
fn author_attributes_do_not_constrain_the_value() {
    let spec = FormSpec::<Person>::new().with_attrs("name", attrs(&[("maxlength", "3")]));
    let values: ValuesByPath = [("name".to_string(), "Ada Lovelace".to_string())]
        .into_iter()
        .collect();
    let submission = Submission::accept(spec, &values)
        .expect("a raw maxlength is not a constraint, so nothing rejects the value");
    expect_that!(
        submission.model(),
        eq(&Person {
            name: "Ada Lovelace".to_string()
        })
    );
}
