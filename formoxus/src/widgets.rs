//! The widget layer: what turns a leaf into markup.
//!
//! One component per leaf, and that is the load-bearing part. A component is
//! the unit of reactivity in Dioxus: a store read inside one subscribes *that*
//! scope. `Node::render` is a plain function with no scope of its own, so
//! reading a value there would subscribe whoever called it — and a single
//! keystroke would re-render the entire form. Spawning a component per leaf is
//! what keeps a write to one path local to one widget.
//!
//! # "widget", not "control" and not "input"
//!
//! Not one of these renders a bare element. `InputView` is the plainest, and even
//! it wraps its `<input>` in a `<label class="fx-form-field">` with a caption, a
//! required marker and an error list — the only exception is `type="hidden"`,
//! which renders bare precisely because there is nothing to wrap. So "control",
//! which suggests the native element itself, describes none of them, and
//! "input" describes only a part of one. A widget is the whole assembly.
//!
//! That leaves room for the interesting ones to be the same kind of thing
//! rather than a separate category. A combobox with a search field, or a
//! Markdown editor with a live preview, is a bigger widget — not a different
//! species — and [`WidgetType::Custom`] is how a consumer supplies one.
//! [`BuiltinWidget::RadioGroup`] shows that built-in does not mean simple: there
//! is no `<radiogroup>`, only *n* radios sharing a `name`. Django draws the
//! line in the same place, and calls the whole range `Widget`.
//!
//! **The word is reused, not inherited.** Under the derive path a widget was a
//! trait a type implemented — `FieldWidget`, `DefaultWidget`, `ProvidedWidget`.
//! Nothing implements anything now: a widget is an ordinary Dioxus component
//! taking `(values, props)`, and a custom one needs no trait at all. Anything
//! in the history saying otherwise predates that.
//!
//! Each component is then named for the element at its core — `InputView`,
//! `SelectView`, `TextareaView`, `RadioGroupView` — since that is what a reader is
//! looking for when they go hunting. A checkbox is an `InputView`, routed by its
//! type to its own body. [`RadioGroup`] is the one named
//! for something HTML has no element for, which is the point made just above.
//!
//! # Layout
//!
//! - [`types`] — the vocabulary: which widgets exist, and the two prop
//!   bundles every widget is handed. This is what `form!` speaks.
//! - [`values`] — reading and writing one path in the store.
//! - [`widget`] — the [`Widget`] trait every widget implements, and
//!   [`AnyWidget`], the form a field holds one in.
//! - [`widget_props`] — what a widget is handed to render a field.
//! - `label` (crate-private) — the caption and required marker every widget
//!   shares.
//! - [`input`] (every `<input>`, the checkbox included), [`textarea`],
//!   [`select`], [`radio_group`] — the widgets themselves, one module per
//!   element.
//! - [`variant_select`] and [`row_buttons`] — the widgets that change a form's
//!   SHAPE rather than a value: which variant is chosen, and the rows of a list.
//! - [`errors`] — the error list rendered under every widget.

pub mod errors;
pub mod input;
pub(crate) mod label;
pub mod radio_group;
pub mod row_buttons;
pub mod select;
pub mod textarea;
pub mod types;
pub mod values;
pub mod variant_select;
pub mod widget;
pub mod widget_props;

// Flat, like the crate root: a caller reaching for `Choice` should not
// have to know which file it happens to live in.
pub use errors::FieldErrors;
pub use formoxus_spec::{BuiltinWidget, RenderedInputType};
pub use input::{
    CheckboxInput, ColorInput, DateInput, DatetimeLocalInput, EmailInput, HiddenInput, MonthInput,
    NumberInput, PasswordInput, SearchInput, TelInput, TextInput, TimeInput, UrlInput, WeekInput,
};
pub use radio_group::RadioGroup;
pub use row_buttons::{AddRowButton, RemoveRowButton};
pub use select::{ABSENT_DISPLAY, Select};
pub use textarea::Textarea;
pub use types::{Choice, FieldProps, InputType, WidgetProps, WidgetType};
pub use values::{get_current, write_value};
// Defined in `nodes`, re-exported here because a custom widget cannot be
// written without it — so `use formoxus::widgets::*` is the whole kit.
pub use crate::nodes::ValuesStore;
pub use variant_select::VariantSelect;
pub use widget::{AnyWidget, Widget};
