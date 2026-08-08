use dioxus::prelude::*;

use crate::error::FieldError;
use crate::fields::{FormField, FormFieldStoreExt};

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
pub trait FieldWidget<T: Clone + 'static> {
    fn render(field: Store<FormField<T>>, props: FieldProps) -> Element;
}

/// The default widget for a field value type — "the default widget per kind".
///
/// The orphan rule puts these impls wherever the *type* is defined: formoxus
/// ships them for the std types it renders (`String`, …); an app implements it
/// for types it owns. A foreign type (from another crate) can still get a widget
/// via an explicit `#[form(component = …)]` override — just not a *default*.
pub trait DefaultWidget: Clone + 'static {
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

/// The default single-line text input, for `String` fields.
pub struct TextInput;

impl FieldWidget<String> for TextInput {
    fn render(field: Store<FormField<String>>, props: FieldProps) -> Element {
        rsx! { TextInputWidget { field, props } }
    }
}

#[component]
fn TextInputWidget(field: Store<FormField<String>>, props: FieldProps) -> Element {
    let FieldProps {
        label,
        required,
        placeholder,
    } = props;
    let mut value = field.value();
    let current = value.cloned().unwrap_or_default();
    rsx! {
        label {
            "{label}"
            if required { span { class: "required", " *" } }
            input {
                r#type: "text",
                required,
                value: "{current}",
                placeholder,
                oninput: move |e| value.set(Some(e.value())),
            }
            FieldErrors { errors: field.errors().cloned() }
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
        rsx! { NumberInputWidget { field, props } }
    }
}

#[component]
fn NumberInputWidget(field: Store<FormField<i32>>, props: FieldProps) -> Element {
    let FieldProps {
        label,
        required,
        placeholder,
    } = props;
    let mut value = field.value();
    // A number input reports `.value` as "" for empty *or* invalid input, so
    // `parse().ok()` collapses both to `None` — no un-storable "raw invalid" state.
    let current = value.cloned().map(|n| n.to_string()).unwrap_or_default();
    rsx! {
        label {
            "{label}"
            if required { span { class: "required", " *" } }
            input {
                r#type: "number",
                step: 1,
                required,
                value: "{current}",
                placeholder,
                oninput: move |e| value.set(e.value().parse::<i32>().ok()),
            }
            FieldErrors { errors: field.errors().cloned() }
        }
    }
}

impl DefaultWidget for i32 {
    type Widget = NumberInput;
}

pub struct CheckboxInput;

impl FieldWidget<bool> for CheckboxInput {
    fn render(field: Store<FormField<bool>>, props: FieldProps) -> Element {
        rsx! { CheckboxWidget { field, props } }
    }
}

#[component]
fn CheckboxWidget(field: Store<FormField<bool>>, props: FieldProps) -> Element {
    let label = props.label;
    let mut value = field.value();
    let current = value.cloned().unwrap_or(false);
    rsx! {
        label {
            "{label}"
            input {
                r#type: "checkbox",
                checked: current,
                onchange: move |_| value.set(Some(!current)),
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
        rsx! { SelectWidget { field, props, choices: vec![
            SelectChoice { value: true, display: "True".to_string() },
            SelectChoice { value: false, display: "False".to_string() },
        ] } }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectChoice<T> {
    pub value: T,
    pub display: String,
}

#[component]
pub fn SelectWidget<T: 'static + Clone + PartialEq>(
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
    rsx! {
        label {
            "{label}"
            if required { span { class: "required", " *" } },
            select {
                required,
                onchange: move |evt| {
                    let v = evt.value();
                    if v.is_empty() {
                        field.value().set(None);
                    } else if let Ok(i) = v.parse::<usize>() {
                        field.value().set(choices().get(i).map(|opt| opt.value.clone()));
                    }
                },
                if required && field.value().is_none() {
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
                    option {
                        value: "",
                        selected: field.value().is_none(),
                        "{none_label}"
                    }
                }
                for (i, opt) in choices().into_iter().enumerate() {
                    option {
                        value: "{i}",
                        selected: field.value().cloned() == Some(opt.value.clone()),
                        "{opt.display}"
                    }
                }
            }
            FieldErrors { errors: field.errors().cloned() }
        }
    }
}
