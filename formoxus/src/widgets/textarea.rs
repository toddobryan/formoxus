//! The `<textarea>`.

use dioxus::prelude::*;
use formoxus_spec::{Attr, AttrSet, BuiltinWidget, FieldControl, HtmlElement};

use super::errors::FieldErrors;
use super::label::field_label;
use super::types::FieldProps;
use super::values::{get_current, write_value};
use crate::classes::Slot;
use crate::field_attrs::FieldAttrs;
use crate::nodes::ValuesStore;
use crate::widgets::{Widget, widget_props::WidgetProps};

/// `<textarea>`, a multi-line text input. For a `String` field only.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Textarea;

impl Widget for Textarea {
    const NAME: &'static str = BuiltinWidget::Textarea.form_name();

    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Control(FieldControl::Textarea));

    fn render(&self, props: WidgetProps) -> Element {
        rsx! {
            TextareaView {
                field_attrs: props.attrs,
                values: props.values,
                props: FieldProps {
                    id: props.id,
                    path: props.path,
                    label: props.label,
                    classes: props.classes,
                    errors: props.errors,
                },
                attrs: props.extra_attrs,
            }
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
pub(crate) fn TextareaView(
    field_attrs: FieldAttrs,
    values: ValuesStore,
    props: FieldProps,
    #[props(extends = textarea)] attrs: Vec<Attribute>,
) -> Element {
    let my_widget = BuiltinWidget::Textarea;
    let is_invalid = props.is_invalid();

    let class = field_attrs.class(props.classes.part(my_widget, "textarea"), is_invalid);
    let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(attrs);

    let field_class = props
        .classes
        .part_class(BuiltinWidget::Textarea, "div", props.is_invalid());

    let current = get_current(&props.path, values);

    let label = field_label(
        &props.id,
        props.label,
        field_attrs.contains(Attr::Required),
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
