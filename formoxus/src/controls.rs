//! The control layer: what turns a leaf into markup.
//!
//! One component per leaf, and that is the load-bearing part. A component is
//! the unit of reactivity in Dioxus: a store read inside one subscribes *that*
//! scope. `FormMember::render` is a plain function with no scope of its own, so
//! reading a value there would subscribe whoever called it — and a single
//! keystroke would re-render the entire form. Spawning a component per leaf is
//! what keeps a write to one path local to one control.
//!
//! # "control", not "widget", and not "input"
//!
//! These were widgets while the derive path existed, where a widget really was
//! a trait a type implemented — `FieldWidget`, `DefaultWidget`,
//! `ProvidedWidget`. Nothing implements anything here: a control is an ordinary
//! Dioxus component taking `(values, props)`, and a custom one needs no trait
//! at all. The name outlived the thing it described.
//!
//! "Control" rather than "input" because an input is a specific HTML element
//! and most of these are not one — [`SelectInput`] renders a `<select>`,
//! [`TextareaInput`] a `<textarea>`. It is also the word the rest of the crate
//! already uses: [`ControlType`], [`ControlProps`], and `control:` in `form!`.
//! The components keep `…Input` in their names where that is what they
//! literally render.
//!
//! # Layout
//!
//! - [`types`] — the vocabulary: which controls exist, and the two prop
//!   bundles every control is handed. This is what `form!` speaks.
//! - [`values`] — reading and writing one path in the store.
//! - [`scalar`] — dispatch, deciding which control a `(value kind, control)`
//!   pair actually gets. The only place that decides which pairs work at all.
//! - [`html`], [`boolean`], [`select`] — the controls themselves.
//! - [`structure`] — controls that change a form's SHAPE rather than a value:
//!   which variant is chosen, and the rows of a list.
//! - [`errors`] — the error list rendered under every control.

pub mod boolean;
pub mod errors;
pub mod html;
pub mod scalar;
pub mod select;
pub mod structure;
pub mod types;
pub mod values;

// Flat, like the crate root: a caller reaching for `SelectChoice` should not
// have to know which file it happens to live in.
pub use boolean::BooleanInput;
pub use errors::FieldErrors;
pub use html::{HtmlInput, TextareaInput};
pub use scalar::ScalarInput;
pub use select::{ABSENT_DISPLAY, SelectChoice, SelectInput};
pub use structure::{AddRowButton, RemoveRowButton, VariantSelect};
pub use types::{ControlProps, ControlType, FieldProps, InputType};
pub use values::{get_current, write_value};
