//! The `<select>` that chooses which enum variant a value takes.

use dioxus::prelude::*;
use formoxus_spec::{Attr, Tag};

use crate::bound_field::Variant;
use crate::label_case::{LabelCase, ToCase};
use crate::widgets::Select;
use crate::widgets::widget_props::WidgetProps;

use super::field_row::{LabelSide, field_row};
use super::select::ABSENT_DISPLAY;

/// The `<select>` a [`VariantNode`](crate::nodes::VariantNode) renders to ask which
/// variant a value takes.
///
/// A SHAPE choice, not a value choice — contrast [`Select`],
/// which picks one of a field's legal values and is a leaf. The answer is not
/// a leaf's value but the enum's structure: it goes out as an
/// [`Edit`](crate::nodes::Edit), through the bound field's `choose`, and
/// rebuilds the form beneath it. It still has a `name`, the enum's own path,
/// so a native post carries the choice and a test can find the select (#17).
#[component]
pub fn VariantSelect(
    variants: Vec<&'static str>,
    selected: Option<String>,
    /// The form's resolved casing, for the variant names below. A prop rather
    /// than a `defaults()` call, so the per-form tier is not skipped — see
    /// [`crate::buttons::ButtonSpec::label`].
    label_case: LabelCase,
    props: WidgetProps<Variant>,
) -> Element {
    let required = props.base_attrs.contains(Attr::Required);
    let attrs: Vec<Attribute> = props.base_attrs.merge_with_attrs(props.extra_attrs.clone());
    let field = props.field();

    let control = rsx! {
        select {
            class: props.control_class::<Select>(Tag::Select),
            id: "{props.id()}",
            // The enum's own path, where `collect_values` already sends the
            // variant's name: so a native post would carry the choice too,
            // and a test can find the picker by `name` like every other
            // control (#17).
            name: "{props.path()}",
            onchange: move |e: FormEvent| {
                let v = e.value();
                let variant = (!v.is_empty()).then_some(v);
                field.choose(variant.as_deref());
            },
            ..attrs,
            // Required + unchosen: an unselectable placeholder that keeps the browser's
            // own validation on the hook. Not required: a real "--none--" the user can
            // pick, which routes through the empty arm above to Unchosen.
            if required && selected.is_none() {
                option { value: "", selected: true, disabled: true, hidden: true, "Choose..." }
            } else if !required {
                option { value: "", selected: selected.is_none(), "{ABSENT_DISPLAY}" }
            }
            for v in variants {
                option {
                    value: "{v}",
                    selected: selected.as_deref() == Some(v),
                    "{v.to_case(label_case)}"
                }
            }
        }
    };
    // The star annotates the LABEL, so it only appears when there is one.
    // Rendered inside a `VariantNode`'s fieldset there isn't: the legend
    // carries both, and a lone `*` floating in front of the select reads as
    // belonging to nothing.
    field_row::<Select>(&props, required, LabelSide::Before, control)
}
