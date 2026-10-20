//! The built-in widget types (`FIELDS_PLAN.md` Step 5, W-C): each wraps
//! today's component, so for the same props it must render exactly what
//! `ScalarWidget` renders for the matching `BuiltinWidget`. Nothing builds one
//! from a `form!` yet, so these drive `Widget::render` through `AnyWidget`
//! directly, the way `FieldNode` will.

use super::render_to_html;
use dioxus::core::Attribute;
use dioxus::prelude::*;
use formoxus::ValidationMessage;
use formoxus::attrs::{Attr, AttrKey, AttrValue, BuiltinWidget, FieldType, RenderedInputType};
use formoxus::classes::Classes;
use formoxus::field_attrs::FieldAttrs;
use formoxus::widgets::widget_props::WidgetProps;
use formoxus::widgets::{AnyWidget, FieldProps, ScalarWidget, TextInput, Widget, WidgetType};
use googletest::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;

const ID: &str = "rand0123-title";
const PATH: &str = "title";

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

#[component]
fn ThroughTextInput() -> Element {
    let values = use_store(values);
    AnyWidget::new(TextInput).render(WidgetProps {
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

#[component]
fn ThroughScalarWidget() -> Element {
    let values = use_store(values);
    rsx! {
        ScalarWidget {
            field_type: FieldType::Text,
            field_attrs: field_attrs(),
            widget: WidgetType::Builtin(BuiltinWidget::Input(RenderedInputType::Text)),
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

// ── `TextInput` ──────────────────────────────────────────────────────────

/// The wrapper adds nothing and drops nothing.
#[gtest]
fn text_input_renders_what_scalar_widget_renders() {
    let html = render_to_html(ThroughTextInput);
    // Not two empty strings agreeing: each piece of the props is there.
    expect_that!(
        html,
        all![
            contains_substring(r#"type="text""#),
            contains_substring(r#"value="A""#),
            contains_substring("Title"),
            contains_substring("too short"),
            contains_substring("minlength=2"),
            contains_substring(r#"autocomplete="off""#),
            contains_substring("fx-invalid"),
        ]
    );
    expect_that!(html, eq(&render_to_html(ThroughScalarWidget)));
}

/// `default_classes` repeats `Classes::fx()`'s slots for a text input until
/// `Classes` is keyed on `Widget::NAME`; this catches the two drifting.
#[gtest]
fn text_input_default_classes_match_classes_fx() {
    let widget = BuiltinWidget::Input(RenderedInputType::Text);
    let fx = Classes::fx();
    let slots = TextInput::default_classes();
    for (part, slot) in &slots {
        expect_that!(fx.part(widget, part), some(eq(slot)), "part `{part}`");
    }
    expect_that!(slots.len(), eq(widget.parts().len()), "one slot per part");
}

#[gtest]
fn text_input_is_named_as_form_spells_it() {
    expect_that!(TextInput::NAME, eq("text"));
    expect_that!(AnyWidget::new(TextInput).name(), eq("text"));
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
