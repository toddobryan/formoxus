//! The built-in widget types (`FIELDS_PLAN.md` Step 5, W-C), driven through
//! `Widget::render` on an `AnyWidget` directly, the way `FieldNode` does, so
//! each is tested apart from the node that hands it its props.

use dioxus::core::Attribute;
use dioxus::prelude::*;
use formoxus::ValidationMessage;
use formoxus::attrs::AttrSet;
use formoxus::attrs::{Attr, AttrKey, AttrValue, BuiltinWidget, RenderedInputType as R};
use formoxus::classes::Classes;
use formoxus::field_attrs::FieldAttrs;
use formoxus::widgets::Choice;
use formoxus::widgets::widget_props::WidgetProps;
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

/// Every text-like widget type, beside the `BuiltinWidget` input it stands in
/// for. A type added to the macro and not here goes untested, so the length
/// is pinned against `BuiltinWidget::ALL` below.
fn text_likes() -> Vec<(AnyWidget, R)> {
    vec![
        (TextInput.into(), R::Text),
        (PasswordInput.into(), R::Password),
        (NumberInput.into(), R::Number),
        (EmailInput.into(), R::Email),
        (TelInput.into(), R::Telephone),
        (UrlInput.into(), R::Url),
        (SearchInput.into(), R::Search),
        (ColorInput.into(), R::Color),
        (DateInput.into(), R::Date),
        (TimeInput.into(), R::Time),
        (DatetimeLocalInput.into(), R::DatetimeLocal),
        (MonthInput.into(), R::Month),
        (WeekInput.into(), R::Week),
    ]
}

/// Every built-in widget type, beside the `BuiltinWidget` it stands in for.
fn builtins() -> Vec<(AnyWidget, BuiltinWidget)> {
    let mut all: Vec<_> = text_likes()
        .into_iter()
        .map(|(w, t)| (w, BuiltinWidget::Input(t)))
        .collect();
    all.extend([
        (HiddenInput.into(), BuiltinWidget::Input(R::Hidden)),
        (CheckboxInput.into(), BuiltinWidget::Input(R::Checkbox)),
        (Textarea.into(), BuiltinWidget::Textarea),
        (Select.into(), BuiltinWidget::Select),
        (RadioGroup.into(), BuiltinWidget::RadioGroup),
    ]);
    all
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

/// Every text-like `RenderedInputType` has a widget type in the table: all
/// the inputs `BuiltinWidget::ALL` has, less hidden and checkbox.
#[gtest]
fn every_text_like_input_has_a_widget_type() {
    let inputs = BuiltinWidget::ALL
        .iter()
        .filter(|w| matches!(w, BuiltinWidget::Input(t) if !matches!(t, R::Hidden | R::Checkbox)))
        .count();
    expect_that!(text_likes().len(), eq(inputs));
}

/// Each piece of the props reaches the markup.
#[gtest]
fn each_text_like_renders_every_prop() {
    for (widget, input_type) in text_likes() {
        let html = html_of(through_widget, (widget.clone(), None));
        expect_that!(
            html,
            all![
                contains_substring(format!(
                    r#"type="{}""#,
                    input_type.to_input_type().html_type()
                )),
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

/// `default_classes` repeats `Classes::fx()`'s slots until `Classes` is keyed
/// on `Widget::NAME`; this catches the two drifting.
///
/// Part by part, so a part `fx()` gives no class (a hidden input's) must have
/// no slot either, and a slot for a part the widget lacks fails too.
#[gtest]
fn each_builtins_default_classes_match_classes_fx() {
    let fx = Classes::fx();
    for (widget, builtin) in builtins() {
        let slots = widget.default_classes();
        let parts: Vec<&str> = builtin.parts().iter().map(|p| p.name).collect();
        for part in &parts {
            let mine = slots.iter().find(|(p, _)| p == part).map(|(_, s)| s);
            expect_that!(
                mine,
                eq(fx.part(builtin, part)),
                "{}'s `{part}`",
                widget.name()
            );
        }
        for (part, _) in &slots {
            expect_that!(
                parts,
                contains(eq(part)),
                "{} has no `{part}`",
                widget.name()
            );
        }
    }
}

/// `NAME` is `form!`'s spelling, so a `classes_plus:` selector keeps working
/// once `Classes` is keyed on it.
#[gtest]
fn each_builtin_is_named_as_form_spells_it() {
    for (widget, builtin) in builtins() {
        expect_that!(widget.name(), eq(builtin.form_name()));
    }
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
