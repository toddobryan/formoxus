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
//! **A widget is a struct implementing [`Widget`] and [`LeafWidget`]**, whose
//! `pub` fields are its props, and a field holds one as an [`AnyWidget`].
//! `Widget` is what every widget has (its name, parts and the attributes it
//! passes through); `LeafWidget` is what renders one value. Its `render` is
//! handed one [`WidgetProps`] and is a plain method, so it must not read the
//! store itself: each built-in passes its props on to a component of its own,
//! which is where the per-leaf scope above comes from. A dev's widget that
//! reads its value or its errors should do the same. (Under the long-gone
//! derive path, the traits were `FieldWidget`, `DefaultWidget` and
//! `ProvidedWidget`, which this is not a return to.)
//!
//! # Writing a widget of your own
//!
//! **Its value goes through a [`FieldHandle`],** from
//! [`WidgetProps::field_handle`]: [`current`](FieldHandle::current) to read
//! it, and one of two handler pairs to write it, by what the element is.
//!
//! - Something the user TYPES into (an `<input>`, a `<textarea>`, an editor):
//!   `oninput: field.on_input()` and `onblur: field.on_blur()`. Both: the form
//!   checks such a field when the user leaves it, so without `on_blur` it is
//!   never checked before submit, unless it already shows an error, which
//!   each keystroke then rechecks.
//! - Something the user CHOOSES with (a `<select>`, a checkbox, a radio):
//!   `onchange: field.on_change()` alone. A choice has no typing to wait out,
//!   so it counts as leaving the field, and is checked at once.
//!
//! An element whose event does not carry the value as `event.value()` calls
//! [`input`](FieldHandle::input), [`change`](FieldHandle::change) and
//! [`blur`](FieldHandle::blur) from a handler of its own. The handle is the
//! only way to write: it is what tells the form a value changed, so a widget
//! gets the form's checking as the user goes (`live_validation:`) for free.
//!
//! **Its errors come from the same props.** [`current_attrs`](WidgetProps::current_attrs)
//! has `aria-invalid` and `aria-describedby` already set from them, so a
//! widget that spreads it onto its element needs nothing else;
//! [`validity`](WidgetCommon::validity) is the three-way answer under it, and
//! [`is_invalid`](WidgetCommon::is_invalid) the yes-or-no a class wants.
//! [`FieldErrors`] renders the messages, as every built-in does, under the
//! element. Each of these reads only this field's errors, so another field's
//! error never wakes it.
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
//! - [`widget`] — the [`Widget`] trait every widget implements, the
//!   [`LeafWidget`] trait for one that renders one value, and [`AnyWidget`],
//!   the form a field holds one in.
//! - `widget_props` (re-exported as [`WidgetProps`] and [`WidgetCommon`]) —
//!   what a widget is handed to render a field, and the part of it every
//!   widget's props share, which `WidgetProps` derefs to.
//! - [`field_row`](mod@field_row) — the row a one-control widget renders
//!   around its control: wrapper, caption and required marker, errors. Public
//!   so a dev's widget can look like a built-in one without copying the layout.
//! - [`input`] (every `<input>`, the checkbox included), [`textarea`],
//!   [`select`], [`radio_group`] — the widgets themselves, one module per
//!   element.
//! - [`variant_select`] and [`item_buttons`] — the widgets that change a form's
//!   SHAPE rather than a value: which variant is chosen, and the items of a list.
//! - [`errors`] — the error list rendered under every widget.

pub mod checkbox_multiple;
pub mod errors;
pub(crate) mod field_handle;
pub mod field_row;
pub mod help;
pub(crate) mod icons;
pub mod input;
pub mod item_buttons;
pub mod radio_group;
pub mod select;
pub mod select_multiple;
pub mod set_item;
pub mod textarea;
pub mod types;
pub mod variant_select;
pub mod widget;
pub(crate) mod widget_props;

// Flat, like the crate root: a caller reaching for `Choice` should not
// have to know which file it happens to live in.
pub use crate::field_attrs::Validity;
pub use checkbox_multiple::CheckboxMultiple;
pub use errors::FieldErrors;
pub use field_handle::FieldHandle;
pub use field_row::{LabelSide, field_label, field_row, required_marker};
pub use help::{described_by, field_help, help_id};
pub use input::{
    CheckboxInput, ColorInput, DateInput, DatetimeLocalInput, EmailInput, HiddenInput, MonthInput,
    NumberInput, PasswordInput, RangeInput, SearchInput, TelInput, TextInput, TimeInput, UrlInput,
    WeekInput,
};
pub use item_buttons::{
    AddItemButton, AddItemProps, DraftAdder, EditItemButton, ItemAdder, MoveItemButtons,
    RemoveItemButton,
};
pub use radio_group::RadioGroup;
pub use select::{ABSENT_DISPLAY, Select};
pub use select_multiple::SelectMultiple;
pub use set_item::{ItemText, ItemView, SetItemProps};
pub use textarea::Textarea;
pub use types::{Choice, InputType};
// Defined in `nodes`, re-exported here because a custom widget cannot be
// written without them — so `use formoxus::widgets::*` is the whole kit. A
// custom adder (`AddItemProps`, above) sends an `Edit`, and fills its item with
// an `ItemValue`.
pub use crate::nodes::{Edit, ErrorsStore, ItemValue, ValuesStore};
pub use variant_select::VariantSelect;
pub use widget::{AnySetWidget, AnyWidget, LeafWidget, SetWidget, Widget};
pub use widget_props::{SetWidgetProps, WidgetCommon, WidgetProps};

#[cfg(test)]
mod tests {
    use super::{
        CheckboxInput, ColorInput, DateInput, DatetimeLocalInput, EmailInput, HiddenInput,
        MonthInput, NumberInput, PasswordInput, RadioGroup, RangeInput, SearchInput, Select,
        TelInput, TextInput, Textarea, TimeInput, UrlInput, WeekInput, Widget,
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
            row_of::<RangeInput>(),
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
