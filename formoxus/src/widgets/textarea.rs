//! The `<textarea>`.

use dioxus::prelude::*;
use formoxus_spec::{
    Attr, AttrSet, BuiltinWidget, FieldControl, FieldType, FieldTypeSet, HtmlElement, Tag,
};

use super::field_row::{LabelSide, field_row};
use super::values::{get_current, write_value};
use crate::widgets::{Widget, widget_props::WidgetProps};

/// `<textarea>`, a multi-line text input. For a `String` field only.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Textarea;

impl Widget for Textarea {
    const NAME: &'static str = BuiltinWidget::Textarea.form_name();

    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Control(FieldControl::Textarea));

    const PARTS: &'static [Tag] = &[Tag::Div, Tag::Label, Tag::Textarea];

    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::Text);

    const ACCEPTS_OPTIONAL: bool = true;

    const TAKES_CHOICES: bool = false;

    fn render(&self, props: WidgetProps) -> Element {
        rsx! {
            TextareaView { widget_props: props }
        }
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
    let values = props.values;
    let path = props.path.clone();
    let current = get_current(&path, values);
    let attrs: Vec<Attribute> = props.attrs.merge_with_attrs(props.extra_attrs.clone());

    let control = rsx! {
        textarea {
            class: props.control_class::<Textarea>(Tag::Textarea),
            id: "{props.id}",
            name: "{path}",
            value: "{current}",
            oninput: move |e: FormEvent| {
                let raw = e.value();
                write_value(&path, values, raw);
            },
            // TODO: delete pattern, since not allowed in HTML
            ..attrs,
        }
    };
    let marked = props.attrs.contains(Attr::Required);
    field_row::<Textarea>(&props, marked, LabelSide::Before, control)
}
