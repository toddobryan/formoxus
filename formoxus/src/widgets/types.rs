//! What a widget IS, as opposed to what it renders: the widget
//! vocabulary `form!` speaks, and the two prop bundles every input takes.
//! This includes [`Choice`], which represents one of the legal values
//! a widget can hold.

use dioxus::prelude::*;

use crate::ValuesStore;
use crate::error::FieldError;

#[derive(Clone)]
pub enum WidgetType {
    Input(InputType),
    Textarea,
    Select,
    SelectMultiple,
    Checkbox,
    CheckboxMultiple,
    RadioGroup,
    File,
    /// A widget the author supplied, via `form!`'s `custom(MyWidget)`.
    ///
    /// The escape hatch for a value kind the built-in widgets cannot serve —
    /// `Markdown` needs a live preview, a `Ref<Source>` needs an async-fed
    /// combobox. Neither is expressible as an `<input type=…>`, and neither
    /// belongs in `ValueKind`: they are presentation, not value family.
    ///
    /// **`fn` pointer, NOT `Box<dyn Fn>` — the derives force it.** `WidgetType`
    /// is `Clone + Debug + PartialEq`, and a boxed closure supplies none of the
    /// three. A non-capturing closure coerces to a plain `fn`, which is `Copy`,
    /// clones trivially, and compares by address. That is what lets this variant
    /// exist without disturbing anything that already holds a `WidgetType`.
    ///
    /// The cost is `Debug`: a fn pointer prints as an address. Hence `name`,
    /// which `form!` fills in from the widget's own path so panics and test
    /// assertions read `custom(MarkdownWidget)` rather than `0x7f…`.
    Custom {
        name: &'static str,
        render: fn(WidgetProps) -> Element,
    },
}

// `Debug` and `PartialEq` are hand-written rather than derived, and only because
// of `Custom`'s `fn` pointer. Both impls reproduce the derive exactly for every
// other variant.
impl std::fmt::Debug for WidgetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input(t) => write!(f, "Input({t:?})"),
            Self::Textarea => f.write_str("Textarea"),
            Self::Select => f.write_str("Select"),
            Self::SelectMultiple => f.write_str("SelectMultiple"),
            Self::Checkbox => f.write_str("Checkbox"),
            Self::CheckboxMultiple => f.write_str("CheckboxMultiple"),
            Self::RadioGroup => f.write_str("RadioGroup"),
            Self::File => f.write_str("File"),
            // `form!`'s own spelling, so the panic in `ScalarWidget` quotes back
            // what the author wrote. Deriving this would print the fn pointer's
            // address beside the name, which is noise in every message it
            // appears in.
            Self::Custom { name, .. } => write!(f, "custom({name})"),
        }
    }
}

impl PartialEq for WidgetType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Input(a), Self::Input(b)) => a == b,
            // Two custom widgets are the same widget when they name the same
            // input. Comparing the `fn` pointers is what rustc warns about and
            // is genuinely meaningless here: identical functions may be merged
            // to one address, and one function may be duplicated across codegen
            // units, so neither equality nor inequality tells you anything about
            // which widget you have.
            (Self::Custom { name: a, .. }, Self::Custom { name: b, .. }) => a == b,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
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

#[derive(Clone, Debug, PartialEq)]
pub enum InputType {
    Text,
    Password,
    Hidden,
    Number,
    Email,
    Telephone,
    Url,
    Search,
    Color,
    Date,
    Time,
    DatetimeLocal,
    Month,
    Week,
}

impl InputType {
    /// The `type=` attribute this renders as.
    ///
    /// HTML's spelling, which is not always the variant's: `tel`, and
    /// `datetime-local` with the hyphen an ident could not carry. `form!`'s
    /// vocabulary spells these `tel` and `datetime_local`, so `InputType` is the
    /// pivot with HTML's names on both sides of it.
    pub fn html_type(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Password => "password",
            Self::Hidden => "hidden",
            // Selectable, but NOT the default for a numeric field — see
            // `FormField::default_widget`. `type="number"` hands back `""` for
            // anything the browser dislikes, so a half-typed value vanishes
            // mid-keystroke. A numeric renders as text and `ValueKind` parses it.
            // Anyone who wants the spinner and the mobile keypad can ask.
            Self::Number => "number",
            Self::Email => "email",
            Self::Telephone => "tel",
            Self::Url => "url",
            Self::Search => "search",
            Self::Color => "color",
            Self::Date => "date",
            Self::Time => "time",
            Self::DatetimeLocal => "datetime-local",
            Self::Month => "month",
            Self::Week => "week",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldProps {
    pub path: String,
    pub label: Option<String>,
    // whether the field(s) below are considered required in the form,
    // modulo weird things like not being able to mark checkboxes required
    pub required: bool,
    pub errors: Vec<FieldError>,
    /// Present and equal to "true" only when there is an error.
    /// `aria-invalid="false"` is NOT the neutral value — per ARIA it
    /// asserts "checked, and passed", so an untouched form would claim to
    /// have validated every field, and a screen reader would say so.
    /// Stylesheets that paint a validated-and-clean state key off it too.
    /// Absent is the only neutral state. Dioxus omits an attribute whose
    /// value is `None`, which is what makes absence expressible at all.
    pub aria_invalid: Option<&'static str>,
}

/// One of the options in a selectable list (e.g., [`super::Select`],
/// [`super::RadioGroup`]).
/// This is a value choice representable as a String. Contrast with
/// [`crate::VariantChoice`], which specifies the SHAPE of the data.
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
