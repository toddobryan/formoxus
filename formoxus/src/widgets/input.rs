use std::fmt::{Debug, Display};
use std::str::FromStr;

use dioxus::prelude::*;

use crate::error::try_from;
use crate::fields::{FieldValue, FormFieldStoreExt};
use crate::widgets::{DefaultWidget, FieldErrors, FieldWidget};
use crate::{fields::FormField, widgets::FieldProps};

#[component]
pub fn InputWidget<T>(
    input_type: String,
    #[props(default)] inputmode: Option<String>,
    field: Store<FormField<T>>,
    props: FieldProps,
) -> Element
where
    T: 'static + Clone + Debug + Default + FromStr + Display,
    T::Err: Display,
{
    let FieldProps {
        label,
        required,
        placeholder,
    } = props;
    let mut value = field.value();
    let current = match value() {
        FieldValue::Empty => T::default().to_string(),
        FieldValue::Valid(t) => t.to_string(),
        FieldValue::Invalid { raw, error: _ } => raw.clone(),
    };
    rsx! {
        label {
            "{label}"
            if required {
                span { class: "required", " *" }
            }
            input {
                r#type: "{input_type}",
                inputmode,
                required,
                value: "{current}",
                placeholder,
                oninput: move |e| {
                    let trimmed = e.value().trim().to_string();
                    if trimmed.is_empty() {
                        value.set(FieldValue::Empty);
                    } else {
                        match try_from::<T>(&trimmed) {
                            Ok(parsed) => value.set(FieldValue::Valid(parsed)),
                            Err(err) => {
                                value
                                    .set(FieldValue::Invalid {
                                        raw: trimmed,
                                        error: err,
                                    })
                            }
                        }
                    }
                },
            }
            FieldErrors { errors: field.errors().cloned() }
        }
    }
}

/// The default single-line text input, for `String` fields.
pub struct TextInput;

impl FieldWidget<String> for TextInput {
    fn render(field: Store<FormField<String>>, props: FieldProps) -> Element {
        rsx! {
            InputWidget::<String> { input_type: "text", field, props }
        }
    }
}

impl DefaultWidget for String {
    type Widget = TextInput;
}

pub struct PasswordInput;

impl FieldWidget<String> for PasswordInput {
    fn render(field: Store<FormField<String>>, props: FieldProps) -> Element {
        rsx! {
            InputWidget::<String> { input_type: "password", field, props  }
        }
    }
}
