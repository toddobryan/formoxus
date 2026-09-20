//! App-wide defaults, and how a form overrides them.
//!
//! Three tiers, outermost first:
//!
//! ```text
//! provide_defaults(...)  ->  form! { label_case: ... }  ->  per-field spec
//! ```
//!
//! Each is consulted only when the one inside it said nothing, which is why
//! [`crate::form::FormSpec`] stores these as `Option` — `None` means "not
//! stated here", not "use the built-in".
//!
//! # Setting them
//!
//! ```ignore
//! fn App() -> Element {
//!     provide_defaults(Formoxus::new().with_label_case(LabelCase::Lower));
//!     rsx! { Router::<Route> {} }
//! }
//! ```
//!
//! Anything below that renders lowercase labels unless its own `form!` says
//! otherwise. To change one setting for a subtree, read the inherited value and
//! shadow it — a nested provider replaces the whole struct, so building from
//! [`defaults()`] is what keeps the other settings:
//!
//! ```ignore
//! provide_defaults(defaults().with_label_case(LabelCase::AllCaps));
//! ```
//!
//! # Why context rather than a global
//!
//! A `static` would be simpler to reach but could not express "different here",
//! and two tests in one binary would fight over it. Context is scoped, needs no
//! static mutable state, and costs nothing at the call sites — the alternative
//! considered was threading a parameter through every one of [`crate::Form`]'s
//! render methods. See `.claude/memory/config_cascade.md`.

use dioxus::prelude::*;

use crate::label_case::LabelCase;

/// The settings a whole app can fix once — formoxus's own configuration.
///
/// `#[non_exhaustive]`, so adding a setting later is not a breaking change.
/// Build one with [`Formoxus::new`] and the `with_*` methods rather than a
/// struct literal.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Formoxus {
    /// How a field's name becomes its label when nothing names one explicitly.
    pub label_case: LabelCase,
    /// Whether browser validation should run before Formoxus' own validation
    pub use_browser_validation: bool,
}

impl Default for Formoxus {
    /// The built-ins — what formoxus does when an app says nothing at all.
    fn default() -> Self {
        Self {
            label_case: LabelCase::Title,
            use_browser_validation: true,
        }
    }
}

impl Formoxus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_label_case(mut self, case: LabelCase) -> Self {
        self.label_case = case;
        self
    }

    pub fn use_browser_validation(mut self, value: bool) -> Self {
        self.use_browser_validation = value;
        self
    }
}

/// Make these the defaults for everything rendered below this component.
///
/// Call it in a component body, usually the app root. A nested call shadows an
/// outer one for that subtree only.
pub fn provide_defaults(config: Formoxus) {
    provide_context(config);
}

/// The defaults in effect at this point in the tree, or the built-ins if
/// nothing provided any.
///
/// **Needs a live Dioxus runtime**, so this is a render-side call. That is why
/// the cascade is resolved on [`crate::Form`] and not on
/// [`crate::FormState`]: the server rebuilds a `FormState` through
/// [`crate::Submission`] with no runtime anywhere, and would panic here.
/// It has no labels to render either way.
pub fn defaults() -> Formoxus {
    try_consume_context::<Formoxus>().unwrap_or_default()
}
