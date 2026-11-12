use dioxus::prelude::*;
use formoxus_spec::{AttrSet, HtmlElement, Tag};

use crate::{
    nodes::{Edit, ItemValue},
    widgets::{
        FieldErrors, SetWidget, SetWidgetProps, Widget, described_by, field_help, required_marker,
    },
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CheckboxMultiple;

impl Widget for CheckboxMultiple {
    const NAME: &'static str = "checkbox_multiple";

    const PARTS: &'static [Tag] = &[Tag::Fieldset, Tag::Legend, Tag::Div, Tag::Input, Tag::Label];

    const EXTENDS: AttrSet = AttrSet::valid_on(HtmlElement::Fieldset);
}

impl SetWidget for CheckboxMultiple {
    fn render(&self, props: SetWidgetProps) -> Element {
        rsx! {
            CheckboxMultipleView { widget_props: props }
        }
    }
}

#[component]
pub(crate) fn CheckboxMultipleView(widget_props: SetWidgetProps) -> Element {
    let props = widget_props;

    let field_class = props.part_class::<CheckboxMultiple>(Tag::Fieldset);
    let legend_class = props.part_class::<CheckboxMultiple>(Tag::Legend);
    let div_class = props.part_class::<CheckboxMultiple>(Tag::Div);
    let input_class = props.part_class::<CheckboxMultiple>(Tag::Input);
    let label_class = props.part_class::<CheckboxMultiple>(Tag::Label);
    let choices = props.choices.iter().enumerate().map(|(index, choice)| {
        let is_checked = props.chosen.contains(&choice.value);
        let chosen_index = props.chosen.iter().position(|e| *e == choice.value);
        let is_disabled =
            !is_checked && props.max_items.is_some_and(|max| props.chosen.len() >= max);
        let path = props.path.clone();
        let value = choice.value.clone();
        let on_edit = props.on_edit;
        let id = format!("{}-{index}", props.id);
        let on_change = move |e: FormEvent| {
            if e.checked() {
                on_edit.call(Edit::AddItem {
                    path: path.clone(),
                    before: None,
                    value: Some(ItemValue::raw(value.clone())),
                });
            } else {
                on_edit.call(Edit::RemoveItem {
                    path: path.clone(),
                    index: chosen_index
                        .expect("an unticked box was ticked, so should be in chosen"),
                });
            }
        };
        rsx! {
            div { class: &div_class,
                input { class: &input_class,
                    r#type: "checkbox",
                    id: "{id}",
                    value: "{choice.value}",
                    checked: is_checked,
                    disabled: is_disabled,
                    onchange: on_change,
                }
                label { class: &label_class,
                    r#for: "{id}",
                    "{choice.display}"
                }
            }
        }
    });
    let choices: Vec<Element> = choices.collect();

    rsx! {
        fieldset {
            class: field_class,
            aria_describedby: described_by(
                &props.id,
                props.help_text.is_some(),
                props.is_invalid()
            ),
            legend {
                class: legend_class,
                if let Some(text) = props.label.clone() {
                    "{text}"
                    if props.required {
                        {required_marker(&props.classes)}
                    }
                }
            }
            {field_help(&props.id, props.help_text.as_deref(), &props.classes)}
            {choices.into_iter()}
            FieldErrors {
                field_id: props.id.clone(),
                paths: vec![props.path.clone()],
                errors: props.errors,
                classes: props.classes.clone(),
            }
        }
    }
}
