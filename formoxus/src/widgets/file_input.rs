use dioxus::prelude::*;
use formoxus_spec::{
    Attr, AttrSet, FieldType, FieldTypeSet, FixedKey, HtmlElement, InputType, Tag, WidgetName,
};

use crate::widgets::{
    LabelSide, LeafWidget, Widget, WidgetProps, field_row,
    icons::{X, icon},
};

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
    let field = props.field();
    let remover = field.clone();
    let file_name = &field.current();
    let output = if file_name.is_empty() {
        ""
    } else {
        match field.picked_file() {
            Some(_) => file_name,
            None => &format!("Current file: {file_name}"),
        }
    };
    let required = props.base_attrs.contains(Attr::Required);

    let attrs: Vec<Attribute> = props
        .current_attrs()
        .merge_with_attrs(props.extra_attrs.clone());

    // The reset count as the input's key, so a reset makes a new, empty
    // element: the only way to clear what a file input shows. From a loop of
    // one, because Dioxus ignores a key on an element nested in a template.
    // (On this block's first root it would be read, since `field_row` places
    // `control` with `{…}`, but it would make the output and Remove again too,
    // and break if the input stopped being first.)
    let resets = field.resets();

    let control = rsx! {
        for n in [resets] {
            input {
                key: "{n}",
                class: props.control_class::<FileInput>(Tag::Input),
                r#type: "file",
                name: "{props.path()}",
                id: "{props.id()}",
                onchange: field.on_pick_file(),
                ..attrs.clone(),
            }
        }
        output {
            class: props.part_class::<FileInput>(Tag::Output),
            "for": "{props.id()}",
            "{output}"
        }
        if !required && !file_name.is_empty() {
            button {
                r#type: "button",
                class: props.classes().fixed_class(FixedKey::RemoveFileButton, false),
                aria_label: "Remove \"{file_name}\"",
                onclick: move |_| {
                    remover.pick_file(None);
                },
                {icon(X)}
            }
        }
    };

    field_row::<FileInput>(&props, required, LabelSide::Before, control)
}
