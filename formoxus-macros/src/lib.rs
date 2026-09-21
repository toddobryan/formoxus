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
//!   title, per-field labels and widgets, `[]` row selectors for `Vec` fields,
//!   a form-wide validator, and a `buttons:` block. Every field path it names is
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
//! module-canonical path (`::formoxus::form::FormSpec`, `::formoxus::FieldError`,
//! …) — NOT via `::formoxus::prelude::…`. The defining module is a stabler
//! contract for generated code than the human-facing prelude.

use proc_macro::TokenStream;

mod form;
mod path;
mod using_fns;

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
/// for any field, nested (`Model.venue.city`) or rows (`Model.answers[].text`),
/// whether or not a `form!` names it.
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
