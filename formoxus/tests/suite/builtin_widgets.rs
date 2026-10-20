//! The built-in widget types (`FIELDS_PLAN.md` Step 5, W-C): each wraps
//! today's component, so for the same props it must render exactly what
//! `ScalarWidget` renders for the matching `BuiltinWidget`. Nothing builds one
//! from a `form!` yet, so these drive `Widget::render` through `AnyWidget`
//! directly, the way `FieldNode` will.

use dioxus::core::Attribute;
use dioxus::prelude::*;
use formoxus::ValidationMessage;
use formoxus::attrs::{Attr, AttrKey, AttrValue, BuiltinWidget, FieldType, RenderedInputType as R};
use formoxus::classes::Classes;
use formoxus::field_attrs::FieldAttrs;
use formoxus::widgets::widget_props::WidgetProps;
use formoxus::widgets::{
    AnyWidget, ColorInput, DateInput, DatetimeLocalInput, EmailInput, FieldProps, MonthInput,
    NumberInput, PasswordInput, ScalarWidget, SearchInput, TelInput, TextInput, TimeInput,
    UrlInput, WeekInput, Widget, WidgetType,
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

/// SSR of `root` given `props`: `render_to_html`, for a root that takes props,
/// so one root serves every widget in the table.
fn html_of<P: Clone + 'static>(root: fn(P) -> Element, props: P) -> String {
    let mut dom = VirtualDom::new_with_props(root, props);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

// ── One set of props, handed to both paths ──────────────────────────────
//
// Everything a widget can be given, so a part the wrapper drops shows up as
// a difference: a label, an error (so the invalid classes and the owned
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

#[expect(
    clippy::needless_pass_by_value,
    reason = "a root component takes its props by value"
)]
fn through_widget(widget: AnyWidget) -> Element {
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
    })
}

fn through_scalar_widget(input_type: R) -> Element {
    let values = use_store(values);
    rsx! {
        ScalarWidget {
            field_type: FieldType::Text,
            field_attrs: field_attrs(),
            widget: WidgetType::Builtin(BuiltinWidget::Input(input_type)),
            values,
            props: FieldProps {
                id: ID.into(),
                path: PATH.into(),
                label: Some("Title".into()),
                classes: Rc::new(Classes::fx()),
                errors: errors(),
            },
            attrs: extra_attrs(),
        }
    }
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

/// The wrapper adds nothing and drops nothing.
#[gtest]
fn each_text_like_renders_what_scalar_widget_renders() {
    for (widget, input_type) in text_likes() {
        let html = html_of(through_widget, widget.clone());
        // Not two empty strings agreeing: each piece of the props is there.
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
        expect_that!(
            html,
            eq(&html_of(through_scalar_widget, input_type)),
            "{}",
            widget.name()
        );
    }
}

/// `default_classes` repeats `Classes::fx()`'s slots until `Classes` is keyed
/// on `Widget::NAME`; this catches the two drifting.
#[gtest]
fn each_text_likes_default_classes_match_classes_fx() {
    let fx = Classes::fx();
    for (widget, input_type) in text_likes() {
        let builtin = BuiltinWidget::Input(input_type);
        let slots = widget.default_classes();
        for (part, slot) in &slots {
            expect_that!(
                fx.part(builtin, part),
                some(eq(slot)),
                "{}'s `{part}`",
                widget.name()
            );
        }
        expect_that!(
            slots.len(),
            eq(builtin.parts().len()),
            "{}: one slot per part",
            widget.name()
        );
    }
}

/// `NAME` is `form!`'s spelling, so a `classes_plus:` selector keeps working
/// once `Classes` is keyed on it.
#[gtest]
fn each_text_like_is_named_as_form_spells_it() {
    for (widget, input_type) in text_likes() {
        expect_that!(
            widget.name(),
            eq(BuiltinWidget::Input(input_type).form_name())
        );
    }
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
