//! `label_case: "Label Case"` — the string is an example of itself.
//!
//! This lives in the consumer suite rather than inside either crate because it
//! is the only place both halves are visible: `form!`'s table maps a string to
//! a `LabelCase` variant, and `to_case` decides what that variant does. Neither
//! crate can check the other — formoxus-macros does not depend on formoxus —
//! so a test written from outside is what keeps them honest.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::label_case::{LabelCase, ToCase};
use formoxus::{Formoxus, defaults, empty_form, form, provide_defaults, use_form};
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Model {
    label_case: String,
}

/// Every spelling `form!` accepts, with what it should produce.
///
/// Written out by hand rather than imported, deliberately: this is the
/// independent statement of intent that the macro's own table is checked
/// against. Sharing one table would make the test agree with the macro by
/// construction and prove nothing.
const EXPECTED: [(&str, LabelCase); 11] = [
    ("labelCase", LabelCase::CamelLower),
    ("LabelCase", LabelCase::CamelCapitalized),
    ("label-case", LabelCase::KebabLower),
    ("Label-Case", LabelCase::KebabCapitalized),
    ("LABEL-CASE", LabelCase::KebabAllCaps),
    ("label_case", LabelCase::SnakeLower),
    ("Label_Case", LabelCase::SnakeCapitalized),
    ("LABEL_CASE", LabelCase::SnakeAllCaps),
    ("Label Case", LabelCase::Title),
    ("label case", LabelCase::Lower),
    ("LABEL CASE", LabelCase::AllCaps),
];

/// **The invariant the whole design rests on.**
///
/// Each key is not a name for a casing, it is a *demonstration* of one. If an
/// arm of `to_case` ever changes, the corresponding key silently becomes a lie
/// about what it does — a user writes `"Label_Case"` and gets something else.
/// Nothing else in the codebase would catch that.
#[gtest]
fn the_key_is_what_the_case_does() {
    for (key, case) in EXPECTED {
        expect_that!(
            "label_case".to_case(case),
            eq(key),
            "the string {key:?} must be exactly what {case:?} produces"
        );
    }
}

/// No two spellings collide, so the table can be a lookup rather than needing
/// disambiguation.
#[gtest]
fn every_spelling_is_distinct() {
    let mut seen: Vec<String> = EXPECTED.iter().map(|(k, _)| k.to_string()).collect();
    seen.sort();
    let before = seen.len();
    seen.dedup();
    expect_that!(seen.len(), eq(before));
}

/// And the macro agrees — `form!` really does resolve each spelling to the
/// variant this file expects, end to end through a rendered label.
///
/// One case per test would be eleven near-identical tests; the loop is worth it
/// here because the failure message names the spelling that broke.
#[gtest]
fn the_macro_resolves_every_spelling() {
    // Written out rather than looped over `EXPECTED`, because `form!` needs a
    // literal — which is the point of it being checked at compile time.
    let built = [
        (
            "labelCase",
            empty_form::<Model>(form! { Model { label_case: "labelCase" } }),
        ),
        (
            "LabelCase",
            empty_form::<Model>(form! { Model { label_case: "LabelCase" } }),
        ),
        (
            "label-case",
            empty_form::<Model>(form! { Model { label_case: "label-case" } }),
        ),
        (
            "Label-Case",
            empty_form::<Model>(form! { Model { label_case: "Label-Case" } }),
        ),
        (
            "LABEL-CASE",
            empty_form::<Model>(form! { Model { label_case: "LABEL-CASE" } }),
        ),
        (
            "label_case",
            empty_form::<Model>(form! { Model { label_case: "label_case" } }),
        ),
        (
            "Label_Case",
            empty_form::<Model>(form! { Model { label_case: "Label_Case" } }),
        ),
        (
            "LABEL_CASE",
            empty_form::<Model>(form! { Model { label_case: "LABEL_CASE" } }),
        ),
        (
            "Label Case",
            empty_form::<Model>(form! { Model { label_case: "Label Case" } }),
        ),
        (
            "label case",
            empty_form::<Model>(form! { Model { label_case: "label case" } }),
        ),
        (
            "LABEL CASE",
            empty_form::<Model>(form! { Model { label_case: "LABEL CASE" } }),
        ),
    ];

    for (spelling, state) in built {
        let want = EXPECTED
            .iter()
            .find(|(k, _)| *k == spelling)
            .map(|(_, c)| format!("{c:?}"))
            .expect("spelling is in EXPECTED");
        let got = state.label_case().expect("the form stated a case");
        expect_that!(
            format!("{got:?}"),
            eq(&want),
            "form! resolved {spelling:?} to the wrong case"
        );
    }
}

/// A form that says nothing stores nothing — `None` is "not stated", not
/// "Title". Resolving it is the job of the tier above, which needs a runtime.
#[gtest]
fn a_form_that_says_nothing_stores_nothing() {
    let state = empty_form::<Model>(form! { Model { title: "No casing stated" } });
    expect_that!(state.label_case(), none());
}

/// The setting reaches the markup, not just the spec.
///
/// Everything above would pass if `label_case` were stored and then ignored —
/// this is the one that fails if the `RenderCtx` plumbing breaks.
#[gtest]
fn the_chosen_case_reaches_the_rendered_label() {
    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Signup {
        email_address: String,
    }

    #[component]
    fn Kebab() -> Element {
        let form = use_form(|| empty_form(form! { Signup { label_case: "LABEL-CASE" } }));
        form.render_fragment()
    }

    let html = super::render_to_html(Kebab);
    expect_that!(html, contains_substring("EMAIL-ADDRESS"));
    // And not the default it would have had otherwise.
    expect_that!(html, not(contains_substring("Email Address")));
}

// ── The cascade: app default -> per-form -> built-in ─────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Person {
    first_name: String,
}

/// With nobody providing anything, the built-in applies.
#[gtest]
fn with_no_app_default_a_silent_form_is_title_case() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Person {} }));
        form.render_fragment()
    }
    expect_that!(super::render_to_html(App), contains_substring("First Name"));
}

/// An app-level default reaches a form that stated nothing.
#[gtest]
fn an_app_default_reaches_a_silent_form() {
    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().with_label_case(LabelCase::SnakeAllCaps));
        let form = use_form(|| empty_form(form! { Person {} }));
        form.render_fragment()
    }
    let html = super::render_to_html(App);
    expect_that!(html, contains_substring("FIRST_NAME"));
    expect_that!(html, not(contains_substring("First Name")));
}

/// **The precedence that makes it a cascade and not just a global.** The form
/// wins over the app default, which is the whole point of the middle tier.
#[gtest]
fn a_form_overrides_the_app_default() {
    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().with_label_case(LabelCase::SnakeAllCaps));
        let form = use_form(|| empty_form(form! { Person { label_case: "label-case" } }));
        form.render_fragment()
    }
    let html = super::render_to_html(App);
    expect_that!(html, contains_substring("first-name"));
    expect_that!(html, not(contains_substring("FIRST_NAME")));
}

/// A nested provider shadows an outer one for its subtree only — which a
/// `static` could not express at all, and is why this is context.
#[gtest]
fn a_nested_provider_shadows_an_outer_one() {
    #[component]
    fn Inner() -> Element {
        provide_defaults(defaults().with_label_case(LabelCase::KebabAllCaps));
        let form = use_form(|| empty_form(form! { Person {} }));
        form.render_fragment()
    }

    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().with_label_case(LabelCase::SnakeAllCaps));
        let outer = use_form(|| empty_form(form! { Person {} }));
        rsx! {
            {outer.render_fragment()}
            Inner {}
        }
    }

    let html = super::render_to_html(App);
    // Both are present: the outer form keeps the outer default.
    expect_that!(html, contains_substring("FIRST_NAME"));
    expect_that!(html, contains_substring("FIRST-NAME"));
}

// ── Where the case has to reach, beyond a field label ────────────────────
//
// A field label is not the only derived text. A button with no `text` and a
// variant name in a `VariantSelect` are both derived from an identifier, and
// both used to be hardcoded to `Title`. Reading `defaults()` at the point of
// use is NOT enough for either: that is only the app tier, so a form stating
// its own case would still be ignored. These pin the full cascade.

#[gtest]
fn a_textless_button_uses_the_forms_case() {
    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Account {
        nickname: String,
    }

    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Account {
                    label_case: "label-case",
                    buttons: { sign_in: { type: submit } }
                }
            })
        });
        form.render(formoxus::using_fns! { sign_in: |_m| async move {} })
    }
    let html = super::render_to_html(App);
    expect_that!(html, contains_substring(">sign-in<"));
    expect_that!(html, not(contains_substring("Sign In")));
}

/// And the app tier still reaches a button, for a form that states nothing.
#[gtest]
fn a_textless_button_uses_the_app_default() {
    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Account {
        nickname: String,
    }

    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().with_label_case(LabelCase::SnakeAllCaps));
        let form = use_form(|| {
            empty_form(form! {
                Account { buttons: { sign_in: { type: submit } } }
            })
        });
        form.render(formoxus::using_fns! { sign_in: |_m| async move {} })
    }
    expect_that!(super::render_to_html(App), contains_substring(">SIGN_IN<"));
}

#[gtest]
fn a_variant_name_uses_the_forms_case() {
    #[derive(Facet, Clone, Debug, PartialEq)]
    #[repr(u8)]
    enum Billing {
        CreditCard { number: String },
        BankTransfer { account: String },
    }

    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Order {
        billing: Billing,
    }

    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Order { label_case: "label-case" } }));
        form.render_fragment()
    }
    let html = super::render_to_html(App);
    expect_that!(html, contains_substring(">credit-card<"));
    expect_that!(html, not(contains_substring(">Credit Card<")));
}
