//! Constraints reaching the BROWSER, not just Rust.
//!
//! `Field::check` enforces every constraint server-side; these pin the
//! other half, that the same constraint is handed to the browser so it can
//! give instant feedback and block submit. The mapping itself is unit-tested
//! beside `FieldAttrs` in `field_attrs.rs`; this is the end-to-end view from `form!`.
//!
//! **Numeric attributes render UNQUOTED** (`maxlength=10`), because they are
//! `AttributeValue::Int`. A `pattern` is text and does get quotes.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;

use super::FORM_ID;
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Zip {
    code: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Counts {
    whole: i64,
    fraction: f64,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Bio {
    about: String,
}

fn render(app: fn() -> Element) -> String {
    super::render_to_html(app)
}

// ── Reaching the input ───────────────────────────────────────────────────

#[gtest]
fn each_text_constraint_reaches_the_input() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Zip { code => { min_length: 5, max_length: 10, pattern: r"\d+" } }
            })
        });
        form.render(using_fns! {})
    }
    let html = render(App);
    expect_that!(html, contains_substring("minlength=5"));
    expect_that!(html, contains_substring("maxlength=10"));
    expect_that!(html, contains_substring(r#"pattern="\d+""#));
}

/// **The pattern reaches the DOM unanchored.** HTML wraps a `pattern` as
/// `^(?:…)$` implicitly, and `Field::check` wraps it Rust-side to match —
/// so handing the browser an already-wrapped pattern would anchor it twice and
/// the two would stop agreeing. Agreement is the whole reason `regress` is a
/// dependency.
#[gtest]
fn a_pattern_reaches_the_dom_unanchored() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Zip { code => { pattern: r"\d{5}" } } }));
        form.render(using_fns! {})
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"pattern="\d{5}""#));
    expect_that!(html, not(contains_substring("^(?:")));
}

/// On a `number` input, which takes `min` and `max`.
#[gtest]
fn a_numeric_bound_reaches_a_number_input() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Counts {
                    whole => { widget: number, min: 1, max: 9 },
                    fraction => { widget: number, min: 0.5 },
                }
            })
        });
        form.render(using_fns! {})
    }
    let html = render(App);
    expect_that!(html, contains_substring("min=1"));
    expect_that!(html, contains_substring("max=9"));
    expect_that!(html, contains_substring("min=0.5"));
}

/// A number's default widget is a TEXT input (so `parse` can say what is
/// wrong with a half-typed `-`), which takes no `min` or `max` (#4): the
/// browser would ignore them, and the markup would be invalid. The bounds
/// still hold, through `check`.
#[gtest]
fn a_numeric_bound_stays_off_a_text_input_but_is_still_checked() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Counts { whole => { min: 1, max: 9 } } }));
        form.render(using_fns! {})
    }
    let html = render(App);
    expect_that!(
        html,
        contains_regex(r#"<input [^>]*type="text"[^>]*name="whole""#)
    );
    expect_that!(html, not(contains_substring("min=")));
    expect_that!(html, not(contains_substring("max=")));

    let mut form = empty_form(form! { Counts { whole => { min: 1, max: 9 } } });
    form.distribute_values(
        &[
            ("whole".to_string(), "12".to_string()),
            ("fraction".to_string(), "0.5".to_string()),
        ]
        .into_iter()
        .collect(),
    );
    expect_that!(form.validate(), none());
    expect_that!(form.collect_errors().fields.get("whole"), some(anything()));
}

/// **No `step`, deliberately.** `<input type="number">` has an implicit
/// `step=1` and would reject `0.5` — but formoxus never defaults a float to
/// `type="number"` (`Int`/`Float` default to `Text`), so the only way to meet
/// that is an explicit `widget: number`, as here, and then supplying `step`
/// is the author's job. See the note on `Field::constraint_attributes`.
#[gtest]
fn no_step_is_emitted_for_a_float() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! { Counts { fraction => { widget: number, min: 0.5, max: 9.5 } } })
        });
        form.render(using_fns! {})
    }
    let html = render(App);
    // Positive first, so the absence below cannot pass by rendering nothing.
    expect_that!(html, contains_substring("min=0.5"));
    expect_that!(html, contains_substring("max=9.5"));
    expect_that!(html, not(contains_substring("step")));
}

/// What a dev writes instead: `step` is a pass-through attribute, emitted as
/// written. `"any"` is what a float on `type="number"` usually wants.
#[gtest]
fn a_dev_step_of_any_reaches_the_number_input() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! { Counts { fraction => { widget: number, step: "any" } } })
        });
        form.render(using_fns! {})
    }
    expect_that!(render(App), contains_substring(r#"step="any""#));
}

#[gtest]
fn a_dev_numeric_step_reaches_the_number_input() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! { Counts { fraction => { widget: number, step: 0.25 } } })
        });
        form.render(using_fns! {})
    }
    expect_that!(render(App), contains_substring("step=0.25"));
}

/// A whole float literal is still a whole step: only a fractional one is
/// refused on an int field (`tests/ui/form_fractional_step_on_an_integer.rs`).
#[gtest]
fn an_int_field_takes_a_whole_float_step() {
    #[component]
    fn App() -> Element {
        let form =
            use_form(|| empty_form(form! { Counts { whole => { widget: number, step: 2.0 } } }));
        form.render(using_fns! {})
    }
    expect_that!(render(App), contains_substring("step=2"));
}

/// A hidden input is not user-editable and browsers do not validate it. It is
/// skipped by `Input`'s early return rather than by the map — that return
/// renders the input BARE (no wrapper, no label, no spread), so this passes for
/// a structural reason and would start failing if the return ever grew a
/// `..attrs`.
#[gtest]
fn a_hidden_input_gets_no_constraints() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Zip { code => { max_length: 10, widget: hidden } }
            })
        });
        form.render(using_fns! {})
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"type="hidden""#));
    expect_that!(html, not(contains_substring("maxlength")));
}

// ── Only where the element takes them (#4) ───────────────────────────────
//
// A field's constraints are its own whatever renders it, but an element only
// takes some: a `<textarea>` has no `pattern`. So the markup gets those the
// widget's element takes (its `EXTENDS`), and `check` still judges the value
// by all of them: the browser just cannot help with the one it lacks.

#[component]
fn PatternedTextarea() -> Element {
    let form = use_form(|| {
        empty_form(form! {
            Bio { about => { max_length: 200, pattern: r"\w+", widget: textarea } }
        })
    });
    form.render(using_fns! {})
}

#[gtest]
fn a_textarea_gets_maxlength_but_not_pattern() {
    let html = render(PatternedTextarea);
    expect_that!(html, contains_substring("<textarea"));
    expect_that!(html, contains_substring("maxlength=200"));
    expect_that!(html, not(contains_substring("pattern=")));
}

/// Off the element, not off the field: the pattern still refuses a value.
#[gtest]
fn a_textareas_pattern_is_still_checked() {
    let mut form = empty_form(form! {
        Bio { about => { pattern: r"\w+", widget: textarea } }
    });
    form.distribute_values(
        &[("about".to_string(), "two words".to_string())]
            .into_iter()
            .collect(),
    );
    expect_that!(form.validate(), none());
    expect_that!(
        form.collect_errors().fields.get("about"),
        some(elements_are![eq(&ValidationMessage::from(
            r"input should match the regular expression \w+"
        ))])
    );
}

// A constraint beside `choices` used to be tested here, rendering `maxlength`
// onto a `<select>` and a `<fieldset>`. `form!` now refuses the pair (the
// choices fix the values, so the dev makes each one meet the constraint), so
// the case lives in `tests/ui/form_choices_with_a_constraint.rs`.

// ── The map's invariant, end to end ──────────────────────────────────────

/// Dioxus does not dedupe attributes and HTML takes the FIRST of a duplicate,
/// so a second one would be silently dropped. The `IndexMap` makes it
/// structural; this checks the conversion back to a `Vec` preserves it.
#[gtest]
fn no_attribute_is_emitted_twice() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Zip { code => { min_length: 5, max_length: 10, pattern: r"\d+" } }
            })
        });
        form.render(using_fns! {})
    }
    let html = render(App);
    for attr in ["minlength", "maxlength", "pattern"] {
        expect_that!(html.matches(attr).count(), eq(1), "{attr} appears once");
    }
}

// ── The crossing that prompted the feature ───────────────────────────────

/// **`browser_validation` does not gate the attributes, and must not.**
/// `novalidate` on the `<form>` disables constraint validation form-wide, so it
/// already neutralizes them — threading the flag into every widget would
/// duplicate what one attribute does. So: same attributes either way, and only
/// the `<form>` differs.
#[gtest]
fn the_attributes_are_rendered_whether_browser_validation_is_on_or_off() {
    #[component]
    fn ValidationOff() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Zip { browser_validation: off, code => { max_length: 10 } }
            })
        });
        form.render(formoxus::using_fns! {})
    }
    #[component]
    fn ValidationOn() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Zip { browser_validation: on, code => { max_length: 10 } }
            })
        });
        form.render(formoxus::using_fns! {})
    }

    let off = render(ValidationOff);
    expect_that!(off, contains_substring("novalidate"));
    expect_that!(off, contains_substring("maxlength=10"));

    let on = render(ValidationOn);
    expect_that!(on, not(contains_substring("novalidate")));
    expect_that!(on, contains_substring("maxlength=10"));
}

// ── `required_true` on a bool (issue #6) ────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Terms {
    agreed: bool,
}

/// HTML `required` on a checkbox means "must be ticked", which is exactly
/// what `required_true` asks, so the browser can block the submit itself.
#[gtest]
fn a_required_true_checkbox_carries_required() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Terms { agreed => { required_true } } }));
        form.render(using_fns! {})
    }
    expect_that!(render(App), contains_substring("required=true"));
}

/// The other half, and the reason the rule needed its own key: a plain bool's
/// unticked box is a complete answer, so it must NOT get `required`, or the
/// browser would refuse to submit an honest "no".
#[gtest]
fn a_plain_checkbox_does_not_carry_required() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Terms {} }));
        form.render(using_fns! {})
    }
    expect_that!(render(App), not(contains_substring("required")));
}

/// With a rule behind it, the ` *` marker is finally honest on a checkbox. It
/// sits inside the label span and is `aria-hidden`, as on every other widget.
#[gtest]
fn a_required_true_checkbox_shows_the_marker() {
    #[component]
    fn Must() -> Element {
        let form = use_form(|| empty_form(form! { Terms { agreed => { required_true } } }));
        form.render(using_fns! {})
    }
    #[component]
    fn Plain() -> Element {
        let form = use_form(|| empty_form(form! { Terms {} }));
        form.render(using_fns! {})
    }
    expect_that!(
        render(Must),
        contains_regex(format!(
            r#"<label class="fx-field-label" for="{FORM_ID}-agreed">Agreed<span class="fx-required" aria-hidden="true"> \*</span></label>"#
        ))
    );
    expect_that!(render(Plain), not(contains_substring("fx-required")));
}

/// A `select` over a non-optional bool needs nothing new: an empty select is a
/// missing answer, so the PRESENCE `required` already gives it the marker.
#[gtest]
fn a_bool_select_gets_the_marker_from_presence() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Terms { agreed => { widget: select } } }));
        form.render(using_fns! {})
    }
    expect_that!(render(App), contains_substring("fx-required"));
}
