//! What a control IS, as opposed to what it renders: the control
//! vocabulary `form!` speaks, and the two prop bundles every input takes.

use dioxus::prelude::*;

use crate::ValuesByPath;
use crate::error::FieldError;

#[derive(Clone)]
pub enum ControlType {
    Input(InputType),
    Textarea,
    Select,
    SelectMultiple,
    Checkbox,
    CheckboxMultiple,
    RadioGroup,
    File,
    /// A control the author supplied, via `form!`'s `custom(MyWidget)`.
    ///
    /// The escape hatch for a value kind the built-in controls cannot serve —
    /// `Markdown` needs a live preview, a `Ref<Source>` needs an async-fed
    /// combobox. Neither is expressible as an `<input type=…>`, and neither
    /// belongs in `ValueKind`: they are presentation, not value family.
    ///
    /// **`fn` pointer, NOT `Box<dyn Fn>` — the derives force it.** `ControlType`
    /// is `Clone + Debug + PartialEq`, and a boxed closure supplies none of the
    /// three. A non-capturing closure coerces to a plain `fn`, which is `Copy`,
    /// clones trivially, and compares by address. That is what lets this variant
    /// exist without disturbing anything that already holds a `ControlType`.
    ///
    /// The cost is `Debug`: a fn pointer prints as an address. Hence `name`,
    /// which `form!` fills in from the control's own path so panics and test
    /// assertions read `custom(MarkdownWidget)` rather than `0x7f…`.
    Custom {
        name: &'static str,
        render: fn(ControlProps) -> Element,
    },
}

// `Debug` and `PartialEq` are hand-written rather than derived, and only because
// of `Custom`'s `fn` pointer. Both impls reproduce the derive exactly for every
// other variant.
impl std::fmt::Debug for ControlType {
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
            // `form!`'s own spelling, so the panic in `ScalarInput` quotes back
            // what the author wrote. Deriving this would print the fn pointer's
            // address beside the name, which is noise in every message it
            // appears in.
            Self::Custom { name, .. } => write!(f, "custom({name})"),
        }
    }
}

impl PartialEq for ControlType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Input(a), Self::Input(b)) => a == b,
            // Two custom controls are the same control when they name the same
            // input. Comparing the `fn` pointers is what rustc warns about and
            // is genuinely meaningless here: identical functions may be merged
            // to one address, and one function may be duplicated across codegen
            // units, so neither equality nor inequality tells you anything about
            // which control you have.
            (Self::Custom { name: a, .. }, Self::Custom { name: b, .. }) => a == b,
            _ => std::mem::discriminant(self) == std::mem::discriminant(other),
        }
    }
}

/// What a custom control is handed: the same pair every built-in control gets.
///
/// One struct rather than two parameters because `render` is a `fn` pointer and
/// a single argument keeps that signature stable as the boundary grows.
///
/// Note this is the control boundary the design notes fix — `(path, label,
/// required, errors)` in `props`, plus the value store — and NOT a
/// `FormField<T>`. A control never sees the typed field: it reads and writes raw
/// strings through `values`, exactly as `HtmlInput` does.
#[derive(Clone, PartialEq)]
pub struct ControlProps {
    pub values: ValuesByPath,
    pub props: FieldProps,
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
            // `FormField::default_control`. `type="number"` hands back `""` for
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
}
