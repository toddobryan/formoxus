//! Value choices: `widget: select { choices: … }`.
//!
//! Distinct from the *shape* choice a `VariantSelect` offers — that picks which
//! fields exist, this picks a value for one field that already exists. See
//! `.claude/memory/choice_fields_design.md`.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::prelude::*;
use formoxus::widgets::SelectChoice;
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    state: String,
    zip: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Optional {
    state: Option<String>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Flagged {
    agreed: bool,
}

const STATES: &[(&str, &str)] = &[("AL", "Alabama"), ("AK", "Alaska")];

fn render(app: fn() -> Element) -> String {
    super::render_to_html(app)
}

// ── Rendering ────────────────────────────────────────────────────────────

#[gtest]
fn a_choice_renders_its_value_and_its_display_separately() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Address { state => { widget: select { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"<option value="AL""#));
    expect_that!(html, contains_substring("Alabama</option>"));
    // The postal code is what the form stores; the name is only ever read.
    expect_that!(html, not(contains_substring(r#"value="Alabama""#)));
}

#[gtest]
fn the_current_value_is_the_selected_option() {
    #[component]
    fn App() -> Element {
        let address = Address {
            state: "AK".into(),
            zip: "99501".into(),
        };
        let form = use_form(move || {
            form_for(
                &address,
                form! { Address { state => { widget: select { choices: STATES } } } },
            )
        });
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(
        html,
        contains_substring(r#"<option value="AK" selected=true>Alaska</option>"#)
    );
}

/// A field with no choices keeps its default widget, so the list is genuinely
/// per-field rather than per-type.
#[gtest]
fn a_sibling_without_choices_is_still_an_input() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Address { state => { widget: select { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"name="zip""#));
    expect_that!(html, contains_substring(r#"<input type="text" name="zip""#));
}

/// An optional field offers a real way back to "unanswered"; a required one
/// gets the unselectable placeholder instead. Both come from `Select` itself,
/// so a choice list never has to include an empty entry — and must not, since
/// `""` IS absence.
#[gtest]
fn an_optional_choice_field_offers_the_absent_entry() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Optional { state => { widget: select { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    expect_that!(
        render(App),
        contains_substring(format!(">{}</option>", formoxus::ABSENT_DISPLAY))
    );
}

#[gtest]
fn a_required_choice_field_gets_an_unselectable_placeholder() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Address { state => { widget: select { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    expect_that!(render(App), contains_substring("disabled=true"));
}

/// A `bool` derives its own choices, so it renders without a list — but a
/// stated one wins, for a form that would rather say Yes/No.
#[gtest]
fn an_explicit_list_overrides_a_bools_derived_one() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Flagged { agreed => { widget: select { choices: &[("true", "Yes"), ("false", "No")] } } }
            })
        });
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring("Yes</option>"));
    expect_that!(html, not(contains_substring("True</option>")));
}

// A `select` with nothing to choose from used to be tested here by rendering
// it and asserting the field was missing. `form!` now rejects it at compile
// time, so the case lives in `tests/ui/form_select_without_choices.rs`.

// ── The list itself ──────────────────────────────────────────────────────

/// `SelectChoice` holds `String`s, so a list of them can never be `const`. The
/// conversions are what let the table be a `const` of pairs anyway — which is
/// how anyone actually writes fifty states.
#[gtest]
fn a_const_table_of_pairs_becomes_choices() {
    let built: Vec<SelectChoice> = STATES.iter().map(Into::into).collect();
    expect_that!(built.len(), eq(2));
    expect_that!(built[0].value, eq("AL"));
    expect_that!(built[0].display, eq("Alabama"));
}

/// A bare string is a choice whose display IS its value, for a list that needs
/// no separate code.
#[gtest]
fn a_bare_string_is_its_own_display() {
    let choice: SelectChoice = "Alabama".into();
    expect_that!(choice.value, eq("Alabama"));
    expect_that!(choice.display, eq("Alabama"));
}

// ── Round trip ───────────────────────────────────────────────────────────

/// The whole point of the raw value: it goes through the same parse path every
/// other field uses, so a chosen value validates into the model unchanged.
#[gtest]
fn a_chosen_value_validates_into_the_model() {
    let spec = || {
        form! { Address { state => { widget: select { choices: STATES } } } }
    };
    let values = [("state", "AK"), ("zip", "99501")]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let submission = Submission::accept(spec(), &values).expect("a chosen value is a valid value");
    expect_that!(submission.model().state, eq("AK"));
}

// ── Radio groups ─────────────────────────────────────────────────────────
//
// The same choice list, rendered as radios. An `Option` field is missing from
// this section on purpose: `radio_group` rejects one at compile time, so that
// case is `tests/ui/form_radio_group_on_an_option.rs`. There is no round-trip
// test either — a radio writes the raw value to the same path through the same
// `write_value`, so `a_chosen_value_validates_into_the_model` already covers it.

#[gtest]
fn a_radio_group_renders_one_radio_per_choice() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Address { state => { widget: radio_group { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(
        html,
        contains_substring(r#"type="radio" name="state" value="AL""#)
    );
    expect_that!(
        html,
        contains_substring(r#"type="radio" name="state" value="AK""#)
    );
    // Value and display stay separate, exactly as in the `<select>`.
    expect_that!(html, contains_substring("Alabama"));
    expect_that!(html, not(contains_substring(r#"value="Alabama""#)));
}

/// Every radio in the group shares one `name` — that is what makes them one
/// group to the browser, and what the submitted value comes back under.
#[gtest]
fn every_radio_shares_the_fields_name() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Address { state => { widget: radio_group { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    expect_that!(render(App).matches(r#"name="state""#).count(), eq(2));
}

/// **Nothing is pre-selected.** A library that checked the first choice would
/// make a required radio field unable to fail its own required check, and it
/// would submit an answer the user never gave.
#[gtest]
fn no_radio_is_checked_before_the_user_picks() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Address { state => { widget: radio_group { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    expect_that!(render(App), not(contains_substring("checked")));
}

#[gtest]
fn the_stored_value_is_the_checked_radio() {
    #[component]
    fn App() -> Element {
        let address = Address {
            state: "AK".into(),
            zip: "99501".into(),
        };
        let form = use_form(move || {
            form_for(
                &address,
                form! { Address { state => { widget: radio_group { choices: STATES } } } },
            )
        });
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"value="AK" checked=true"#));
    // Exactly that one, not both.
    expect_that!(html.matches("checked=true").count(), eq(1));
}

/// A `bool` derives its own choices, so it is the one kind that renders as
/// radios without a list.
#[gtest]
fn a_bool_derives_true_and_false_radios() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Flagged { agreed => { widget: radio_group } } }));
        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"value="true""#));
    expect_that!(html, contains_substring("True"));
    expect_that!(html, contains_substring(r#"value="false""#));
    expect_that!(html, contains_substring("False"));
}

/// `required` on each input, and NO absent entry. On a radio group `required`
/// applies to the whole group, so the browser blocks submit until one is
/// picked — which is the only thing standing in for the `--none--` option a
/// `<select>` offers, and the reason an optional field is rejected outright.
#[gtest]
fn every_radio_is_required_and_nothing_offers_absence() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Address { state => { widget: radio_group { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    let html = render(App);
    // Per radio, not a count over the whole form: `zip` is a required field
    // too, so its `<input>` carries `required=true` as well.
    expect_that!(html, contains_substring(r#"value="AL" required=true"#));
    expect_that!(html, contains_substring(r#"value="AK" required=true"#));
    expect_that!(html, not(contains_substring(formoxus::ABSENT_DISPLAY)));
}

/// The group's label is a `legend`, not a `label`: a `<label>` names one
/// control and there are several here.
#[gtest]
fn the_group_label_is_a_legend() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Address { state => { widget: radio_group { choices: STATES } } }
            })
        });
        form.render_fragment()
    }
    expect_that!(render(App), contains_substring("<legend>State"));
}
