//! Proc-macro crate for **formoxus** (GH #7). Consumers depend on the `formoxus`
//! facade crate (with its `derive` feature, on by default), which re-exports the
//! derive from here — they don't depend on this crate directly.
//!
//! `#[derive(Form)]` turns a plain *declaration* struct into the reactive form
//! machinery that today is hand-written in `crates/formoxus/examples/form_example.rs`
//! (that example is the SPEC — keep them in sync as codegen lands). Given:
//!
//! ```ignore
//! #[derive(Form)]
//! #[form(model = SampleModel)]
//! struct SampleForm {
//!     text: String,
//!     count: i32,
//!     max: Option<i32>,      // optional (type-driven)
//!     flag: bool,
//!     opt_flag: Option<bool>,
//! }
//! ```
//!
//! the derive should generate the entities the example writes by hand:
//! - the `FormField`-wrapped state struct (+ `errors: Vec<FormError>`), `Store`-derivable;
//! - `from_model(&Model)` — TYPE-DRIVEN seeding: plain `T` → `FormField::with_value`,
//!   `Option<T>` → `FormField::with_optional`;
//! - `validate(&mut self) -> Option<Model>` — plain `T` → `.required()`,
//!   `Option<T>` → `.optional()`; the cross-field `validate_form` stays HAND-WRITTEN;
//! - the `render` component — `render_default(field.into(), FieldProps { .. })` per
//!   field, with the **optional-bool ⇒ select** special-case (an `Option<bool>` field
//!   routes through the select widget, not the checkbox default), and
//!   `#[form(component = W)]` → `W::render(field.into(), ..)`.
//!
//! Required-ness is TYPE-DRIVEN (no `#[form(required)]`/`optional]`): a field is
//! required iff its type is not `Option<…>`. Value-level validators (`non_empty`, …)
//! are orthogonal and opt-in via `#[form(non_empty)]` etc.
//!
//! Emitted code must reference formoxus items by fully-qualified, module-canonical
//! path (`::formoxus::fields::FormField`, `::formoxus::form::FormState`,
//! `::formoxus::error::FormError`, …) — NOT via `::formoxus::prelude::…`. The crate
//! root no longer re-exports these; the canonical home is the defining module, which
//! is a stabler contract for generated code than the human-facing prelude.
//!
//! Right now this is a SKELETON: it parses the input and emits nothing, so it
//! compiles and is a no-op. Fill in the real codegen incrementally.
//!
//! Open decisions left for the real implementation:
//! - **Naming.** The declaration struct is `SampleForm`; the example names the
//!   state struct `SampleFormState` and the component `SampleForm`. Decide what the
//!   derive emits and how it avoids a name clash (struct vs. `#[component] fn`).
//! - **Label source.** `FieldProps.label` — from the field ident (Title Case via
//!   `heck`), or a `#[form(label = "…")]` literal override?
//! - **`placeholder`** — always `None`, or a `#[form(placeholder = "…")]` literal?
//! - **Attribute parsing** — `darling` is wired (as in `surreal-table-macros`) for
//!   `#[form(model = …, component = …, label = …, non_empty, …)]`.

use proc_macro::TokenStream;

mod container;
mod error;
mod field_container_meta;
mod field_meta;
mod field_set;
mod field_set_meta;
mod form;
mod form2;
mod form_meta;

/// Derive `Form`. See the module docs — currently a compiling no-op skeleton,
/// ready for the real codegen. The trait `formoxus::Form` and this derive share a
/// name but live in different namespaces (type vs. macro).
#[proc_macro_derive(Form, attributes(form))]
pub fn derive_form(input: TokenStream) -> TokenStream {
    form::derive_form(input.into()).into()
}

// `form` is here too, not just `field_set`: `FieldMeta` (shared with `Form`)
// always parses per-field attributes under the `form` namespace regardless of
// the container, so a field on a `FieldSet`-derived struct still writes
// `#[form(component = ..., ...)]` — omitting it here left rustc rejecting that
// as an unknown attribute before darling ever saw it.
#[proc_macro_derive(FieldSet, attributes(field_set, form))]
pub fn derive_fieldset(input: TokenStream) -> TokenStream {
    field_set::derive_field_set(input.into()).into()
}

#[proc_macro]
pub fn form2(input: TokenStream) -> TokenStream {
    form2::impl_form2(input.into()).into()
}
