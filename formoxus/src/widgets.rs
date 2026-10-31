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
//! it puts its `<input>` in a `<div class="fx-form-field">` beside a caption, a
//! required marker and an error list — the only exception is `type="hidden"`,
//! which renders bare precisely because there is nothing to wrap. So "control",
//! which suggests the native element itself, describes none of them, and
//! "input" describes only a part of one. A widget is the whole assembly.
//!
//! That leaves room for the interesting ones to be the same kind of thing
//! rather than a separate category. A combobox with a search field, or a
//! Markdown editor with a live preview, is a bigger widget — not a different
//! species — and a dev's own [`Widget`] impl is how one is supplied.
//! [`RadioGroup`] shows that built-in does not mean simple: there
//! is no `<radiogroup>`, only *n* radios sharing a `name`. Django draws the
//! line in the same place, and calls the whole range `Widget`.
//!
//! **A widget is a struct implementing [`Widget`]**, whose `pub` fields are
//! its props, and a field holds one as an [`AnyWidget`]. Its `render` is
//! handed one [`WidgetProps`] and is a plain method, so it must not read the
//! store itself: each built-in passes its props on to a component of its own,
//! which is where the per-leaf scope above comes from. A dev's widget that
//! reads `values` should do the same. (Under the long-gone derive path, the
//! traits were `FieldWidget`, `DefaultWidget` and `ProvidedWidget`, which this
//! is not a return to.)
//!
//! Each component is then named for the element at its core — `InputView`,
//! `SelectView`, `TextareaView`, `RadioGroupView` — since that is what a reader is
//! looking for when they go hunting. A checkbox is an `InputView`, routed by its
//! type to its own body. [`RadioGroup`] is the one named
//! for something HTML has no element for, which is the point made just above.
//!
//! # Layout
//!
//! - [`types`] — [`Choice`], one legal value a chooser offers.
//! - [`values`] — reading and writing one path in the store.
//! - [`widget`] — the [`Widget`] trait every widget implements, and
//!   [`AnyWidget`], the form a field holds one in.
//! - `widget_props` (re-exported as [`WidgetProps`]) — what a widget is handed
//!   to render a field.
//! - [`field_row`](mod@field_row) — the row a one-control widget renders
//!   around its control: wrapper, caption and required marker, errors. Public
//!   so a dev's widget can look like a built-in one without copying the layout.
//! - [`input`] (every `<input>`, the checkbox included), [`textarea`],
//!   [`select`], [`radio_group`] — the widgets themselves, one module per
//!   element.
//! - [`variant_select`] and [`row_buttons`] — the widgets that change a form's
//!   SHAPE rather than a value: which variant is chosen, and the rows of a list.
//! - [`errors`] — the error list rendered under every widget.

pub mod errors;
pub mod field_row;
pub(crate) mod icons;
pub mod input;
pub mod radio_group;
pub mod row_buttons;
pub mod select;
pub mod textarea;
pub mod types;
pub mod values;
pub mod variant_select;
pub mod widget;
pub(crate) mod widget_props;

// Flat, like the crate root: a caller reaching for `Choice` should not
// have to know which file it happens to live in.
pub use errors::FieldErrors;
pub use field_row::{LabelSide, field_label, field_row, required_marker};
pub use input::{
    CheckboxInput, ColorInput, DateInput, DatetimeLocalInput, EmailInput, HiddenInput, MonthInput,
    NumberInput, PasswordInput, SearchInput, TelInput, TextInput, TimeInput, UrlInput, WeekInput,
};
pub use radio_group::RadioGroup;
pub use row_buttons::{AddRowButton, MoveRowButtons, RemoveRowButton};
pub use select::{ABSENT_DISPLAY, Select};
pub use textarea::Textarea;
pub use types::{Choice, InputType};
pub use values::{get_current, write_value};
// Defined in `nodes`, re-exported here because a custom widget cannot be
// written without it — so `use formoxus::widgets::*` is the whole kit.
pub use crate::nodes::ValuesStore;
pub use variant_select::VariantSelect;
pub use widget::{AnyWidget, Widget};
pub use widget_props::WidgetProps;

#[cfg(test)]
mod tests {
    use super::{
        CheckboxInput, ColorInput, DateInput, DatetimeLocalInput, EmailInput, HiddenInput,
        MonthInput, NumberInput, PasswordInput, RadioGroup, SearchInput, Select, TelInput,
        TextInput, Textarea, TimeInput, UrlInput, WeekInput, Widget,
    };
    use formoxus_spec::WidgetName;
    use googletest::prelude::*;
    use std::collections::HashSet;

    /// The row `form!` reads for `W`: the `WidgetName` whose `form_name` is
    /// `W::NAME`, and the last segment of `W`'s type name.
    fn row_of<W: Widget>() -> (Option<WidgetName>, &'static str) {
        let type_name = std::any::type_name::<W>();
        let last = type_name.rsplit("::").next().unwrap_or(type_name);
        (WidgetName::from_form_name(W::NAME), last)
    }

    /// Every built-in's `NAME` finds a row, and that row's `type_name` is the
    /// built-in itself. What makes `widget: tel` render a `TelInput`: `form!`
    /// emits `type_name` from the row, and nothing else would notice a row
    /// naming the wrong type. Each `impl` picks its own `WidgetName` variant
    /// for `NAME`, so this catches one that picked another's too. And every
    /// row is some built-in's, so none names a type that is not one.
    #[gtest]
    fn each_builtins_row_names_the_builtin_itself() {
        let rows = [
            row_of::<TextInput>(),
            row_of::<PasswordInput>(),
            row_of::<HiddenInput>(),
            row_of::<NumberInput>(),
            row_of::<EmailInput>(),
            row_of::<TelInput>(),
            row_of::<UrlInput>(),
            row_of::<SearchInput>(),
            row_of::<ColorInput>(),
            row_of::<DateInput>(),
            row_of::<TimeInput>(),
            row_of::<DatetimeLocalInput>(),
            row_of::<MonthInput>(),
            row_of::<WeekInput>(),
            row_of::<CheckboxInput>(),
            row_of::<Textarea>(),
            row_of::<Select>(),
            row_of::<RadioGroup>(),
        ];
        let mut seen = HashSet::new();
        for (row, type_name) in rows {
            expect_that!(
                row.map(WidgetName::type_name),
                some(eq(type_name)),
                "{type_name}"
            );
            seen.extend(row);
        }
        expect_that!(seen.len(), eq(WidgetName::ALL.len()));
    }
}
