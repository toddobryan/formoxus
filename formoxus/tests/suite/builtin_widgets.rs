//! The built-in widget types (`FIELDS_PLAN.md` Step 5, W-C), driven through
//! `Widget::render` on an `AnyWidget` directly, the way `FieldNode` does, so
//! each is tested apart from the node that hands it its props.

use dioxus::core::Attribute;
use dioxus::prelude::*;
use formoxus::ValidationMessage;
use formoxus::attrs::AttrSet;
use formoxus::attrs::{Attr, AttrKey, AttrValue, InputType as T};
use formoxus::classes::Classes;
use formoxus::field_attrs::FieldAttrs;
use formoxus::widgets::Choice;
use formoxus::widgets::WidgetProps;
use formoxus::widgets::{
    AnyWidget, CheckboxInput, ColorInput, DateInput, DatetimeLocalInput, EmailInput, HiddenInput,
    MonthInput, NumberInput, PasswordInput, RadioGroup, SearchInput, Select, TelInput, TextInput,
    Textarea, TimeInput, UrlInput, WeekInput, Widget,
};
use googletest::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;

const ID: &str = "rand0123-title";
const PATH: &str = "title";

/// Every text-like widget type, beside the `type=` it renders. Nothing here
/// notices one added to the macro and left out of this list; that every
/// built-in has a `WidgetName` row is `formoxus`'s own
/// `each_builtins_row_names_the_builtin_itself`.
fn text_likes() -> Vec<(AnyWidget, T)> {
    vec![
        (TextInput.into(), T::Text),
        (PasswordInput.into(), T::Password),
        (NumberInput.into(), T::Number),
        (EmailInput.into(), T::Email),
        (TelInput.into(), T::Telephone),
        (UrlInput.into(), T::Url),
        (SearchInput.into(), T::Search),
        (ColorInput.into(), T::Color),
        (DateInput.into(), T::Date),
        (TimeInput.into(), T::Time),
        (DatetimeLocalInput.into(), T::DatetimeLocal),
        (MonthInput.into(), T::Month),
        (WeekInput.into(), T::Week),
    ]
}

/// SSR of `root` given `props`: `render_to_html`, for a root that takes props,
/// so one root serves every widget in the table.
fn html_of<P: Clone + 'static>(root: fn(P) -> Element, props: P) -> String {
    let mut dom = VirtualDom::new_with_props(root, props);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

// ── One set of props, handed to every widget ────────────────────────────
//
// Everything a widget can be given, so a part a widget drops shows up as a
// missing substring: a label, an error (so the invalid classes and the owned
// `aria-` attributes appear), a field attribute and an author one.

fn field_attrs() -> FieldAttrs {
    FieldAttrs::from([
        (AttrKey::Std(Attr::MinLength), AttrValue::Int(2)),
        (AttrKey::Std(Attr::Required), AttrValue::Flag),
    ])
}

fn extra_attrs() -> Vec<Attribute> {
    vec![Attribute::new("autocomplete", "off", None, false)]
}

fn errors() -> Vec<ValidationMessage> {
    vec![ValidationMessage::from("too short")]
}

fn values() -> HashMap<String, String> {
    HashMap::from([(PATH.to_string(), "A".to_string())])
}

fn through_widget((widget, choices): (AnyWidget, Option<Vec<Choice>>)) -> Element {
    let values = use_store(values);
    widget.render(WidgetProps {
        id: ID.into(),
        path: PATH.into(),
        label: Some("Title".into()),
        errors: errors(),
        attrs: field_attrs(),
        extra_attrs: extra_attrs(),
        values,
        classes: Rc::new(Classes::fx()),
        choices,
    })
}

// ── The text-like inputs ─────────────────────────────────────────────────

/// Each piece of the props reaches the markup.
#[gtest]
fn each_text_like_renders_every_prop() {
    for (widget, input_type) in text_likes() {
        let html = html_of(through_widget, (widget.clone(), None));
        expect_that!(
            html,
            all![
                contains_substring(format!(r#"type="{}""#, input_type.html_type())),
                contains_substring(r#"value="A""#),
                contains_substring("Title"),
                contains_substring("too short"),
                contains_substring("minlength=2"),
                contains_substring(r#"autocomplete="off""#),
                contains_substring("fx-invalid"),
            ],
            "{}",
            widget.name()
        );
    }
}

/// Bare: nothing a user sees, so nothing to label or mark invalid.
#[gtest]
fn hidden_input_renders_bare() {
    let html = html_of(through_widget, (HiddenInput.into(), None));
    expect_that!(
        html,
        all![
            contains_substring(r#"type="hidden""#),
            contains_substring(r#"value="A""#),
            not(contains_substring("Title")),
            not(contains_substring("too short")),
            not(contains_substring("<label")),
        ]
    );
}

#[gtest]
fn checkbox_and_textarea_render_every_prop() {
    for (widget, element) in [
        (AnyWidget::from(CheckboxInput), r#"type="checkbox""#),
        (Textarea.into(), "<textarea"),
    ] {
        expect_that!(
            html_of(through_widget, (widget.clone(), None)),
            all![
                contains_substring(element),
                contains_substring("Title"),
                contains_substring("too short"),
                contains_substring(r#"autocomplete="off""#),
                contains_substring("fx-invalid"),
            ],
            "{}",
            widget.name()
        );
    }
}

/// `NAME` is `form!`'s spelling, so a `classes_plus:` selector keeps working
/// once `Classes` is keyed on it.
#[gtest]
fn each_builtin_is_named_as_form_spells_it() {
    expect_that!(Textarea::NAME, eq("textarea"));
    expect_that!(TextInput::NAME, eq("text"));
    expect_that!(TelInput::NAME, eq("tel"));
    expect_that!(DatetimeLocalInput::NAME, eq("datetime_local"));
}

/// What a text `<input>` takes, less what formoxus owns.
#[gtest]
fn text_input_extends_a_text_inputs_attributes() {
    let extends = TextInput::EXTENDS;
    expect_that!(extends.contains(Attr::Placeholder), eq(true));
    expect_that!(extends.contains(Attr::MinLength), eq(true));
    expect_that!(extends.contains(Attr::Rows), eq(false), "a textarea's");
    expect_that!(extends.contains(Attr::Step), eq(false), "a number input's");
    expect_that!(extends.contains(Attr::Required), eq(false), "owned");
}

/// The reason W-C gives each input type its own Rust type: what an `<input>`
/// takes depends on its `type`.
#[gtest]
fn each_input_type_extends_its_own_attributes() {
    expect_that!(NumberInput::EXTENDS.contains(Attr::Step), eq(true));
    expect_that!(NumberInput::EXTENDS.contains(Attr::MinLength), eq(false));
    expect_that!(ColorInput::EXTENDS.contains(Attr::Alpha), eq(true));
    expect_that!(TextInput::EXTENDS.contains(Attr::Alpha), eq(false));
    expect_that!(DateInput::EXTENDS.contains(Attr::Min), eq(true));
    expect_that!(EmailInput::EXTENDS.contains(Attr::Pattern), eq(true));
    expect_that!(NumberInput::EXTENDS.contains(Attr::Pattern), eq(false));
    // Valid on an email input, but owned (a leaf holds one string).
    expect_that!(EmailInput::EXTENDS.contains(Attr::Multiple), eq(false));
}

/// Empty, because `InputView` spreads nothing onto a hidden input: anything
/// written for one would be dropped, so `form!` should refuse it.
#[gtest]
fn hidden_input_extends_nothing() {
    expect_that!(HiddenInput::EXTENDS, eq(AttrSet::EMPTY));
}

#[gtest]
fn checkbox_and_textarea_extend_their_own_elements() {
    expect_that!(Textarea::EXTENDS.contains(Attr::Rows), eq(true));
    expect_that!(Textarea::EXTENDS.contains(Attr::MaxLength), eq(true));
    expect_that!(Textarea::EXTENDS.contains(Attr::Pattern), eq(false));
    expect_that!(
        CheckboxInput::EXTENDS.contains(Attr::Placeholder),
        eq(false)
    );
    expect_that!(CheckboxInput::EXTENDS.contains(Attr::Disabled), eq(true));
}

// ── The choosers ─────────────────────────────────────────────────────────

fn states() -> Vec<Choice> {
    vec![Choice::new("NY", "New York"), Choice::new("OH", "Ohio")]
}

/// The field's choices reach the widget through `WidgetProps`, and win over
/// a bool's true/false.
#[gtest]
fn each_chooser_renders_the_choices_it_is_given() {
    for widget in [AnyWidget::from(Select), RadioGroup.into()] {
        let html = html_of(through_widget, (widget.clone(), Some(states())));
        expect_that!(
            html,
            all![
                contains_substring(r#"value="NY""#),
                contains_substring("Ohio"),
                not(contains_substring(r#"value="true""#)),
            ],
            "{}",
            widget.name()
        );
    }
}

/// With no list, a chooser over a bool offers true and false.
#[gtest]
fn each_chooser_falls_back_to_a_bools_choices() {
    for widget in [AnyWidget::from(Select), RadioGroup.into()] {
        expect_that!(
            html_of(through_widget, (widget.clone(), None)),
            all![
                contains_substring(r#"value="true""#),
                contains_substring(r#"value="false""#),
            ],
            "{}",
            widget.name()
        );
    }
}

#[gtest]
fn the_choosers_extend_their_own_elements() {
    expect_that!(Select::EXTENDS.contains(Attr::Size), eq(true));
    expect_that!(Select::EXTENDS.contains(Attr::Placeholder), eq(false));
    // The spread lands on the fieldset, so a radio's attributes are not here.
    expect_that!(RadioGroup::EXTENDS.contains(Attr::Disabled), eq(true));
    expect_that!(RadioGroup::EXTENDS.contains(Attr::Checked), eq(false));
}
