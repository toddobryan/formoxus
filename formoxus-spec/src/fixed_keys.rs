//! The elements no widget renders, as keys for the classes formoxus gives
//! them. Here rather than in `formoxus` so `form!` can check a
//! `classes_plus:` name at compile time.

use crate::attrs::str_eq;
use crate::buttons::ButtonType;

/// The elements no widget renders, each keying a class slot of its own in
/// `formoxus::classes::Classes`. The value in each comment is what
/// `Classes::fx()` gives it.
///
/// Keyed by the node or role that renders the element, not by the element:
/// three keys render a `<fieldset>`, and the key says which. So `FieldSet` is
/// `formoxus::nodes::FieldSet`'s `<fieldset>`, capitalized like that type,
/// and not [`HtmlElement::Fieldset`](crate::HtmlElement::Fieldset), which is spelled
/// as HTML spells the element.
///
/// `#[non_exhaustive]` so a slot can be added without breaking a `match`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FixedKey {
    /// The `<form>`: `fx-form`.
    Form,
    /// The form's title, a `<div>` its `<form>` is labelled by: `fx-form-title`.
    FormTitle,
    /// The `<ul>` of form-level errors, from the spec's validator:
    /// `fx-form-errors`.
    FormErrors,
    /// One `<li>` in it: `fx-form-error`.
    FormError,
    /// The `<ul>` of button problems: a button whose `fn` does not match how
    /// it was declared. A programmer's mistake, not the user's, so it is
    /// marked apart: `fx-form-errors fx-problems`.
    ButtonProblems,
    /// One `<li>` in it: `fx-form-error fx-problem`, so it styles like a
    /// form error by default and `.fx-problem` reaches it alone.
    ButtonProblem,
    /// The `<div>` around the buttons: `fx-buttons`.
    ButtonRow,
    /// One button, by its type: `fx-button fx-button-{submit, reset,
    /// cancel, destructive, action}`.
    Button(ButtonType),
    /// A list's add-row button: `fx-button fx-button-add-row`.
    AddRowButton,
    /// A row's remove button: `fx-button fx-button-remove-row`.
    RemoveRowButton,
    /// The `<fieldset>` around a nested struct: `fx-group fx-field-set`.
    FieldSet,
    /// Its `<legend>`: `fx-legend fx-field-label`.
    FieldSetLegend,
    /// The `<fieldset>` around a list: `fx-group fx-list-set`.
    ListSet,
    /// Its `<legend>`: `fx-legend fx-field-label`.
    ListSetLegend,
    /// The `<div>` around one row of a list: `fx-form-row`.
    ListRow,
    /// The `<fieldset>` around an enum field: `fx-group fx-variant-set`.
    VariantSet,
    /// Its `<legend>`: `fx-legend fx-field-label`.
    VariantSetLegend,
    /// The ` *` after a required field's caption: `fx-required`.
    RequiredMarker,
    /// The `<ul>` of one field's errors: `fx-field-errors`.
    FieldErrors,
    /// One `<li>` in it: `fx-field-error`.
    FieldError,
}

impl FixedKey {
    /// Every key, one `Button` per [`ButtonType`].
    pub const ALL: &[Self] = &[
        Self::Form,
        Self::FormTitle,
        Self::FormErrors,
        Self::FormError,
        Self::ButtonProblems,
        Self::ButtonProblem,
        Self::ButtonRow,
        Self::Button(ButtonType::Submit),
        Self::Button(ButtonType::Reset),
        Self::Button(ButtonType::Cancel),
        Self::Button(ButtonType::Destructive),
        Self::Button(ButtonType::Action),
        Self::AddRowButton,
        Self::RemoveRowButton,
        Self::FieldSet,
        Self::FieldSetLegend,
        Self::ListSet,
        Self::ListSetLegend,
        Self::ListRow,
        Self::VariantSet,
        Self::VariantSetLegend,
        Self::RequiredMarker,
        Self::FieldErrors,
        Self::FieldError,
    ];

    /// How `form!`'s `classes_plus:` names this key: the variant in
    /// `snake_case`, and a button as `button.` plus its `form!` type keyword.
    pub const fn form_name(&self) -> &'static str {
        match self {
            Self::Form => "form",
            Self::FormTitle => "form_title",
            Self::FormErrors => "form_errors",
            Self::FormError => "form_error",
            Self::ButtonProblems => "button_problems",
            Self::ButtonProblem => "button_problem",
            Self::ButtonRow => "button_row",
            Self::AddRowButton => "add_row_button",
            Self::RemoveRowButton => "remove_row_button",
            Self::FieldSet => "field_set",
            Self::FieldSetLegend => "field_set_legend",
            Self::ListSet => "list_set",
            Self::ListSetLegend => "list_set_legend",
            Self::ListRow => "list_row",
            Self::VariantSet => "variant_set",
            Self::VariantSetLegend => "variant_set_legend",
            Self::RequiredMarker => "required_marker",
            Self::FieldErrors => "field_errors",
            Self::FieldError => "field_error",
            Self::Button(ButtonType::Submit) => "button.submit",
            Self::Button(ButtonType::Reset) => "button.reset",
            Self::Button(ButtonType::Cancel) => "button.cancel",
            Self::Button(ButtonType::Destructive) => "button.destructive",
            Self::Button(ButtonType::Action) => "button.action",
        }
    }

    /// The key `form!` names `name`, or `None`. The inverse of
    /// [`Self::form_name`].
    pub const fn from_form_name(name: &str) -> Option<Self> {
        let mut i = 0;
        while i < Self::ALL.len() {
            let key = Self::ALL[i];
            if str_eq(name, key.form_name()) {
                return Some(key);
            }
            i += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::FixedKey;
    use crate::buttons::ButtonType;
    use googletest::prelude::*;

    /// No wildcard, so a new key stops this compiling until it is in `ALL`;
    /// with the count and the unique names below, `ALL` holds each once.
    #[gtest]
    fn all_lists_every_key_and_every_button() {
        for key in FixedKey::ALL {
            match key {
                FixedKey::Form
                | FixedKey::FormTitle
                | FixedKey::FormErrors
                | FixedKey::FormError
                | FixedKey::ButtonProblems
                | FixedKey::ButtonProblem
                | FixedKey::ButtonRow
                | FixedKey::AddRowButton
                | FixedKey::RemoveRowButton
                | FixedKey::FieldSet
                | FixedKey::FieldSetLegend
                | FixedKey::ListSet
                | FixedKey::ListSetLegend
                | FixedKey::ListRow
                | FixedKey::VariantSet
                | FixedKey::VariantSetLegend
                | FixedKey::RequiredMarker
                | FixedKey::FieldErrors
                | FixedKey::FieldError
                | FixedKey::Button(
                    ButtonType::Submit
                    | ButtonType::Reset
                    | ButtonType::Cancel
                    | ButtonType::Destructive
                    | ButtonType::Action,
                ) => {}
            }
        }
        expect_that!(FixedKey::ALL.len(), eq(19 + ButtonType::ALL.len()));
    }

    /// Also proves the names unique: `from_form_name` returns the first match.
    #[gtest]
    fn a_keys_form_name_round_trips() {
        for &key in FixedKey::ALL {
            expect_that!(FixedKey::from_form_name(key.form_name()), some(eq(key)));
        }
        expect_that!(FixedKey::from_form_name("ButtonRow"), none());
        expect_that!(FixedKey::from_form_name("button"), none());
    }

    /// A button's name is its `form!` type keyword, so the two cannot drift.
    #[gtest]
    fn a_buttons_key_is_named_by_its_type_keyword() {
        for &ty in ButtonType::ALL {
            expect_that!(
                FixedKey::Button(ty).form_name(),
                eq(&format!("button.{}", ty.form_name()))
            );
        }
    }
}
