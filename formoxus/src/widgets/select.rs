//! `<select>` over a fixed set of choices.

use dioxus::prelude::*;

use super::errors::FieldErrors;
use super::types::FieldProps;
use super::values::{get_current, write_value};
use crate::ValuesByPath;

/// One option in a [`Select`].
#[derive(Clone, Debug, PartialEq)]
pub struct SelectChoice {
    /// The raw string written into the value map, so it has to be exactly what
    /// `parse_scalar` expects for this field's type — `"true"`, not `"True"`.
    /// That the two can differ at all is why this isn't just a `Vec<String>`.
    pub value: String,
    /// What the user reads.
    pub display: String,
}

impl SelectChoice {
    pub fn new(value: impl Into<String>, display: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            display: display.into(),
        }
    }
}

// A choice list is usually written as a literal table, and `SelectChoice` holds
// `String`s, so it cannot be a `const` array. These conversions are what let the
// list be a plain `&[(&str, &str)]` — which CAN be `const` — and still arrive as
// choices. The borrowed-tuple impl exists because iterating a slice yields
// references, so `STATES.iter()` would otherwise miss.
impl From<(&str, &str)> for SelectChoice {
    fn from((value, display): (&str, &str)) -> Self {
        Self::new(value, display)
    }
}

impl From<&(&str, &str)> for SelectChoice {
    fn from(pair: &(&str, &str)) -> Self {
        Self::from(*pair)
    }
}

impl From<(String, String)> for SelectChoice {
    fn from((value, display): (String, String)) -> Self {
        Self::new(value, display)
    }
}

/// A choice whose display text IS its value — `"Alabama"` rather than
/// `("AL", "Alabama")`.
impl From<&str> for SelectChoice {
    fn from(both: &str) -> Self {
        Self::new(both, both)
    }
}

impl From<&&str> for SelectChoice {
    fn from(both: &&str) -> Self {
        Self::new(*both, *both)
    }
}

impl From<String> for SelectChoice {
    fn from(both: String) -> Self {
        Self::new(both.clone(), both)
    }
}

/// The three states of an `Option<bool>`, minus the absent one — that comes
/// from `Select`'s own "no value" option, so it is spelled in exactly one
/// place rather than once per caller.
pub(super) fn bool_choices() -> Vec<SelectChoice> {
    vec![
        SelectChoice::new("true", "True"),
        SelectChoice::new("false", "False"),
    ]
}

/// A `<select>` over a fixed set of choices, bound to one path in the value map.
///
/// Reads its current value from `values` like every other leaf input rather
/// than taking it as a prop. That is not just consistency: computing `selected`
/// for a prop would mean reading the store in `FormField::render`, a plain
/// function with no scope of its own, which subscribes *the caller* — so one
/// change here would re-render the whole form. [`VariantSelect`] takes its
/// selection as a prop precisely because a variant choice is NOT a leaf and has
/// no path to read.
///
/// An option's value is the raw string itself, NOT an index into `choices`.
/// Indexing is the usual dodge for a `T` that might not survive a round trip
/// through a string, but here `T -> String -> T` is a guaranteed identity for
/// every builtin scalar (see the `roundtrip` tests), so the indirection — and
/// the silent `unwrap_or_default()` it needs when an index doesn't match — buys
/// nothing and loses the value.
#[component]
pub fn Select(values: ValuesByPath, choices: Vec<SelectChoice>, props: FieldProps) -> Element {
    let FieldProps {
        path,
        label: label_text,
        required,
        errors,
    } = props;

    let current = get_current(&path, values);

    // See `Input` for why this is `Option` rather than a plain bool.
    let invalid = (!errors.is_empty()).then_some("true");

    rsx! {
        label { class: "form-field",
            if let Some(text) = label_text {
                span { class: "field-label", "{text}" }
            }
            if required {
                span { class: "required", " *" }
            }
            select {
                aria_invalid: invalid,
                // Unlike `VariantSelect`, this one IS a leaf, so it must carry a
                // `name` or `apply_form_values` would never see it.
                name: "{path}",
                required,
                // No branch on emptiness: the "no value" option's value is `""`,
                // and `""` IS absence at both boundaries, so the same write does
                // for every option.
                onchange: move |e: FormEvent| write_value(&path, values, e.value()),
                // Required and unanswered: an unselectable placeholder, so the
                // browser's own validation blocks submit and the user can't
                // choose their way back to "unanswered". Optional: a real
                // selectable entry, because absent is a legitimate answer.
                if required && current.is_empty() {
                    option { value: "", selected: true, disabled: true, hidden: true, "Choose..." }
                } else if !required {
                    option { value: "", selected: current.is_empty(), "{ABSENT_DISPLAY}" }
                }
                for choice in choices {
                    option {
                        value: "{choice.value}",
                        selected: choice.value == current,
                        "{choice.display}"
                    }
                }
            }
            FieldErrors { errors }
        }
    }
}

/// The "leave this out" entry in an optional enum's picker.
///
/// Display only, and it stays that way for a structural reason rather than a
/// cosmetic one: the `<select>` carries no `name`, so nothing it holds is ever
/// collected by `FormData::values()` and this text cannot come back as a value.
/// That is what keeps it from reintroducing the sentinel problem
/// [`VariantChoice`](crate::VariantChoice) exists to avoid — a model
/// with a genuine `None` variant would otherwise be indistinguishable from an
/// unanswered optional field. What the select actually emits is `""`, which
/// `VariantSelect` turns into `ChooseVariant { variant: None }`.
///
/// [`Select`] shows the same text, and it *does* carry a `name` — but it is
/// safe there for the same reason by a different route: the option's value is
/// `""`, never this text, and `""` is absence at both boundaries.
/// **Not localizable yet, and it should be.** This is English punctuation baked
/// into a library: a form rendered in French or Japanese still reads
/// `--none--`. The eventual shape is configuration — most likely alongside
/// whatever mechanism makes error rendering overridable, since both are "text
/// formoxus emits on the consumer's behalf" — rather than a second const.
/// Deliberately deferred; nothing depends on it being fixed soon.
pub const ABSENT_DISPLAY: &str = "--none--";
