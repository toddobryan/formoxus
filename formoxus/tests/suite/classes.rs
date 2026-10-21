//! `Classes` from a consumer's side: that each type is reachable at the path
//! the prelude rule promises, which only a dependent crate can prove.
//!
//! What `Classes::fx()` holds and how the builders behave is unit-tested in
//! `src/classes.rs`; what the classes render as is the snapshot's job.
//!
//! The second half is the cascade: `Form::classes()` resolving the app's
//! `Classes`, a form's `classes:` and its `classes_plus:` edits. Those tests
//! render the resolved lookups as text; the third half renders real forms, to
//! show the same cascade reaching the markup (P3-7), along with a field's own
//! `class:`/`class_plus:` on top of it. Each needs a component for
//! `provide_defaults`.

use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::AttrSet;
use formoxus::classes::{FixedKey, Slot};
use formoxus::widgets::widget_props::WidgetProps;
use formoxus::widgets::{BuiltinWidget, RenderedInputType, Widget};
use formoxus::{Classes, Form, FormSpec, Formoxus, empty_form, form, provide_defaults, use_form};
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
#[derive(Clone, Debug, Default, PartialEq)]
struct ClassProbe;

impl Widget for ClassProbe {
    const NAME: &'static str = "class_probe";
    const EXTENDS: AttrSet = AttrSet::EMPTY;

    fn render(&self, props: WidgetProps) -> Element {
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

    fn default_classes() -> Vec<(&'static str, Slot)> {
        Vec::new()
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
                    name => { widget: ClassProbe },
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

// ── Through to the markup ────────────────────────────────────────────────

/// The app's classes for the rendered tests: none of them `fx()`'s, so any
/// `fx-` in the output would be a lookup that skipped the cascade.
fn rendered_app_classes() -> Classes {
    let text = BuiltinWidget::Input(RenderedInputType::Text);
    Classes::bare()
        .with_fixed(FixedKey::Form, Slot::new("app-form"))
        .with_part(text, "div", Slot::new("app-field"))
        .with_part(
            text,
            "input",
            Slot::new("app-input").with_invalid("app-bad"),
        )
}

/// `spec` rendered under [`rendered_app_classes`], validated first when
/// `invalid`, so the required `name` has an error to show.
fn render_under_app(spec: fn() -> FormSpec<Signup>, invalid: bool) -> String {
    /// What `App` renders: the spec, and whether to validate it first. A
    /// thread-local because `render_to_html` takes a bare `fn() -> Element`.
    type Case = (fn() -> FormSpec<Signup>, bool);
    thread_local! {
        static SPEC: std::cell::Cell<Option<Case>> = const { std::cell::Cell::new(None) };
    }
    #[component]
    fn App() -> Element {
        let (spec, invalid) = SPEC.with(std::cell::Cell::get).expect("set before render");
        provide_defaults(Formoxus::new().with_classes(rendered_app_classes()));
        let form = use_form(|| {
            let mut state = empty_form(spec());
            if invalid {
                let _ = state.validate();
            }
            state
        });
        form.render(formoxus::using_fns! {})
    }
    SPEC.with(|cell| cell.set(Some((spec, invalid))));
    resolved(App)
}

/// Every `class="…"` value in `html`, in order.
fn class_values(html: &str) -> Vec<&str> {
    html.split(r#"class=""#)
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect()
}

#[gtest]
fn a_silent_form_renders_the_apps_classes() {
    let html = render_under_app(|| form! { Signup {} }, false);
    expect_that!(
        class_values(&html),
        elements_are![eq(&"app-form"), eq(&"app-field"), eq(&"app-input")]
    );
}

/// A form's `classes:` replaces the app's whole, in the markup too: not one
/// of the app's classes survives.
#[gtest]
fn a_forms_classes_replace_the_apps_in_the_markup() {
    let html = render_under_app(
        || {
            form! {
                Signup {
                    classes: Classes::bare().with_fixed(FixedKey::Form, Slot::new("mine")),
                }
            }
        },
        false,
    );
    expect_that!(class_values(&html), elements_are![eq(&"mine")]);
}

/// `classes_plus:` on an app's CUSTOM classes, not `fx()`'s: what it names
/// changes, `+` appending to the app's base, and nothing else does.
#[gtest]
fn classes_plus_edits_only_what_it_names_on_the_apps_classes() {
    let html = render_under_app(
        || {
            form! {
                Signup {
                    classes_plus: {
                        form: + [mine],
                        text.input: [replaced],
                    },
                }
            }
        },
        false,
    );
    expect_that!(
        class_values(&html),
        elements_are![eq(&"app-form mine"), eq(&"app-field"), eq(&"replaced")]
    );
}

/// Both keys: `classes:` first, then `classes_plus:` on top, so with
/// `bare()` the edits are the only classes on the page.
#[gtest]
fn classes_then_classes_plus_in_the_markup() {
    let html = render_under_app(
        || {
            form! {
                Signup {
                    classes_plus: { text.input: + [only] },
                    classes: Classes::bare(),
                }
            }
        },
        false,
    );
    expect_that!(class_values(&html), elements_are![eq(&"only")]);
}

/// A field's `class:` replaces a CONFIGURED base, the app's, not just
/// `fx()`'s.
#[gtest]
fn a_fields_class_replaces_the_apps_base() {
    let html = render_under_app(|| form! { Signup { name => { class: [wide] } } }, false);
    expect_that!(class_values(&html), contains(eq(&"wide")));
    expect_that!(html, not(contains_substring("app-input")));
}

/// A field's `class_plus:` appends to the app's base.
#[gtest]
fn a_fields_class_plus_appends_to_the_apps_base() {
    let html = render_under_app(
        || form! { Signup { name => { class_plus: [wide] } } },
        false,
    );
    expect_that!(class_values(&html), contains(eq(&"app-input wide")));
}

/// P3-g: an author's `class:` replaces the base, but the slot's invalid class
/// still marks an errored field, so a preset's error styling survives it.
#[gtest]
fn the_invalid_class_survives_a_fields_class() {
    let html = render_under_app(|| form! { Signup { name => { class: [wide] } } }, true);
    expect_that!(class_values(&html), contains(eq(&"wide app-bad")));
}

/// And under `class_plus:`, between the base it appends to and the invalid
/// class: base, author's, invalid.
#[gtest]
fn the_invalid_class_follows_a_fields_class_plus() {
    let html = render_under_app(|| form! { Signup { name => { class_plus: [wide] } } }, true);
    expect_that!(class_values(&html), contains(eq(&"app-input wide app-bad")));
}

/// A part with no slot still takes the author's classes, and `class_plus:`
/// onto nothing has no stray leading space.
#[gtest]
fn a_fields_classes_reach_a_part_with_no_slot() {
    let replaced = render_under_app(
        || form! { Signup { name => { class: [wide] }, classes: Classes::bare() } },
        false,
    );
    expect_that!(class_values(&replaced), elements_are![eq(&"wide")]);
    let appended = render_under_app(
        || form! { Signup { name => { class_plus: [wide] }, classes: Classes::bare() } },
        false,
    );
    expect_that!(class_values(&appended), elements_are![eq(&"wide")]);
}
