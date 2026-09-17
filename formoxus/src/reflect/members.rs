//! The `FormMember` trait every part of a form implements, and the container
//! kinds that make a form a tree rather than a flat list.

use dioxus::prelude::*;
use facet::{Partial, ReflectError};
use indexmap::IndexMap;
use std::{collections::HashMap, fmt::Debug};

mod field_set;
mod list_set;
mod option_member;
mod variant_set;

pub use field_set::FieldSet;
pub use list_set::ListSet;
pub use option_member::OptionMember;
pub use variant_set::{VariantChoice, VariantSet};

use crate::error::{FieldError, FormAccessError};
use crate::label_case::{LabelCase, ToCase};
use crate::reflect::FieldSpec;
use crate::reflect::form::FieldErrors;
use dioxus::stores::Store;

pub type FieldSpecs = IndexMap<String, FieldSpec>;

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

    fn collect_errors(&self, prefix: &str, out: &mut FieldErrors);

    /// Apply a structural edit — choose a variant, add or remove a row.
    ///
    /// One method rather than one per edit kind because the containment walk
    /// each container performs is identical regardless of the edit; only the
    /// member that owns the path cares what kind it is. `Edit::path()` is the
    /// only thing a container reads, so containers stay kind-agnostic.
    fn edit(&mut self, prefix: &str, edit: &Edit) -> Result<(), FormAccessError>;

    fn push_field_error(&mut self, prefix: &str, path: &str, error: FieldError) -> Result<(), FormAccessError>;

    fn apply_specs(&mut self, prefix: &str, fields: &FieldSpecs);
}

impl Clone for Box<dyn FormMember> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Is `path` exactly `nested`, or somewhere inside its subtree?
///
/// The guard every container's `choose_variant` uses before recursing. Testing
/// containment up front means at most ONE child is ever asked, so a child's real
/// failure ("no such variant") propagates verbatim instead of being swallowed as
/// "no such path" by a parent that was still shopping around.
pub(crate) fn owns(nested: &str, path: &str) -> bool {
    path == nested || path.strip_prefix(nested).is_some_and(|rest| rest.starts_with('.'))
}

pub(crate) fn no_such_path(path: &str) -> FormAccessError {
    FormAccessError(format!("no such path: {path}"))
}

/// Guard that `path` is this member's own path or somewhere inside it.
pub(crate) fn ensure_owned(container: &str, path: &str) -> Result<(), FormAccessError> {
    if owns(container, path) {
        Ok(())
    } else {
        Err(no_such_path(path))
    }
}

/// A member's name humanized for display: `can_shuffle` -> "Can Shuffle".
///
/// The fallback when nothing set a label explicitly, which is every member today
/// — the shape carries a field's name but no prose for it. Reuses the derive
/// path's [`crate::label_case`], so both paths title-case identically.
///
/// `None` for a `ListSet` row key (`#3`) and for an all-digit name. Neither is
/// prose: a row key is an identity, and a tuple-struct field's "0" is a
/// position. The list itself is what carries the label.
pub(crate) fn default_label(name: &str) -> Option<String> {
    if name.starts_with(ROW_SIGIL) || name.chars().all(|c| c.is_ascii_digit()) {
        None
    } else {
        Some(name.to_case(LabelCase::Title))
    }
}

/// A variant descriptor segment: `Circle` -> `$Circle`.
///
/// `$` cannot begin a Rust identifier, so a descriptor can never collide with a
/// field name. That's what lets two variants of the same enum each have a
/// `size` without both claiming `footprint.size` — see [`model_path`].
pub(crate) fn variant_segment(variant: &str) -> String {
    format!("${variant}")
}

/// Row keys are `#`-prefixed for the same reason variants are `$`-prefixed:
/// neither character can begin a Rust identifier, so a key can never be mistaken
/// for a field name.
pub(crate) const ROW_SIGIL: char = '#';

/// A list-row key segment: `3` -> `#3`.
///
/// A row's name is its *identity*, not its position — which is what lets a row
/// be inserted in the middle, removed, or reordered without renaming its
/// neighbours. Renaming them would move every leaf beneath them to a different
/// key in the value store, and the values would have to be shuffled to match:
/// the same silent-corruption hazard [`variant_segment`] exists to prevent,
/// except recurring on every insert and remove rather than only on a variant
/// switch.
///
/// The cost is that a row's *position* is no longer recoverable from its path.
/// Order lives in `ListSet::rows` and nowhere else — see [`model_path`].
pub(crate) fn row_segment(key: usize) -> String {
    format!("{ROW_SIGIL}{key}")
}

/// Strip variant descriptors from a leaf path, giving the path through the
/// *model*: `footprint.$Circle.size` -> `footprint.size`.
///
/// Leaf paths mirror the model exactly apart from these segments, which exist
/// only to keep same-named fields in different variants apart. A path segment
/// beginning with `$` is never a field, and a descriptor never appears as the
/// final segment of a leaf path — it is only ever a prefix.
///
/// A list-row key (`answers.#3.text`) is left alone: it *is* a real element of
/// the model, just identified by [identity rather than
/// position](row_segment). Which element it is cannot be read off the string —
/// only `ListSet::rows` knows that — so the result names the row without
/// placing it.
pub fn model_path(path: &str) -> String {
    path.split('.')
        .filter(|segment| !segment.starts_with('$'))
        .collect::<Vec<_>>()
        .join(".")
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
    pub on_edit: Callback<Edit>,
}

impl RenderCtx {
    /// The context a whole form starts from: at the root, and required until
    /// some `OptionMember` says otherwise.
    pub fn root(values: ValuesByPath, on_edit: Callback<Edit>) -> Self {
        Self { prefix: String::new(), values, required: true, on_edit }
    }

    /// Descend into a named child — the `qualify` every container already does.
    pub fn nested(&self, name: &str) -> Self {
        Self {
            prefix: qualify(&self.prefix, name),
            values: self.values,
            required: self.required,
            on_edit: self.on_edit,
        }
    }

    /// Everything from here down is optional. `OptionMember` is the only member
    /// that *changes* the context rather than passing it along unaltered.
    pub fn optional(&self) -> Self {
        Self {
            prefix: self.prefix.clone(),
            values: self.values,
            required: false,
            on_edit: self.on_edit,
        }
    }

    /// This member's own qualified path, for a leaf's `name` attribute.
    pub fn path(&self, name: &str) -> String {
        qualify(&self.prefix, name)
    }
}

/// A change to the form's *shape*, as opposed to its values — the counterpart
/// to `apply`. Crosses the widget boundary in a `Callback<Edit>`, which is why
/// it owns its strings rather than borrowing.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    ChooseVariant {
        path: String,
        variant: Option<String>,
    },
    AddRow {
        path: String,
        /// Insert the new row *before* the row currently at this position, or
        /// append when `None`. A position rather than a key because that is what
        /// the control knows: an "insert here" button sits between two rendered
        /// rows and knows only where it is. Keys identify a row across edits;
        /// positions locate a gap at one instant, which is all an insert needs.
        before: Option<usize>,
    },
    RemoveRow {
        path: String,
        index: usize,
    }
}

impl Edit {
    /// Every edit names a target, and the containment walk reads only this —
    /// never the kind.
    pub fn path(&self) -> &str {
        match self {
            Edit::ChooseVariant { path, .. } => path,
            Edit::AddRow { path, .. } => path,
            Edit::RemoveRow { path, .. } => path,
        }
    }

    pub fn new_choose_variant(path: &str, variant: Option<&str>) -> Edit {
        Edit::ChooseVariant {
            path: path.to_string(),
            variant: variant.map(str::to_string),
        }
    }
}
