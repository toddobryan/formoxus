//! Proc-macro crate for **formoxus**. Consumers depend on the `formoxus` facade
//! crate, which re-exports both macros from here — they don't depend on this
//! crate directly.
//!
//! Two function-like macros, no derives. That's the whole design: a derive runs
//! on a type *definition*, so it can only ever expand in the crate that owns the
//! type. These expand at the **call site**, in the consuming crate, which is what
//! lets a form name a model from one crate and a widget from another without
//! tripping the orphan rule — see `.claude/memory/facet_form_design_decisions.md`.
//!
//! - [`macro@form`] builds a `FormSpec` for a model from a declarative block:
//!   title, per-field labels and widgets, `[]` item selectors for `Vec` fields,
//!   `?` on an `Option` field (required, so a path spells every container),
//!   `$Enum::Variant` segments for a variant's fields, nested `path => { … }`
//!   blocks sharing a prefix, a form-wide validator, and a `buttons:` block.
//!   Every field path it names is
//!   checked against the model's real shape at compile time, by emitting a
//!   witness expression per path — a typo is a compile error, not a runtime
//!   `no_such_path`.
//! - [`macro@path`] builds one compile-checked `Path<Model>`, by the same
//!   witness trick, for any field — whether or not a form mentions it. It is
//!   what keeps `reject_field` and friends from taking a bare `&str`.
//! - [`macro@using_fns`] supplies the handlers for one `render`, keyed by button
//!   name. It's a runtime-reconciled map rather than a struct because the
//!   reflection path has no per-form type to hang a struct literal on; which
//!   handlers validate first is read from their **arity**.
//!
//! Emitted code must reference formoxus items by fully-qualified,
//! module-canonical path (`::formoxus::form::FormSpec`, `::formoxus::error::ValidationError`,
//! …) — NOT via `::formoxus::prelude::…`. The defining module is a stabler
//! contract for generated code than the human-facing prelude.

use proc_macro::TokenStream;

mod form;
mod path;
mod using_fns;

// TODO: document the whole grammar here — the form-level keywords (`title`,
// `label_case`, `browser_validation`, `validator`, `buttons`) and every
// field-body key (`widget`, `label`, `min`, `max`, `min_length`, `max_length`,
// `pattern`, `required`, `adder` on a list's own path), including
// which constraints are legal on which value kinds.
// This is the only place a user can learn it: `FieldBody` holds the key table
// but is `pub(crate)`, and a one-line summary is no substitute. Wait until the
// constraint work is finished so the list is not documented mid-flight.
/// `form! { Model { … } }` — build a `FormSpec` for `Model`.
#[proc_macro]
pub fn form(input: TokenStream) -> TokenStream {
    form::impl_form(input.into()).into()
}

/// `path!(Model.field)` — a compile-checked [`Path`](../formoxus/path/struct.Path.html)
/// into `Model`.
///
/// Emits a witness borrow of the field beside the path's string form, so a
/// typo is `E0609` at the call site instead of a runtime `no_such_path`. Works
/// for any field, nested (`Model.venue.city`), items (`Model.answers[].text`),
/// through an `Option` (`Model.address?.city`, keyed `address.city`) or inside a
/// variant (`Model.shape.$Shape::Circle.radius`, keyed
/// `shape.$Circle.radius`), whether or not a `form!` names it.
///
/// **One item of a `Vec`, by index:** `Model.groups[g].$Group::Random.set[q]`,
/// where each index is any `usize` expression, evaluated where the path is
/// written. For an error about that one item, from a `validator:` or a
/// server: the index counts the items as the model has them, which for a
/// `Vec` is the order shown, and the form turns it into the item's key when
/// it places the error, wherever moves and removals have left the item. An
/// index past the end names no item, so the error is shown with the form's
/// own. Only on a `Vec`: a set iterates in an order that need not be the
/// order it is shown in, so `[i]` on one does not compile. `form!` takes
/// only `[]`, since its settings apply to every item.
///
/// ```ignore
/// fn one_group_per_question(a: &Assignment) -> Vec<ValidationError<Assignment>> {
///     // … for the second copy of a question, at group g, position q:
///     vec![ValidationError::at(
///         path!(Assignment.groups[g].$Group::All.questions[q]),
///         "Already in group 1.",
///     )]
/// }
/// ```
#[proc_macro]
pub fn path(input: TokenStream) -> TokenStream {
    path::impl_path(input.into()).into()
}

/// The handlers for one `render`, keyed by button name — see
/// [`using_fns`] for why this is a map and not a struct.
#[proc_macro]
pub fn using_fns(input: TokenStream) -> TokenStream {
    using_fns::impl_using_fns(input.into()).into()
}
