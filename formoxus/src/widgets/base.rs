use std::fmt::Debug;
use std::str::FromStr;

use dioxus::core::Element;
use dioxus::prelude::*;

use crate::error::{FieldError, FormError};
use crate::fields::FormField;
use crate::form::Provider;

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

/// A widget that additionally needs externally-supplied data to render — e.g. a
/// picker's list of choices — via `#[form(component = ..., provided)]`.
/// `Choices` is whatever shape *this widget* needs; it's the widget's call, not
/// the field's value type, since a `Ref<Source>` field's picker needs a
/// `Vec<SourcePath>` of candidates, not a `Ref<Source>` itself. The caller
/// supplies the [`Provider`] at the `store.render(handlers, providers)` call
/// site — see [`crate::form::Provider`].
pub trait ProvidedWidget<T: Clone + Debug + FromStr + 'static> {
    type Choices: 'static;
    fn render(
        field: Store<FormField<T>>,
        props: FieldProps,
        provide: Provider<Self::Choices>,
    ) -> Element;
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
pub fn FormErrors(errors: Vec<FormError>) -> Element {
    rsx! {
        if !errors.is_empty() {
            ul { class: "form-errors",
                for error in errors.iter() {
                    li { class: "form-error", "{error.0}"}
                }
            }
        }
    }
}
