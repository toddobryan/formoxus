//! The widget layer: what turns a leaf into markup.
//!
//! One component per leaf, and that is the load-bearing part. A component is
//! the unit of reactivity in Dioxus: a store read inside one subscribes *that*
//! scope. `FormMember::render` is a plain function with no scope of its own, so
//! reading a value there would subscribe whoever called it — and a single
//! keystroke would re-render the entire form. Spawning a component per leaf is
//! what keeps a write to one path local to one widget.
//!
//! # "widget", not "control" and not "input"
//!
//! Not one of these renders a bare element. [`Input`] is the plainest, and even
//! it wraps its `<input>` in a `<label class="form-field">` with a caption, a
//! required marker and an error list — the only exception is `type="hidden"`,
//! which renders bare precisely because there is nothing to wrap. So "control",
//! which suggests the native element itself, describes none of them, and
//! "input" describes only a part of one. A widget is the whole assembly.
//!
//! That leaves room for the interesting ones to be the same kind of thing
//! rather than a separate category. A combobox with a search field, or a
//! Markdown editor with a live preview, is a bigger widget — not a different
//! species — and [`WidgetType::Custom`] is how a consumer supplies one.
//! [`WidgetType::RadioGroup`] shows that built-in does not mean simple: there
//! is no `<radiogroup>`, only *n* radios sharing a `name`. Django draws the
//! line in the same place, and calls the whole range `Widget`.
//!
//! **The word is reused, not inherited.** Under the derive path a widget was a
//! trait a type implemented — `FieldWidget`, `DefaultWidget`, `ProvidedWidget`.
//! Nothing implements anything now: a widget is an ordinary Dioxus component
//! taking `(values, props)`, and a custom one needs no trait at all. Anything
//! in the history saying otherwise predates that.
//!
//! Each component is then named for the element at its core — [`Input`],
//! [`Select`], [`Textarea`], [`Checkbox`] — since that is what a reader is
//! looking for when they go hunting.
//!
//! # Layout
//!
//! - [`types`] — the vocabulary: which widgets exist, and the two prop
//!   bundles every widget is handed. This is what `form!` speaks.
//! - [`values`] — reading and writing one path in the store.
//! - [`scalar`] — dispatch, deciding which widget a `(value kind, widget)`
//!   pair actually gets. The only place that decides which pairs work at all.
//! - [`input`], [`textarea`], [`checkbox`], [`select`] — the widgets
//!   themselves, one module per element.
//! - [`structure`] — widgets that change a form's SHAPE rather than a value:
//!   which variant is chosen, and the rows of a list.
//! - [`errors`] — the error list rendered under every widget.

pub mod checkbox;
pub mod errors;
pub mod input;
pub mod radio_group;
pub mod scalar;
pub mod select;
pub mod structure;
pub mod textarea;
pub mod types;
pub mod values;

// Flat, like the crate root: a caller reaching for `SelectChoice` should not
// have to know which file it happens to live in.
pub use checkbox::Checkbox;
pub use errors::FieldErrors;
pub use input::Input;
pub use radio_group::RadioGroup;
pub use scalar::ScalarWidget;
pub use select::{ABSENT_DISPLAY, Select, SelectChoice};
pub use structure::{AddRowButton, RemoveRowButton, VariantSelect};
pub use textarea::Textarea;
pub use types::{FieldProps, InputType, WidgetProps, WidgetType};
pub use values::{get_current, write_value};
