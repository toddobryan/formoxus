use dioxus::prelude::*;

use crate::{FieldError, FormField, FormFieldStoreExt};

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
pub fn render_default<T: DefaultWidget>(
    field: Store<FormField<T>>,
    props: FieldProps,
) -> Element {
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
