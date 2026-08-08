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
//! Emitted code must reference formoxus items by fully-qualified path
//! (`::formoxus::FormField`, `::formoxus::FieldProps`, `::formoxus::render_default`, …)
//! so the derive works from any consumer crate.
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

mod error;
mod field_meta;
mod form;
mod form_meta;

/// Derive `Form`. See the module docs — currently a compiling no-op skeleton,
/// ready for the real codegen. The trait `formoxus::Form` and this derive share a
/// name but live in different namespaces (type vs. macro).
#[proc_macro_derive(Form, attributes(form))]
pub fn derive_form(input: TokenStream) -> TokenStream {
    form::derive_form(input.into()).into()
}
