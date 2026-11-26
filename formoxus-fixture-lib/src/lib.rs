//! A library with types and forms of its own, standing in for a crate a dev
//! depends on (`CUSTOM_FIELDS_PLAN.md` box 5.9, decision AE).
//!
//! It exists because a library's `types!` list can only be included from
//! ANOTHER crate (decision AB.4): the suite includes [`FixtureTypes`],
//! replaces one of its types, and builds the forms below to see what reaches
//! them. What each item is for:
//!
//! 1. [`Badge`] is never replaced by anything in the suite, so a test of it
//!    reads the same whether or not the suite's overrides are installed.
//! 2. [`Stamp`] is the one the suite's overrides `replace`. Every test that
//!    builds a form holding one installs the overrides first, because
//!    `install` is once per process and the tests run in any order.
//! 3. [`Seal`] lives in a second list, [`SealTypes`], which the suite does
//!    NOT include. The suite's overrides list gives `Seal` an entry of its
//!    own anyway, to show that an entry which is not a `replace` is not
//!    installed (decision AF.3).
//! 4. [`PinnedStamp`] is the field [`forms::card`] names on one path with
//!    `field:`, so an override must not reach that path (AB.7).

use facet::Facet;
use formoxus::attrs::FieldType;
use formoxus::error::ValidationMessage;
use formoxus::field::Field;
use formoxus::widgets::{AnyWidget, TextInput};

#[derive(Facet, Clone, Debug, PartialEq)]
pub struct Badge(pub String);

#[derive(Facet, Clone, Debug, PartialEq)]
pub struct Stamp(pub String);

#[derive(Facet, Clone, Debug, PartialEq)]
pub struct Seal(pub String);

#[derive(Facet, Clone, Debug, PartialEq)]
pub struct Card {
    pub title: String,
    pub badge: Badge,
    pub stamp: Stamp,
    /// Named with `field: PinnedStamp` in [`forms::card`].
    pub pinned: Stamp,
}

#[derive(Facet, Clone, Debug, PartialEq)]
pub struct SealedCard {
    pub seal: Seal,
}

formoxus::types! {
    pub struct FixtureTypes;
    Badge => newtype String,
    Stamp => newtype String,
}

formoxus::types! {
    pub struct SealTypes;
    Seal => newtype String,
}

/// The field [`forms::card`] pins on `pinned`. It writes `pinned:` ahead of
/// the text, so a test can tell it from both the list's field and an
/// override.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PinnedStamp;

impl Field for PinnedStamp {
    type Value = Stamp;
    type Context = ();
    const KIND: FieldType = FieldType::Text;

    fn format(&self, value: &Stamp) -> String {
        format!("pinned:{}", value.0)
    }

    fn parse(&self, raw: &str) -> Result<Stamp, ValidationMessage> {
        Ok(Stamp(
            raw.strip_prefix("pinned:").unwrap_or(raw).to_string(),
        ))
    }

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

/// The library's forms. Each module points `form!` at a list, as a library
/// would: a `use` from anywhere in the crate works, because the list's replay
/// macro has a name of its own (decision AI).
pub mod forms {
    use super::{Card, PinnedStamp};
    use crate::FixtureTypes as FormoxusTypes;
    use formoxus::form::FormSpec;
    use formoxus::prelude::*;

    /// `stamp` is left to the types; `pinned` names its own field.
    pub fn card() -> FormSpec<Card> {
        form! { Card { pinned => { field: PinnedStamp } } }
    }

    /// A form over a type the suite's list never includes.
    pub mod sealed {
        use crate::SealTypes as FormoxusTypes;
        use crate::SealedCard;
        use formoxus::form::FormSpec;
        use formoxus::prelude::*;

        pub fn card() -> FormSpec<SealedCard> {
            form! { SealedCard {} }
        }
    }
}
