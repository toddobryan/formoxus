//! `browser_validation: off` and `Form::reset`.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::{Formoxus, empty_form, form, form_for, provide_defaults, use_form, using_fns};
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Contact {
    name: String,
    email: String,
}

fn render(app: fn() -> Element) -> String {
    super::render_to_html(app)
}

// ── browser_validation ───────────────────────────────────────────────────

/// The default is unchanged behaviour: the browser still gates submission.
#[gtest]
fn by_default_the_form_has_no_novalidate() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact {} }));
        form.render(using_fns! {})
    }
    expect_that!(render(App), not(contains_substring("novalidate")));
}

/// `off` reaches the `<form>` element.
#[gtest]
fn turning_it_off_renders_novalidate() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact { browser_validation: off } }));
        form.render(using_fns! {})
    }
    expect_that!(render(App), contains_substring("novalidate"));
}

/// **The attribute is absent, not `novalidate="false"`.** In HTML a boolean
/// attribute that is merely present is true, so rendering it with a false value
/// would turn validation off exactly when the form asked to keep it on.
#[gtest]
fn on_renders_no_attribute_at_all() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact { browser_validation: on } }));
        form.render(using_fns! {})
    }
    expect_that!(render(App), not(contains_substring("novalidate")));
}

/// The app-wide default reaches a form that stated nothing.
#[gtest]
fn an_app_default_reaches_a_silent_form() {
    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().use_browser_validation(false));
        let form = use_form(|| empty_form(form! { Contact {} }));
        form.render(using_fns! {})
    }
    expect_that!(render(App), contains_substring("novalidate"));
}

/// And the form still wins over it — the middle tier of the cascade.
#[gtest]
fn a_form_overrides_the_app_default() {
    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().use_browser_validation(false));
        let form = use_form(|| empty_form(form! { Contact { browser_validation: on } }));
        form.render(using_fns! {})
    }
    expect_that!(render(App), not(contains_substring("novalidate")));
}

// ── reset ────────────────────────────────────────────────────────────────

/// An edited field goes back to what it started as.
#[gtest]
fn reset_restores_the_seeded_values() {
    #[component]
    fn App() -> Element {
        let contact = Contact {
            name: "Ada".into(),
            email: "ada@example.com".into(),
        };
        let form = use_form(move || form_for(&contact, form! { Contact {} }));

        // Type over the seeded value, then put it back.
        formoxus::controls::write_value("name", form.values(), "Grace".to_string());
        form.reset();

        form.render_fragment()
    }
    let html = render(App);
    expect_that!(html, contains_substring(r#"value="Ada""#));
    expect_that!(html, not(contains_substring("Grace")));
}

/// A form built empty resets back to empty, not to whatever was typed.
#[gtest]
fn reset_of_an_empty_form_clears_it() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact {} }));
        formoxus::controls::write_value("name", form.values(), "typed".to_string());
        form.reset();
        form.render_fragment()
    }
    expect_that!(render(App), not(contains_substring("typed")));
}

/// Errors go too — a reset form is clean, not merely re-valued.
#[gtest]
fn reset_clears_validation_errors() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Contact {} }));
        // Required fields are empty, so this fails and records errors.
        let _ = form.validate();
        form.reset();
        form.render_fragment()
    }
    expect_that!(render(App), not(contains_substring("field-error")));
}
