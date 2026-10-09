//! Author attributes (C6): written in `form!`, through `FormField::render` and
//! `ScalarWidget`, onto the element each widget puts them on.
//!
//! These go through `form!`, the way an author writes them. A few stay on
//! `FormSpec::with_attrs`, each saying why: what `form!` refuses (a quoted key
//! the table knows) or cannot yet express (`custom(…)` drops attributes), and
//! the `From` impl that only a hand-built spec uses. The conversion itself is
//! unit-tested beside `FieldAttrs::to_attributes` in `fields.rs`. Which element
//! is valid for which attribute is in
//! `.claude/memory/html_attributes_reference.md`.
//!
//! **A quoted attribute is always text**, so it renders quoted
//! (`data-x="1"`). A table attribute renders by its row's type: `rows: 4` is
//! an `Int`, so `rows=4`, like a constraint's `maxlength=10`.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::{Attr, AttrKey, AttrValue};
use formoxus::fields::FieldAttrs;
use formoxus::members::ValuesByPath;
use formoxus::prelude::*;
use formoxus::widgets::WidgetType;
use googletest::prelude::*;

use super::models::Shape;

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

/// Quoted (`NonStd`) attributes, the pass-through kind, for the tests that
/// stay on a hand-built spec. Unlike `form!`, a hand-built spec can use a name
/// the table knows (`"maxlength"`), which is what those tests are about.
fn attrs<const N: usize>(pairs: [(&'static str, &str); N]) -> FieldAttrs {
    FieldAttrs::from(
        pairs.map(|(name, value)| (AttrKey::NonStd(name), AttrValue::String(value.to_string()))),
    )
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
    let html = render!(form! {
        Person { name => { placeholder: "Ada", autocomplete: "name" } }
    });
    let input = tag(&html, "input");
    expect_that!(input, contains_substring(r#"placeholder="Ada""#));
    expect_that!(input, contains_substring(r#"autocomplete="name""#));
}

/// The `From` impl is the other way in, and the one a literal table uses.
/// Hand-built on purpose: `form!` never goes through it.
#[gtest]
fn an_array_of_pairs_converts_into_author_attributes() {
    let html = render!(FormSpec::<Person>::new().with_attrs(
        "name",
        FieldAttrs::from([(
            AttrKey::NonStd("placeholder"),
            AttrValue::String("Ada".to_string())
        )])
    ));
    expect_that!(
        tag(&html, "input"),
        contains_substring(r#"placeholder="Ada""#)
    );
}

#[gtest]
fn author_attributes_reach_the_textarea() {
    let html = render!(form! { Person { name => { widget: textarea, rows: 4 } } });
    expect_that!(tag(&html, "textarea"), contains_substring("rows=4"));
}

#[gtest]
fn author_attributes_reach_the_checkbox() {
    let html = render!(form! { Terms { agreed => { "data-terms": "v2" } } });
    let input = tag(&html, "input");
    expect_that!(input, contains_substring(r#"type="checkbox""#));
    expect_that!(input, contains_substring(r#"data-terms="v2""#));
}

#[gtest]
fn author_attributes_reach_the_select() {
    let html = render!(form! {
        Person {
            name => { widget: select { choices: STATES }, autocomplete: "address-level1" },
        }
    });
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
    let html = render!(form! {
        Person { name => { widget: radio_group { choices: STATES }, "data-group": "states" } }
    });
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
    let html = render!(form! { Person { name => { widget: hidden, "data-x": "1" } } });
    expect_that!(html, not(contains_substring("data-x")));
}

/// A custom widget that spreads whatever it is handed onto one `<input>`.
///
/// Hand-built rather than `custom(…)` in `form!`, because the macro's closure
/// passes only `values` and `props` today: attributes reaching a `form!`
/// custom widget is step 4 of `ATTRIBUTES_PLAN.md`. This pins the runtime
/// half, `ScalarWidget`'s `Custom` arm, on its own.
fn spreading_widget() -> WidgetType {
    WidgetType::Custom {
        name: "Spreading",
        render: |p| {
            let attrs = p.attrs;
            rsx! { input { class: "spreading", ..attrs } }
        },
    }
}

/// **A custom widget gets the field's attributes, formoxus's own included.**
/// Not only the author's: a constraint like `maxlength` is what the browser
/// enforces, so a custom widget that never saw it would accept what `check`
/// later rejects. The `Custom` arm once passed only the caller's extras, and
/// nothing failed.
#[gtest]
fn a_custom_widget_receives_the_fields_attributes() {
    let html = render!(
        FormSpec::<Person>::new()
            .with_custom_widget("name", spreading_widget())
            .with_attrs(
                "name",
                FieldAttrs::from([
                    (AttrKey::Std(Attr::MaxLength), AttrValue::Int(10)),
                    (
                        AttrKey::NonStd("data-x"),
                        AttrValue::String("1".to_string())
                    ),
                ])
            )
    );
    let input = tag(&html, "input");
    expect_that!(input, contains_substring(r#"class="spreading""#));
    expect_that!(input, contains_substring("maxlength=10"));
    expect_that!(input, contains_substring(r#"data-x="1""#));
}

// ── class and class_plus ─────────────────────────────────────────────────
//
// Each widget writes `class` as a literal, resolved by `FieldAttrs::class`
// against its own base, and `to_attributes` leaves `Class`/`ClassPlus` out of
// the spread. Before that, an author's `class` was spread as a SECOND `class`
// attribute beside the widget's, and a browser keeps only the first. So every
// test here reads ALL the `class` attributes on the tag, not just one.

/// An enum field, for the variant `<select>` a `VariantSet` renders.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Drawing {
    shape: Shape,
}

/// The value of every `class="…"` in `tag`, in order. More than one is the bug
/// these tests exist for.
fn classes_of(tag: &str) -> Vec<String> {
    tag.split(r#" class=""#)
        .skip(1)
        .map(|rest| rest[..rest.find('"').expect("a class value ends with \"")].to_string())
        .collect()
}

/// `class` replaces formoxus's classes outright, and `class_plus` appends to
/// them with a space; either way the tag has exactly one `class`.
#[gtest]
fn class_and_class_plus_on_the_input() {
    let replaced = render!(form! { Person { name => { class: [wide, dark] } } });
    expect_that!(
        classes_of(tag(&replaced, "input")),
        elements_are![eq("wide dark")]
    );

    let appended = render!(form! { Person { name => { class_plus: [wide] } } });
    expect_that!(
        classes_of(tag(&appended, "input")),
        elements_are![eq("fx-control fx-input wide")]
    );
}

#[gtest]
fn class_and_class_plus_on_the_textarea() {
    let replaced = render!(form! { Person { name => { widget: textarea, class: [wide] } } });
    expect_that!(
        classes_of(tag(&replaced, "textarea")),
        elements_are![eq("wide")]
    );

    let appended = render!(form! { Person { name => { widget: textarea, class_plus: [wide] } } });
    expect_that!(
        classes_of(tag(&appended, "textarea")),
        elements_are![eq("fx-control fx-textarea wide")]
    );
}

#[gtest]
fn class_and_class_plus_on_the_checkbox() {
    let replaced = render!(form! { Terms { agreed => { class: [big] } } });
    expect_that!(
        classes_of(tag(&replaced, "input")),
        elements_are![eq("big")]
    );

    let appended = render!(form! { Terms { agreed => { class_plus: [big] } } });
    expect_that!(
        classes_of(tag(&appended, "input")),
        elements_are![eq("fx-control fx-checkbox big")]
    );
}

#[gtest]
fn class_and_class_plus_on_the_select() {
    let replaced = render!(form! {
        Person { name => { widget: select { choices: STATES }, class: [wide] } }
    });
    expect_that!(
        classes_of(tag(&replaced, "select")),
        elements_are![eq("wide")]
    );

    let appended = render!(form! {
        Person { name => { widget: select { choices: STATES }, class_plus: [wide] } }
    });
    expect_that!(
        classes_of(tag(&appended, "select")),
        elements_are![eq("fx-control fx-select wide")]
    );
}

/// **An enum field's attributes reach its variant `<select>`.** `VariantSet`
/// took no attributes at all until step 4.
///
/// Hand-built, because `form!` refuses `class` on an enum field at compile
/// time: the table's `for:` column names only String, number and bool fields,
/// and an enum is none of those. A quoted key is not type-checked, so it gets
/// through `form!` (the next test).
#[gtest]
fn class_and_class_plus_on_the_variant_select() {
    let replaced = render!(FormSpec::<Drawing>::new().with_attrs(
        "shape",
        FieldAttrs::from([(AttrKey::Std(Attr::Class), AttrValue::List(&["wide"]))])
    ));
    expect_that!(
        classes_of(tag(&replaced, "select")),
        elements_are![eq("wide")]
    );

    let appended = render!(FormSpec::<Drawing>::new().with_attrs(
        "shape",
        FieldAttrs::from([(AttrKey::Std(Attr::ClassPlus), AttrValue::List(&["wide"]))])
    ));
    expect_that!(
        classes_of(tag(&appended, "select")),
        elements_are![eq("fx-control fx-select wide")]
    );
}

/// A quoted key on an enum field, through `form!`, reaches the variant
/// `<select>`.
#[gtest]
fn a_quoted_attribute_on_an_enum_field_reaches_the_variant_select() {
    let html = render!(form! { Drawing { shape => { "data-x": "1" } } });
    expect_that!(tag(&html, "select"), contains_substring(r#"data-x="1""#));
}

/// **On a radio group an author's `class` goes nowhere, for now.** Where it
/// belongs (the `<fieldset>`, each radio, or a `group_class`/`input_class`
/// pair) is on hold; meanwhile `RadioGroup` never calls `FieldAttrs::class`,
/// and the spread no longer carries it. Pinned so the decision, when it comes,
/// changes this on purpose.
#[gtest]
fn on_a_radio_group_an_author_class_goes_nowhere_yet() {
    let html = render!(form! {
        Person { name => { widget: radio_group { choices: STATES }, class: [wide] } }
    });
    expect_that!(html, not(contains_substring("wide")));
}

// ── style and style_plus ─────────────────────────────────────────────────
//
// Unlike `class`, `style` rides the spread: formoxus writes no style of its
// own, so there is no literal to resolve it against and no base to append to.
// `form!` hands over each declaration whole (`"font-size: 20px"`); joining
// them is `to_attributes`' job.

/// **Declarations are separated by `;`.** A comma is not a CSS separator:
/// `color: red, font-size: 20px` is ONE declaration with an invalid value,
/// which a browser drops whole, so neither style applies.
#[gtest]
fn style_joins_its_declarations_with_semicolons() {
    let html = render!(form! { Person { name => { style: { color: red, font_size: 20px } } } });
    let input = tag(&html, "input");
    expect_that!(
        input,
        contains_substring(r#"style="color: red; font-size: 20px""#)
    );
    expect_that!(input.matches(" style=").count(), eq(1));
}

/// One declaration gets no separator at all, so nothing trails it.
#[gtest]
fn a_single_style_declaration_stands_alone() {
    let html = render!(form! { Person { name => { style: { color: red } } } });
    expect_that!(
        tag(&html, "input"),
        contains_substring(r#"style="color: red""#)
    );
}

/// **`style_plus` acts exactly like `style`, for now.** It means "append to
/// formoxus's", and formoxus emits none. The day a widget writes a style of its
/// own, this changes, and `style_plus` wants the `class` treatment.
#[gtest]
fn style_plus_renders_like_style_while_formoxus_has_none() {
    let html =
        render!(form! { Person { name => { style_plus: { color: red, font_size: 20px } } } });
    let input = tag(&html, "input");
    expect_that!(
        input,
        contains_substring(r#"style="color: red; font-size: 20px""#)
    );
    expect_that!(input.matches(" style=").count(), eq(1));
}

/// **On a radio group, `style` lands on the `<fieldset>`**, because it rides
/// the spread and that is where `RadioGroup` spreads. Unlike `class`, which
/// goes nowhere there (above). If the `RadioGroup` hold settles on per-radio
/// placement, this should change with it.
#[gtest]
fn on_a_radio_group_style_lands_on_the_fieldset() {
    let html = render!(form! {
        Person { name => { widget: radio_group { choices: STATES }, style: { gap: 4px } } }
    });
    expect_that!(
        tag(&html, "fieldset"),
        contains_substring(r#"style="gap: 4px""#)
    );
    expect_that!(html.matches("gap: 4px").count(), eq(1));
}

// ── Against formoxus's own attributes ────────────────────────────────────

/// **A quoted attribute overrides a table attribute of the same HTML name,
/// when it comes later.** Both are in the one map under different keys, so
/// both are emitted, and `ScalarWidget`'s merge, keyed by HTML name, keeps the
/// later. So a raw `maxlength` replaces `max_length`'s, and the browser then
/// allows 99 characters while `check` still rejects more than 10.
///
/// This pins the hazard rather than endorsing it. `form!` refuses a quoted
/// key the table knows ("write `max_length:`"), and this is what that refusal
/// prevents. Only a hand-built `FormSpec` can still reach it.
#[gtest]
fn an_author_attribute_overrides_a_constraint_attribute() {
    let html = render!(FormSpec::<Person>::new().with_attrs(
        "name",
        FieldAttrs::from([
            (AttrKey::Std(Attr::MaxLength), AttrValue::Int(10)),
            (
                AttrKey::NonStd("maxlength"),
                AttrValue::String("99".to_string())
            ),
        ])
    ));
    let input = tag(&html, "input");
    expect_that!(input, contains_substring(r#"maxlength="99""#));
    expect_that!(input, not(contains_substring("maxlength=10")));
}

/// **Author attributes are presentation only: the server never reads them.**
/// A raw `maxlength` limits nothing in `validate`, which is why the constraint
/// keys exist and why a raw attribute must not stand in for one. Hand-built,
/// because `form!` refuses the quoted `"maxlength"` for exactly this reason.
#[gtest]
fn author_attributes_do_not_constrain_the_value() {
    let spec = FormSpec::<Person>::new().with_attrs("name", attrs([("maxlength", "3")]));
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
