//! Constraints reaching the BROWSER, not just Rust.
//!
//! `ValueKind::check` enforces every constraint server-side; these pin the
//! other half, that the same constraint is handed to the browser so it can
//! give instant feedback and block submit. The mapping itself is unit-tested
//! beside `attrs` in `fields.rs`; this is the end-to-end view from `form!`.
//!
//! **Numeric attributes render UNQUOTED** (`maxlength=10`), because they are
//! `AttributeValue::Int`. A `pattern` is text and does get quotes.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;
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
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring("minlength=5"));
    expect_that!(html, contains_substring("maxlength=10"));
    expect_that!(html, contains_substring(r#"pattern="\d+""#));
}

/// **The pattern reaches the DOM unanchored.** HTML wraps a `pattern` as
/// `^(?:…)$` implicitly, and `ValueKind::check` wraps it Rust-side to match —
/// so handing the browser an already-wrapped pattern would anchor it twice and
/// the two would stop agreeing. Agreement is the whole reason `regress` is a
/// dependency.
#[gtest]
fn a_pattern_reaches_the_dom_unanchored() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Zip { code => { pattern: r"\d{5}" } } }));
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"pattern="\d{5}""#));
    expect_that!(html, not(contains_substring("^(?:")));
}

#[gtest]
fn a_numeric_bound_reaches_the_input() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Counts {
                    whole => { min: 1, max: 9 },
                    fraction => { min: 0.5 },
                }
            })
        });
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring("min=1"));
    expect_that!(html, contains_substring("max=9"));
    expect_that!(html, contains_substring("min=0.5"));
}

/// **No `step`, deliberately.** `<input type="number">` has an implicit
/// `step=1` and would reject `0.5` — but formoxus never defaults a float to
/// `type="number"` (`Int`/`Float` default to `Text`), so the only way to meet
/// that is an explicit `widget: number`, and then supplying `step` is the
/// author's job. See the note in `ValueKind::attrs`.
#[gtest]
fn no_step_is_emitted_for_a_float() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Counts { fraction => { min: 0.5, max: 9.5 } } }));
        form.render_fragment()
    }
    let html = render(App);
    // Positive first, so the absence below cannot pass by rendering nothing.
    expect_that!(html, contains_substring("min=0.5"));
    expect_that!(html, contains_substring("max=9.5"));
    expect_that!(html, not(contains_substring("step")));
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
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"type="hidden""#));
    expect_that!(html, not(contains_substring("maxlength")));
}

// ── Emitted even where HTML says they do not belong ──────────────────────
//
// These pin a DECISION, not an accident: formoxus emits every derived
// attribute onto whatever element the widget renders and lets the browser
// ignore what does not apply. That is invalid HTML, knowingly — the W3C
// validator flags 50 of the 70 (input type, attribute) pairs, so there was no
// coherent subset to carve out. Tracked in
// <https://github.com/toddobryan/formoxus/issues/4>. If that issue is fixed,
// these tests are the ones that should change.

#[gtest]
fn a_textarea_gets_pattern_although_html_has_no_such_attribute() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Bio { about => { max_length: 200, pattern: r"\w+", widget: textarea } }
            })
        });
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring("<textarea"));
    expect_that!(html, contains_substring("maxlength=200"));
    expect_that!(html, contains_substring(r#"pattern="\w+""#));
}

/// A chooser is not an `<input>` at all, and `Select`/`RadioGroup` both accept
/// a `Text` value kind — so a constrained `String` carries its attributes onto
/// a `<select>` or a `<fieldset>`. Which is why issue #4 cannot be fixed by
/// gating on `InputType` alone.
#[gtest]
fn a_chooser_carries_them_onto_a_select_or_fieldset() {
    const STATES: &[(&str, &str)] = &[("AL", "Alabama")];

    #[component]
    fn AsSelect() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Zip { code => { max_length: 10, widget: select { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    #[component]
    fn AsRadios() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Zip { code => { max_length: 10, widget: radio_group { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    expect_that!(render(AsSelect), contains_substring("<select"));
    expect_that!(render(AsSelect), contains_substring("maxlength=10"));
    expect_that!(render(AsRadios), contains_substring("<fieldset"));
    expect_that!(render(AsRadios), contains_substring("maxlength=10"));
}

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
        form.render_fragment()
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

// ── `required: true` on a bool (issue #6) ────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Terms {
    agreed: bool,
}

/// HTML `required` on a checkbox means "must be ticked", which is exactly
/// what `required: true` asks, so the browser can block the submit itself.
#[gtest]
fn a_required_true_checkbox_carries_required() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Terms { agreed => { required: true } } }));
        form.render_fragment()
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
        form.render_fragment()
    }
    expect_that!(render(App), not(contains_substring("required")));
}

/// With a rule behind it, the ` *` marker is finally honest on a checkbox. It
/// sits inside the label span and is `aria-hidden`, as on every other widget.
#[gtest]
fn a_required_true_checkbox_shows_the_marker() {
    #[component]
    fn Must() -> Element {
        let form = use_form(|| empty_form(form! { Terms { agreed => { required: true } } }));
        form.render_fragment()
    }
    #[component]
    fn Plain() -> Element {
        let form = use_form(|| empty_form(form! { Terms {} }));
        form.render_fragment()
    }
    expect_that!(
        render(Must),
        contains_substring(
            r#"<span class="fx-field-label">Agreed<span class="fx-required" aria-hidden="true"> *</span></span>"#
        )
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
        form.render_fragment()
    }
    expect_that!(render(App), contains_substring("fx-required"));
}
