use dioxus::prelude::*;
use formoxus_spec::{
    Attr, AttrSet, FieldType, FieldTypeSet, HtmlElement, InputType, Tag, WidgetName,
};

use crate::widgets::{LabelSide, LeafWidget, Widget, WidgetProps, field_row};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct FileInput;

impl Widget for FileInput {
    const NAME: &'static str = WidgetName::FileInput.form_name();

    const PARTS: &'static [Tag] = &[Tag::Div, Tag::Label, Tag::Input, Tag::Output];

    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Input(InputType::File));
}

impl LeafWidget for FileInput {
    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::File);

    const ACCEPTS_OPTIONAL: bool = true;

    const TAKES_CHOICES: bool = false;

    fn render(&self, props: WidgetProps) -> Element {
        rsx! {
            FileInputView { widget_props: props }
        }
    }
}

#[component]
pub(crate) fn FileInputView(widget_props: WidgetProps) -> Element {
    let props = widget_props;
    let handle = props.field_handle();
    let current = handle.current();
    let attrs: Vec<Attribute> = props
        .current_attrs()
        .merge_with_attrs(props.extra_attrs.clone());

    let control = rsx! {
        input {
            class: props.control_class::<FileInput>(Tag::Input),
            id: "{props.id}",
            name: "{props.path}",
        }
    };
    let marked = props.base_attrs.contains(Attr::Required);
    field_row::<FileInput>(&props, marked, LabelSide::Before, control)
}
