//! `Classes` from a consumer's side: that each type is reachable at the path
//! the prelude rule promises, which only a dependent crate can prove.
//!
//! What `Classes::fx()` holds and how the builders behave is unit-tested in
//! `src/classes.rs`; what the classes render as is the snapshot's job.
//!
//! The second half is the cascade: `Form::classes()` resolving the app's
//! `Classes`, a form's `classes:` and its `classes_plus:` edits. Each test
//! renders the resolved lookups as text, since nothing renders THROUGH the
//! classes until P3-7; it needs a component for `provide_defaults`.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::classes::{FixedKey, Slot};
use formoxus::widgets::{BuiltinWidget, FieldProps, RenderedInputType, ValuesStore};
use formoxus::{Classes, Form, Formoxus, empty_form, form, provide_defaults, use_form};
use googletest::prelude::*;

#[gtest]
fn classes_is_in_the_prelude_and_at_the_crate_root() {
    use formoxus::prelude::*;
    expect_that!(Classes::default(), eq(&formoxus::Classes::fx()));
}

#[gtest]
fn a_custom_classes_is_built_from_formoxus_classes_and_widgets() {
    let classes = formoxus::Classes::bare()
        .with_fixed(FixedKey::Form, Slot::new("form"))
        .with_part(
            BuiltinWidget::Textarea,
            "textarea",
            Slot::new("input").with_invalid("is-invalid"),
        );
    expect_that!(classes.fixed_class(FixedKey::Form, false), some(eq("form")));
    expect_that!(
        classes.part_class(BuiltinWidget::Textarea, "textarea", true),
        some(eq("input is-invalid"))
    );
    expect_that!(classes.fixed_class(FixedKey::FormTitle, false), none());
}

// ── The cascade ──────────────────────────────────────────────────────────

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    name: String,
}

/// The app's classes for these tests: two fixed slots and a select, none of
/// them `fx()`'s, so a test can tell an app value from a built-in one.
fn app_classes() -> Classes {
    Classes::bare()
        .with_fixed(FixedKey::Form, Slot::new("app-form"))
        .with_fixed(FixedKey::ButtonRow, Slot::new("app-row"))
        .with_part(
            BuiltinWidget::Select,
            "select",
            Slot::new("app-select").with_invalid("app-bad"),
        )
}

/// What a test reads off the resolved classes: the form, the button row, and
/// a select and a text input in both states, `-` for no class.
fn lookups(form: &Form<Signup>) -> String {
    let classes = form.classes();
    let show = |class: Option<String>| class.unwrap_or_else(|| "-".to_string());
    let text = BuiltinWidget::Input(RenderedInputType::Text);
    format!(
        "form=[{}] row=[{}] select=[{}] select!=[{}] input!=[{}]",
        show(classes.fixed_class(FixedKey::Form, false)),
        show(classes.fixed_class(FixedKey::ButtonRow, false)),
        show(classes.part_class(BuiltinWidget::Select, "select", false)),
        show(classes.part_class(BuiltinWidget::Select, "select", true)),
        show(classes.part_class(text, "input", true)),
    )
}

fn resolved(app: fn() -> Element) -> String {
    super::render_to_html(app)
}

/// No provider, no keys: `fx()`, the built-in default.
#[gtest]
fn a_silent_form_with_no_app_classes_gets_fx() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| empty_form(form! { Signup {} }));
        rsx! { "{lookups(&form)}" }
    }
    expect_that!(
        resolved(App),
        eq(
            "form=[fx-form] row=[fx-buttons] select=[fx-control fx-select] \
            select!=[fx-control fx-select] input!=[fx-control fx-input]"
        )
    );
}

#[gtest]
fn a_silent_form_gets_the_apps_classes() {
    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().with_classes(app_classes()));
        let form = use_form(|| empty_form(form! { Signup {} }));
        rsx! { "{lookups(&form)}" }
    }
    expect_that!(
        resolved(App),
        eq("form=[app-form] row=[app-row] select=[app-select] \
            select!=[app-select app-bad] input!=[-]")
    );
}

/// The point of `classes_plus:`: it changes only what it names, on top of
/// whatever the APP set, which the form never restates.
#[gtest]
fn classes_plus_layers_on_the_apps_classes() {
    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().with_classes(app_classes()));
        let form = use_form(|| {
            empty_form(form! {
                Signup {
                    classes_plus: {
                        form: + [mine],
                        select.select: { invalid: [is_invalid] },
                    }
                }
            })
        });
        rsx! { "{lookups(&form)}" }
    }
    expect_that!(
        resolved(App),
        eq("form=[app-form mine] row=[app-row] select=[app-select] \
            select!=[app-select is-invalid] input!=[-]")
    );
}

/// A form's `classes:` replaces the app's whole.
#[gtest]
fn classes_replaces_the_apps_classes() {
    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().with_classes(app_classes()));
        let form = use_form(|| empty_form(form! { Signup { classes: Classes::bare() } }));
        rsx! { "{lookups(&form)}" }
    }
    expect_that!(
        resolved(App),
        eq("form=[-] row=[-] select=[-] select!=[-] input!=[-]")
    );
}

/// Both keys: `classes:` first, then the edits on top of it, so a `+` onto
/// `bare()` is all there is.
#[gtest]
fn classes_then_classes_plus_apply_in_order() {
    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().with_classes(app_classes()));
        let form = use_form(|| {
            empty_form(form! {
                Signup {
                    classes_plus: {
                        form: + [mine],
                        text.input: { base: [form_control], invalid: + [is_invalid] },
                    },
                    classes: Classes::bare(),
                }
            })
        });
        rsx! { "{lookups(&form)}" }
    }
    expect_that!(
        resolved(App),
        eq("form=[mine] row=[-] select=[-] select!=[-] input!=[form-control is-invalid]")
    );
}

/// The list syntax reaches the slot normalised: `_` to `-`, quoted verbatim.
#[gtest]
fn classes_plus_values_are_normalised_as_a_fields_are() {
    #[component]
    fn App() -> Element {
        let form = use_form(|| {
            empty_form(form! {
                Signup { classes_plus: { button_row: [btn_group, "mt-4"] } }
            })
        });
        rsx! { "{lookups(&form)}" }
    }
    expect_that!(resolved(App), contains_substring("row=[btn-group mt-4]"));
}

// ── A custom widget sees them too ────────────────────────────────────────

/// A custom widget that shows what it was handed: the form slot and the
/// select's, from `props.classes`.
#[component]
fn ClassProbe(values: ValuesStore, props: FieldProps) -> Element {
    let _ = values;
    let form = props
        .classes
        .fixed_class(FixedKey::Form, false)
        .unwrap_or_default();
    let select = props
        .classes
        .part_class(BuiltinWidget::Select, "select", false)
        .unwrap_or_default();
    rsx! {
        span { "probe form=[{form}] select=[{select}]" }
    }
}

/// P3-b: a custom widget's classes are the FORM's resolved ones, the app's
/// with `classes_plus:` applied, so a custom widget can borrow a built-in
/// slot (here the select's) and follow whatever preset the app chose.
#[gtest]
fn a_custom_widget_receives_the_forms_resolved_classes() {
    #[component]
    fn App() -> Element {
        provide_defaults(Formoxus::new().with_classes(app_classes()));
        let form = use_form(|| {
            empty_form(form! {
                Signup {
                    name => { widget: custom(ClassProbe) },
                    classes_plus: { form: + [mine] },
                }
            })
        });
        form.render(formoxus::using_fns! {})
    }
    expect_that!(
        resolved(App),
        contains_substring("probe form=[app-form mine] select=[app-select]")
    );
}
