//! Reflection-based forms for Dioxus: build a form at runtime from a model's
//! [`facet`] shape, instead of from a hand-written form struct.
//!
//! A model needs only `#[derive(Facet)]`. [`form!`](macro@form) declares the
//! form over it — title, labels, controls, validators, buttons — and
//! [`use_form`] turns that declaration into a live, reactive form. Values
//! convert through facet's own vtables, so there is no `FromStr`/`Display`
//! bound to satisfy and no per-form type to write.
//!
//! ```ignore
//! #[derive(Facet)]
//! struct Signup { email: String, password: String }
//!
//! let spec = form! { Signup { title: "Sign up", password: { control: password } } };
//! let form = use_form(spec);
//! rsx! { {form.render(using_fns! { submit: |model| async move { … } })} }
//! ```
//!
//! # How the pieces fit
//!
//! - [`FormSpec`] is the **declaration** — what `form!` produces. It is a
//!   schema, not state, and it never crosses a server-fn boundary.
//! - [`FormState`] is the **built tree**: plain data, no signals, testable with
//!   no Dioxus runtime. [`Form`] is the reactive handle over it.
//! - Leaf paths (`x.y`, `x.y[]`, `x.y[].z`) are the wire format. A form submits
//!   `(path, value)` pairs and the server rebuilds against the same
//!   [`FormSpec`] — see [`Submission`], which packages that side.
//!
//! The design reasoning behind all of this — including the parts that were
//! tried and abandoned — lives in `.claude/memory/`, starting from its
//! `MEMORY.md` index.

pub mod build;
pub mod buttons;
pub mod error;
pub mod fields;
pub mod form;
pub mod label_case;
pub mod members;
pub mod submission;
pub mod widgets;

/// `form! { Model { … } }` — build a [`FormSpec`] for `Model`.
///
/// A function-like macro, not a derive: it expands in the *consuming* crate, so
/// it can name the model's own type and capture runtime values from the scope
/// it sits in. Every field path it names is checked against the model's real
/// shape at compile time.
pub use formoxus_macros::form;

/// `using_fns! { save: |m| async move { … }, … }` — the handlers for one
/// [`Form::render`], keyed by button name.
///
/// Which closures validate first is read from their **arity**: `|m| …` receives
/// the model and only runs once it validates, `|| …` takes nothing and runs
/// regardless. Names are checked against the form's declared buttons at render,
/// not at compile time — there is no per-form type to hang a struct literal on.
pub use formoxus_macros::using_fns;

// A flat root, so `use formoxus::*` (and the test modules' `use crate::*`)
// reaches the whole vocabulary without knowing which module each name lives in.
pub use crate::error::{FieldError, FormError};
pub use buttons::{ButtonFn, ButtonSpec, ButtonType, Fns, Invocation};
pub use fields::{FieldValue, FormField};
pub use form::{
    FieldErrors, FieldSpec, Form, FormErrors, FormSpec, FormState, Handler, IntoSlot, Provider,
    UncheckedHandler, empty_form, form_for, handler, provider, unchecked_handler, use_form,
    use_form_values,
};
pub use members::{
    Edit, FieldSet, FormMember, ListSet, RenderCtx, ValuesByPath, VariantChoice, VariantSet,
    model_path,
};
pub use submission::Submission;
pub use widgets::ABSENT_DISPLAY;

/// The common surface: `use formoxus::prelude::*;`.
///
/// The crate root re-exports the same vocabulary flat, so the prelude is a
/// convenience rather than a separate contract. For precise imports, reach into
/// the defining module (`formoxus::form::FormSpec`, …).
pub mod prelude {
    pub use crate::error::{FieldError, FormError};
    pub use crate::form::{
        Form, FormSpec, FormState, Provider, empty_form, form_for, handler, provider,
        unchecked_handler, use_form, use_form_values,
    };
    pub use crate::submission::Submission;
    // Straight from the macro crate, not `crate::{form, …}`: `crate::form` names
    // both the module and the macro, so re-exporting it that way would put the
    // MODULE into every glob import of this prelude too. `formoxus_macros::form`
    // is only ever the macro.
    pub use formoxus_macros::{form, using_fns};
}
