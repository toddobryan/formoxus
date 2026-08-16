use std::fmt::Debug;
use std::str::FromStr;

use dioxus::prelude::*;

use crate::{fields::{FieldValue, FormField, FormFieldStoreExt}, widgets::{DefaultWidget, FieldErrors, FieldProps, FieldWidget}};

pub struct CheckboxInput;

impl FieldWidget<bool> for CheckboxInput {
    fn render(field: Store<FormField<bool>>, props: FieldProps) -> Element {
        rsx! {
            CheckboxWidget { field, props }
        }
    }
}

#[component]
fn CheckboxWidget(field: Store<FormField<bool>>, props: FieldProps) -> Element {
    let label = props.label;
    let mut value = field.value();
    let current = match value() {
        FieldValue::Empty => false,
        FieldValue::Valid(t) => t,
        FieldValue::Invalid { raw, error: _ } => raw == "true",
    };
    rsx! {
        label {
            "{label}"
            input {
                r#type: "checkbox",
                checked: current,
                onchange: move |_| value.set(FieldValue::Valid(!current)),
            }
            FieldErrors { errors: field.errors().cloned() }
        }

    }
}

impl DefaultWidget for bool {
    type Widget = CheckboxInput;
}

pub struct UnsetBooleanSelect;

impl FieldWidget<bool> for UnsetBooleanSelect {
    fn render(field: Store<FormField<bool>>, props: FieldProps) -> Element {
        rsx! {
            SelectWidget {
                field,
                props,
                choices: vec![
                    SelectChoice {
                        value: true,
                        display: "True".to_string(),
                    },
                    SelectChoice {
                        value: false,
                        display: "False".to_string(),
                    },
                ],
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectChoice<T> {
    pub value: T,
    pub display: String,
}

#[component]
pub fn SelectWidget<T: 'static + Clone + Debug + Default + FromStr + PartialEq>(
    field: Store<FormField<T>>,
    props: FieldProps,
    choices: ReadSignal<Vec<SelectChoice<T>>>,
) -> Element {
    let FieldProps {
        label,
        required,
        placeholder,
    } = props;
    // Label for the selectable "no value" option in an optional select (a required
    // select uses a hidden placeholder that fails validation instead). Defaults to "None".
    let none_label = placeholder.clone().unwrap_or_else(|| "None".to_string());
    let mut current = field.value();
    rsx! {
        label {
            "{label}"
            if required {
                span { class: "required", " *" }
            }
            select {
                required,
                onchange: move |evt| {
                    let v = evt.value();
                    if v.is_empty() {
                        current.set(FieldValue::Empty);
                    } else if let Ok(i) = v.parse::<usize>() {
                        current.set(FieldValue::Valid(choices().get(i).map(|opt| opt.value.clone()).unwrap_or_default()));
                    }
                },
                if required && current().is_empty() {
                    option {
                        value: "",
                        selected: true,
                        disabled: true,
                        hidden: true,
                        "{placeholder.clone().unwrap_or_default()}"
                    }
                } else if !required {
                    // Optional select: a visible, selectable "no value" option. Its empty
                    // value routes through the `is_empty()` arm of onchange → sets None.
                    option { value: "", selected: current().is_empty(), "{none_label}" }
                }
                for (i , opt) in choices().into_iter().enumerate() {
                    option {
                        value: "{i}",
                        selected: field.value()() == FieldValue::Valid(opt.value),
                        "{opt.display}"
                    }
                }
            }
            FieldErrors { errors: field.errors().cloned() }
        }
    }
}
