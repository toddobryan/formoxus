//! What a button IS, as `form!` spells it: its type and when its handler runs.
//!
//! Here rather than in `formoxus` so `form!` reads the same names it expands
//! to, as it does with [`crate::WidgetName`]; `formoxus` re-exports both
//! as `formoxus::buttons::{ButtonType, Invocation}`.

use crate::attrs::str_eq;

/// The five button types `form!`'s `type:` accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonType {
    Submit,
    Reset,
    Cancel,
    Destructive,
    /// Acts on the model like `Submit` — a preview, a save-draft — but renders
    /// as `type="button"` so it does not submit the form out from under its own
    /// handler. Named `Button` until 2026-09-29, which collided with the element
    /// name and made its class read `fx-button-button`.
    Action,
}

impl ButtonType {
    /// Every button type, in the order `form!`'s messages list them.
    pub const ALL: &[Self] = &[
        Self::Submit,
        Self::Reset,
        Self::Cancel,
        Self::Destructive,
        Self::Action,
    ];

    /// The keyword after `type:` in `form!`.
    ///
    /// Lowercase for the reason widget names are: two of these five are
    /// literally the HTML `type` attribute, and the author is writing HTML's
    /// word. `action` is deliberately NOT HTML's `button`: it renders as
    /// `type="button"`, but an author writing `button` and getting a class
    /// spelled `fx-button-action` would be a mapping to know rather than read.
    pub const fn form_name(&self) -> &'static str {
        match self {
            Self::Submit => "submit",
            Self::Reset => "reset",
            Self::Cancel => "cancel",
            Self::Destructive => "destructive",
            Self::Action => "action",
        }
    }

    /// The type `form!` names `name`, or `None` if it is not one. The inverse
    /// of [`Self::form_name`].
    pub const fn from_form_name(name: &str) -> Option<Self> {
        let mut i = 0;
        while i < Self::ALL.len() {
            let ty = Self::ALL[i];
            if str_eq(name, ty.form_name()) {
                return Some(ty);
            }
            i += 1;
        }
        None
    }

    /// The `type` attribute. Only `submit` and `reset` have native behaviour
    /// worth keeping; everything else is an ordinary `button` so that clicking
    /// it does not submit the form out from under its own handler.
    pub const fn html_type(&self) -> &'static str {
        match self {
            Self::Submit => "submit",
            Self::Reset => "reset",
            Self::Cancel | Self::Destructive | Self::Action => "button",
        }
    }

    /// `Submit` and `Action` act ON the model (a save, a preview), so they want
    /// it validated. `Cancel`, `Reset` and `Destructive` are escape hatches
    /// that have to work on an invalid form — validating first would trap the
    /// user in it.
    pub const fn default_invocation(&self) -> Invocation {
        match self {
            Self::Submit | Self::Action => Invocation::IfModelValidates,
            Self::Cancel | Self::Reset | Self::Destructive => Invocation::Unconditional,
        }
    }
}

/// Whether the model must pass validation before the handler is reached.
///
/// Spelled `invocation` in `form!` rather than `handler: validated |
/// unchecked`, which was the obvious alternative: `validated` names the
/// *handler*, and what is actually being chosen is whether the model has to
/// pass validation before the handler is reached at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Invocation {
    /// Validate first; the handler receives the model and never runs if it
    /// fails.
    IfModelValidates,
    /// Run regardless, with no model.
    Unconditional,
}

impl Invocation {
    /// Both, in the order `form!`'s messages list them.
    pub const ALL: &[Self] = &[Self::IfModelValidates, Self::Unconditional];

    /// The keyword after `invocation:` in `form!`.
    pub const fn form_name(&self) -> &'static str {
        match self {
            Self::IfModelValidates => "if_model_validates",
            Self::Unconditional => "unconditional",
        }
    }

    /// The invocation `form!` names `name`, or `None`.
    pub const fn from_form_name(name: &str) -> Option<Self> {
        let mut i = 0;
        while i < Self::ALL.len() {
            let invocation = Self::ALL[i];
            if str_eq(name, invocation.form_name()) {
                return Some(invocation);
            }
            i += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{ButtonType, Invocation};
    use googletest::prelude::*;

    /// No wildcard, so a new type stops this compiling until it is in `ALL`.
    /// Five entries and no duplicates (the round trip below) is then all five.
    #[gtest]
    fn all_lists_every_button_type() {
        for ty in ButtonType::ALL {
            match ty {
                ButtonType::Submit
                | ButtonType::Reset
                | ButtonType::Cancel
                | ButtonType::Destructive
                | ButtonType::Action => {}
            }
        }
        expect_that!(ButtonType::ALL.len(), eq(5));
    }

    /// Also proves the names unique: `from_form_name` returns the first match.
    #[gtest]
    fn a_button_types_form_name_round_trips() {
        for &ty in ButtonType::ALL {
            expect_that!(ButtonType::from_form_name(ty.form_name()), some(eq(ty)));
        }
        expect_that!(ButtonType::from_form_name("Submit"), none());
        expect_that!(ButtonType::from_form_name("button"), none());
    }

    #[gtest]
    fn only_submit_and_reset_keep_their_native_type() {
        let html: Vec<(&str, &str)> = ButtonType::ALL
            .iter()
            .map(|t| (t.form_name(), t.html_type()))
            .collect();
        expect_that!(
            html,
            elements_are![
                eq(&("submit", "submit")),
                eq(&("reset", "reset")),
                eq(&("cancel", "button")),
                eq(&("destructive", "button")),
                eq(&("action", "button")),
            ]
        );
    }

    #[gtest]
    fn only_submit_and_action_validate_by_default() {
        for &ty in ButtonType::ALL {
            let validates = matches!(ty, ButtonType::Submit | ButtonType::Action);
            expect_that!(
                ty.default_invocation() == Invocation::IfModelValidates,
                eq(validates),
                "{ty:?}"
            );
        }
    }

    #[gtest]
    fn an_invocations_form_name_round_trips() {
        for &invocation in Invocation::ALL {
            match invocation {
                Invocation::IfModelValidates | Invocation::Unconditional => {}
            }
            expect_that!(
                Invocation::from_form_name(invocation.form_name()),
                some(eq(invocation))
            );
        }
        expect_that!(Invocation::from_form_name("validated"), none());
    }
}
