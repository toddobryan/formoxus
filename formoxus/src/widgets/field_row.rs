//! The row every one-control widget renders, and its parts: the caption and
//! required marker, shared by every widget so each is rendered, and given its
//! class, in one place.

use dioxus::prelude::*;
use formoxus_spec::{BuiltinWidget, FixedKey, Tag};

use crate::classes::Classes;
use crate::widgets::{FieldErrors, Widget, WidgetProps};

/// Which side of its form control a field's caption goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LabelSide {
    /// Caption, then control: how every text-like input, a textarea and a
    /// select read.
    Before,
    /// Control, then caption: a checkbox reads "☐ I agree", not "I agree ☐".
    After,
}

/// `div > label + control + errors`, around a `control` the widget built.
///
/// The widget still sets the control's own `class`, `id` and attributes: a
/// wrapper cannot add attributes to a finished `Element`. A plain fn, not a
/// `#[component]` taking `children`, so it adds no scope and changes no
/// markup. The wrapper is unconditional and the caption is not (see
/// [`field_label`]). `marked` is whether the caption gets the required
/// marker, which is `required` for most widgets but `required_true` for a
/// checkbox. A hidden input (bare) and a radio group (`fieldset`/`legend`)
/// do not use it.
pub(crate) fn field_row<W: Widget>(
    props: &WidgetProps,
    marked: bool,
    side: LabelSide,
    control: Element,
) -> Element {
    let label = field_label::<W>(
        &props.id,
        props.label.clone(),
        marked,
        &props.classes,
        props.is_invalid(),
    );
    let (before, after) = match side {
        LabelSide::Before => (label, rsx! {}),
        LabelSide::After => (rsx! {}, label),
    };
    rsx! {
        div {
            class: props.part_class::<W>(Tag::Div),
            {before}
            {control}
            {after}
            FieldErrors {
                field_id: &props.id,
                errors: props.errors.clone(),
                classes: props.classes.clone(),
            }
        }
    }
}

/// The caption, pointing at the form control by `id`, with the required marker
/// when `marked`. Nothing at all when there is no caption, so the marker never
/// floats on its own: an asterisk with nothing to qualify means nothing.
///
/// The marker is INSIDE the label, not a sibling: a consumer who makes
/// `.fx-field-label` a block (the natural choice above an input) would
/// otherwise push a lone asterisk onto its own line.
pub(crate) fn field_label<W: Widget>(
    id: &str,
    text: Option<String>,
    marked: bool,
    classes: &Classes,
    invalid: bool,
) -> Element {
    let label_class = classes.part_class::<W>(Tag::Label, invalid);
    rsx! {
        if let Some(text) = text {
            label {
                class: label_class,
                r#for: "{id}",
                "{text}"
                if marked {
                    {required_marker(classes)}
                }
            }
        }
    }
}

/// The ` *` after a required field's caption, in a `<label>` or a `<legend>`.
///
/// `aria-hidden` because it is a VISUAL convention only: the `required`
/// attribute is what a screen reader announces, so the asterisk would just be
/// noise in the accessible name.
pub(crate) fn required_marker(classes: &Classes) -> Element {
    let class_for_required = classes.fixed_class(FixedKey::RequiredMarker, false);
    rsx! {
        span { class: class_for_required, aria_hidden: "true", " *" }
    }
}
