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

/// A field's current errors (renders nothing when there are none).
/// Shared by every built-in widget.
///
/// **The `small` is a DEFAULT, not a commitment.** It is chosen so that Pico
/// styles this for free: Pico is classless, and its validation rule is
/// `input[aria-invalid="true"] + small`, which colours the message with the
/// theme's own `--pico-del-color` in light and dark alike. Meeting that rule is
/// why the element is a `small`, why it must be the control's IMMEDIATE next
/// sibling, and why several errors share ONE `small` as spans rather than
/// getting a `small` apiece — only the first sibling would match, and the rest
/// would render as ordinary body text.
///
/// Anyone not using Pico pays almost nothing for that choice. Formoxus ships no
/// stylesheet at all; `field-errors`/`field-error` are its own class names, so a
/// Bootstrap or Tailwind user styles them like any other markup. What they
/// inherit is the element name, and the only real cost is semantic — `small`
/// means "fine print", which an error message arguably is not. **When
/// error rendering becomes overridable (the same mechanism as custom widgets),
/// this is the component to swap**, and this choice stops being global.
///
/// The `aria-invalid` half lives on each widget's control and is NOT a Pico
/// dependency: it is the W3C ARIA attribute assistive technology reads to
/// announce a field as errored. Set it whatever CSS the consumer brings —
/// leaving it off is an accessibility defect, not a styling preference.
#[component]
pub fn FieldErrors(errors: Vec<FieldError>) -> Element {
    rsx! {
        if !errors.is_empty() {
            small { class: "field-errors",
                for error in errors.iter() {
                    span { class: "field-error", "{error.0}" }
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
