//! The `<textarea>`.

use dioxus::prelude::*;
use formoxus_spec::{Attr, AttrSet, BuiltinWidget, FieldControl, HtmlElement};

use super::errors::FieldErrors;
use super::label::field_label;
use super::values::{get_current, write_value};
use crate::classes::Slot;
use crate::widgets::{Widget, widget_props::WidgetProps};

/// `<textarea>`, a multi-line text input. For a `String` field only.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Textarea;

impl Widget for Textarea {
    const NAME: &'static str = BuiltinWidget::Textarea.form_name();

    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Control(FieldControl::Textarea));

    fn render(&self, props: WidgetProps) -> Element {
        rsx! {
            TextareaView { widget_props: props }
        }
    }

    fn default_classes() -> Vec<(&'static str, Slot)> {
        vec![
            ("div", Slot::new("fx-form-field").with_invalid("fx-invalid")),
            ("label", Slot::new("fx-field-label")),
            ("textarea", Slot::new("fx-control fx-textarea")),
        ]
    }
}

/// A multi-line text input bound to one path in the value map.
///
/// Otherwise identical to `InputView` — same controlled-value binding, same
/// `aria-invalid` rule, same label layout. There is no hidden-input branch to
/// mirror: `Textarea` is only ever chosen as an override on a `Text` field, and
/// nothing here needs the extra `InputType` cases (`Password`'s masking,
/// `Hidden`'s bare markup) that make `Input` carry one.
#[component]
pub(crate) fn TextareaView(widget_props: WidgetProps) -> Element {
    // Not `props`: Dioxus reads a component's lone `props` parameter as its
    // whole props struct, which `WidgetProps` is not.
    let props = widget_props;
    let my_widget = BuiltinWidget::Textarea;
    let is_invalid = props.is_invalid();

    let class = props
        .attrs
        .class(props.classes.part(my_widget, "textarea"), is_invalid);
    let attrs: Vec<Attribute> = props.attrs.merge_with_attrs(props.extra_attrs);

    let field_class = props
        .classes
        .part_class(BuiltinWidget::Textarea, "div", is_invalid);

    let values = props.values;
    let current = get_current(&props.path, values);

    let label = field_label(
        &props.id,
        props.label,
        props.attrs.contains(Attr::Required),
        &props.classes,
        is_invalid,
        my_widget,
    );

    rsx! {
        div {
            class: field_class,
            {label}
            textarea {
                class: class,
                id: &props.id,
                name: "{props.path}",
                value: "{current}",
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&props.path, values, raw);
                },
                // TODO: delete pattern, since not allowed in HTML
                ..attrs,
            }
            FieldErrors { field_id: &props.id, errors: props.errors, classes: props.classes.clone() }
        }
    }
}
