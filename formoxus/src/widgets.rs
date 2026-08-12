use std::fmt::{Debug, Display};
use std::str::FromStr;

use dioxus::prelude::*;

use crate::error::{FieldError, try_from};
use crate::fields::{FieldValue, FormField, FormFieldStoreExt};

/// Presentational config a form hands a widget when rendering a field — the bits
/// that come from the declaration (`#[form(label = …)]`, required-ness), not the
/// value or its errors (those live in the [`FormField`]).
#[derive(Clone, PartialEq)]
pub struct FieldProps {
    pub label: String,
    pub required: bool,
    pub placeholder: Option<String>,
}

/// A widget that knows how to render a [`FormField`] of value type `T`: its input
/// (bound to the value lens) and its errors. Implemented on a marker type (e.g.
/// [`TextInput`]); a field selects one by naming its type.
pub trait FieldWidget<T: Clone + Debug + FromStr + 'static> {
    fn render(field: Store<FormField<T>>, props: FieldProps) -> Element;
}

/// The default widget for a field value type — "the default widget per kind".
///
/// The orphan rule puts these impls wherever the *type* is defined: formoxus
/// ships them for the std types it renders (`String`, …); an app implements it
/// for types it owns. A foreign type (from another crate) can still get a widget
/// via an explicit `#[form(component = …)]` override — just not a *default*.
pub trait DefaultWidget: Clone + Debug + FromStr + 'static {
    type Widget: FieldWidget<Self>;
}

/// Render a field with its value type's default widget.
pub fn render_default<T: DefaultWidget>(field: Store<FormField<T>>, props: FieldProps) -> Element {
    <T::Widget as FieldWidget<T>>::render(field, props)
}

/// A field's current errors as a list (renders nothing when there are none).
/// Shared by every built-in widget.
#[component]
pub fn FieldErrors(errors: Vec<FieldError>) -> Element {
    rsx! {
        if !errors.is_empty() {
            ul { class: "field-errors",
                for error in errors.iter() {
                    li { class: "field-error", "{error.0}" }
                }
            }
        }
    }
}

#[component]
pub fn InputWidget<T>(input_type: String, field: Store<FormField<T>>, props: FieldProps) -> Element 
where T: 'static + Clone + Debug + Default + FromStr + Display, T::Err: Display {
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

/// The default numeric input, for `i32` fields.
pub struct NumberInput;

impl FieldWidget<i32> for NumberInput {
    fn render(field: Store<FormField<i32>>, props: FieldProps) -> Element {
        rsx! {
            InputWidget::<i32> { input_type: "number", field, props }
        }
    }
}

impl DefaultWidget for i32 {
    type Widget = NumberInput;
}

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
