use dioxus::prelude::*;
use formoxus_spec::{AttrSet, FieldType, FieldTypeSet, HtmlElement, InputType, Tag, WidgetName};

use crate::widgets::{LeafWidget, Widget, WidgetProps};

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
    todo!()
}
