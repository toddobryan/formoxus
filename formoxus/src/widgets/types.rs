//! [`Choice`], one of the legal values a chooser can hold, and the
//! `InputType` re-export.

/// Defined in `formoxus-spec`, because the attribute table and `form!` both
/// key on it and `formoxus-macros` cannot reach this crate. Re-exported here so
/// that `formoxus::widgets::InputType` is still its address.
pub use formoxus_spec::InputType;

/// One of the options in a selectable list (e.g., [`super::Select`],
/// [`super::RadioGroup`]).
/// This is a value choice representable as a String. Contrast with
/// [`crate::nodes::VariantChoice`], which specifies the SHAPE of the data.
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
