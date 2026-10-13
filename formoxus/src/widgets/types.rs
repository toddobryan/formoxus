//! What a widget IS, as opposed to what it renders: the widget
//! vocabulary `form!` speaks, and the two prop bundles every input takes.
//! This includes [`Choice`], which represents one of the legal values
//! a widget can hold.

use dioxus::prelude::*;
use formoxus_spec::BuiltinWidget;

use crate::error::ValidationMessage;
use crate::members::ValuesStore;

#[derive(Clone, Copy)]
pub enum WidgetType {
    Builtin(BuiltinWidget),
    Custom {
        name: &'static str,
        render: fn(WidgetProps) -> Element,
    },
}

// `Debug` and `PartialEq` are hand-written rather than derived, because of
// `Custom`'s `fn` pointer. `Debug` prints `form!`'s spelling (`select`, `tel`,
// `custom(MarkdownWidget)`), so a panic quotes back what the author wrote;
// deriving it would print the pointer's address beside a custom widget's name.
impl std::fmt::Debug for WidgetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Builtin(w) => write!(f, "{}", w.form_name()),
            Self::Custom { name, .. } => write!(f, "custom({name})"),
        }
    }
}

impl PartialEq for WidgetType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Builtin(a), Self::Builtin(b)) => a == b,
            // Two custom widgets are the same widget when they name the same
            // input. Comparing the `fn` pointers is what rustc warns about and
            // is genuinely meaningless here: identical functions may be merged
            // to one address, and one function may be duplicated across codegen
            // units, so neither equality nor inequality tells you anything about
            // which widget you have.
            (Self::Custom { name: a, .. }, Self::Custom { name: b, .. }) => a == b,
            _ => false,
        }
    }
}

impl From<BuiltinWidget> for WidgetType {
    fn from(value: BuiltinWidget) -> Self {
        WidgetType::Builtin(value)
    }
}

/// What a custom widget is handed: the same pair every built-in widget gets.
///
/// One struct rather than two parameters because `render` is a `fn` pointer and
/// a single argument keeps that signature stable as the boundary grows.
///
/// Note this is the widget boundary the design notes fix — `(path, label,
/// required, errors)` in `props`, plus the value store — and NOT a
/// `FormField<T>`. A widget never sees the typed field: it reads and writes raw
/// strings through `values`, exactly as `Input` does.
#[derive(Clone, Debug, PartialEq)]
pub struct WidgetProps {
    pub values: ValuesStore,
    pub props: FieldProps,
    /// Pass-through attributes for the element this widget puts them on. A
    /// custom widget decides that for itself, the same way each built-in does.
    pub attrs: Vec<Attribute>,
}

/// Defined in `formoxus-spec`, because the attribute table and `form!` both
/// key on it and `formoxus-macros` cannot reach this crate. Re-exported here so
/// that `formoxus::widgets::InputType` is still its address.
pub use formoxus_spec::InputType;

#[derive(Clone, Debug, PartialEq)]
pub struct FieldProps {
    /// The field's name in the PAGE: the control's `id`, what its `<label for>`
    /// and `aria-describedby` point at. `{form id}-{path}`, built by
    /// `RenderCtx::id`, so it differs between two copies of one form on a page.
    pub id: String,
    /// The field's name in the DATA: its `name` attribute, its key in the value
    /// store, and what the server matches a submitted value to. The same for
    /// every copy of the form, and never containing the form id, which the
    /// server does not have.
    pub path: String,
    pub label: Option<String>,
    pub errors: Vec<ValidationMessage>,
}

impl FieldProps {
    /// The field wrapper's classes: `fx-form-field`, plus `fx-invalid` when
    /// there are errors to show. This `div` wraps the label, the form control and
    /// its error list.
    ///
    /// **Why this picks a whole string instead of letting `rsx!` add a class.**
    /// The tempting version is two `class:` attributes on the element:
    ///
    /// ```text
    /// div { class: "fx-form-field", class: if invalid { "fx-invalid" }, … }
    /// ```
    ///
    /// `rsx!` does merge those — it joins every `class:` value with a space. But
    /// it joins them *unconditionally*: when `invalid` is false the second value
    /// is the empty string, and the result is `class="fx-form-field "`, with a
    /// trailing space, on every field that has no error. Harmless to a browser,
    /// but it is on nearly every field on every page, and it breaks any test
    /// that matches the attribute exactly. Choosing between two complete
    /// strings here never produces the stray space.
    pub fn field_class(&self) -> &'static str {
        if self.errors.is_empty() {
            "fx-form-field"
        } else {
            "fx-form-field fx-invalid"
        }
    }

    /// The field wrapper's classes plus additional, space-separated classes
    pub fn field_class_plus(&self, plus: &str) -> String {
        format!("{} {}", self.field_class(), plus)
    }
}

/// One of the options in a selectable list (e.g., [`super::Select`],
/// [`super::RadioGroup`]).
/// This is a value choice representable as a String. Contrast with
/// [`crate::members::VariantChoice`], which specifies the SHAPE of the data.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    /// The raw string written into the value map, so it has to be exactly what
    /// `parse_scalar` expects for this field's type — `"true"`, not `"True"`.
    /// That the two can differ at all is why this isn't just a `Vec<String>`.
    pub value: String,
    /// What the user reads.
    pub display: String,
}

impl Choice {
    pub fn new(value: impl Into<String>, display: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            display: display.into(),
        }
    }
}

// A choice list is usually written as a literal table, and `Choice` holds
// `String`s, so it cannot be a `const` array. These conversions are what let the
// list be a plain `&[(&str, &str)]` — which CAN be `const` — and still arrive as
// choices. The borrowed-tuple impl exists because iterating a slice yields
// references, so `STATES.iter()` would otherwise miss.
impl From<(&str, &str)> for Choice {
    fn from((value, display): (&str, &str)) -> Self {
        Self::new(value, display)
    }
}

impl From<&(&str, &str)> for Choice {
    fn from(pair: &(&str, &str)) -> Self {
        Self::from(*pair)
    }
}

impl From<(String, String)> for Choice {
    fn from((value, display): (String, String)) -> Self {
        Self::new(value, display)
    }
}

/// A choice whose display text IS its value — `"Alabama"` rather than
/// `("AL", "Alabama")`.
impl From<&str> for Choice {
    fn from(both: &str) -> Self {
        Self::new(both, both)
    }
}

impl From<&&str> for Choice {
    fn from(both: &&str) -> Self {
        Self::new(*both, *both)
    }
}

impl From<String> for Choice {
    fn from(both: String) -> Self {
        Self::new(both.clone(), both)
    }
}

/// The two choices for a `bool` field. If used in a [`super::Select`]
/// for an `Option<bool>` field, the [`super::Select`] provides the
/// "no value" choice itself.
pub(super) fn bool_choices() -> Vec<Choice> {
    vec![Choice::new("true", "True"), Choice::new("false", "False")]
}
