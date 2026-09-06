//! The `FormMember` trait every part of a form implements, and the container
//! kinds that make a form a tree rather than a flat list.

use dioxus::prelude::*;
use facet::{Partial, ReflectError};
use std::{collections::HashMap, fmt::Debug};

mod field_set;
mod list_set;
mod option_member;
mod variant_set;

pub use field_set::FieldSet;
pub use list_set::ListSet;
pub use option_member::OptionMember;
pub use variant_set::{VariantChoice, VariantSet};

use crate::error::FormAccessError;
use dioxus::stores::Store;

pub trait FormMember: Debug {
    fn name(&self) -> String;
    fn label(&self) -> Option<String>;
    fn render(&self, ctx: &RenderCtx) -> Element;
    /// This member's current value as the string an `<input>` would show.
    /// Containers have no scalar value of their own and return `""` — the
    /// widget layer only ever asks leaves for this.
    fn raw_value(&self) -> String;
    /// Flatten this member's leaves into `(qualified_path, raw_value)` pairs,
    /// e.g. `("location.street", "123 Main St")`. Paths are qualified because
    /// two field sets in one form can each have a `street`.
    fn collect_leaves(&self, prefix: &str, out: &mut Vec<(String, String)>);
    /// The reverse of [`collect_leaves`](Self::collect_leaves): each leaf looks
    /// up its own qualified path in `values` and takes the raw string back in.
    /// This is the "shuffle back" from widget state into plain form data.
    fn apply_leaves(&mut self, prefix: &str, values: &HashMap<String, String>);
    fn validate(&mut self);
    fn has_errors(&self) -> bool;
    fn clone_box(&self) -> Box<dyn FormMember>;
    fn write_value_into<'p>(&self, partial: Partial<'p>) -> Result<Partial<'p>, ReflectError>;
    
    fn write_into<'p>(&self, partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        let mut partial = partial.begin_field(&self.name())?;
        partial = self.write_value_into(partial)?;
        partial.end()
    }

    fn is_present(&self) -> bool;
    fn clear_errors(&mut self);

    fn choose_variant(&mut self, prefix: &str, path: &str, variant: &str) -> Result<(), FormAccessError>;
}

impl Clone for Box<dyn FormMember> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Shown for an `Option<Enum>` the user chose to leave empty — in the disabled
/// input below, and (later) as the "none" entry in a variant picker. Display
/// only: a disabled input isn't submitted, so this never comes back through
/// `FormData::values()` and can't be mistaken for a value. That's what keeps it
/// from reintroducing the sentinel problem `VariantChoice` exists to avoid.
pub(crate) const ABSENT_DISPLAY: &str = "--none--";

/// Is `path` exactly `nested`, or somewhere inside its subtree?
///
/// The guard every container's `choose_variant` uses before recursing. Testing
/// containment up front means at most ONE child is ever asked, so a child's real
/// failure ("no such variant") propagates verbatim instead of being swallowed as
/// "no such path" by a parent that was still shopping around.
pub(crate) fn owns(nested: &str, path: &str) -> bool {
    path == nested || path.strip_prefix(nested).is_some_and(|rest| rest.starts_with('.'))
}

/// `("", "title") -> "title"`, `("location", "street") -> "location.street"`.
pub(crate) fn qualify(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}.{name}")
    }
}

/// The live raw values, keyed by qualified path — what `leaves()` produces and
/// what `apply()` consumes, held in a store so a write touches one input.
pub type ValuesByPath = Store<HashMap<String, String>>;

/// What a member needs in order to render: where it sits in the path tree,
/// where the live values are, and whether the browser should treat its leaves
/// as required.
///
/// A struct rather than three parameters because this only grows — the variant
/// `<select>` and the add/remove-row buttons will each need a way to signal a
/// structural edit back up.
///
/// **`required` is a presentation hint, not the authority.** HTML5 `required`
/// is per-input, so it cannot express the all-or-nothing rule an optional
/// struct follows (absent means EVERY leaf empty; a partly filled one is an
/// error). Marking those leaves required would block a deliberately blank
/// address; leaving them unmarked lets the browser accept a half-filled one.
/// The second is the lesser evil, and `validate()` remains where the real rule
/// lives — see the optional-container tests.
#[derive(Clone, Debug)]
pub struct RenderCtx {
    pub prefix: String,
    pub values: ValuesByPath,
    pub required: bool,
}

impl RenderCtx {
    /// The context a whole form starts from: at the root, and required until
    /// some `OptionMember` says otherwise.
    pub fn root(values: ValuesByPath) -> Self {
        Self { prefix: String::new(), values, required: true }
    }

    /// Descend into a named child — the `qualify` every container already does.
    pub fn nested(&self, name: &str) -> Self {
        Self {
            prefix: qualify(&self.prefix, name),
            values: self.values,
            required: self.required,
        }
    }

    /// Everything from here down is optional. `OptionMember` is the only member
    /// that *changes* the context rather than passing it along unaltered.
    pub fn optional(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            values: self.values,
            required: false,
        }
    }

    /// This member's own qualified path, for a leaf's `name` attribute.
    pub fn path(&self, name: &str) -> String {
        qualify(&self.prefix, name)
    }
}
