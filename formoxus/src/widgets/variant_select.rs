//! The `<select>` that chooses which enum variant a value takes.

use dioxus::prelude::*;
use formoxus_spec::{Attr, BuiltinWidget};

use crate::field_attrs::FieldAttrs;
use crate::label_case::{LabelCase, ToCase};
use crate::nodes::Edit;
use crate::widgets::FieldProps;

use super::errors::FieldErrors;
use super::label::field_label;
use super::select::ABSENT_DISPLAY;

/// The `<select>` a [`VariantSet`](crate::nodes::VariantSet) renders to ask which
/// variant a value takes.
///
/// A SHAPE choice, not a value choice — contrast [`Select`](super::Select),
/// which picks one of a field's legal values and is a leaf. This one carries no
/// `name`, because the answer is not a value to submit: it goes out as an
/// [`Edit`] and rebuilds the form beneath it.
#[component]
pub fn VariantSelect(
    field_attrs: FieldAttrs,
    variants: Vec<&'static str>,
    selected: Option<String>,
    on_edit: Callback<Edit>,
    /// The form's resolved casing, for the variant names below. A prop rather
    /// than a `defaults()` call, so the per-form tier is not skipped — see
    /// [`crate::buttons::ButtonSpec::label`].
    label_case: LabelCase,
    props: FieldProps,
    #[props(extends = select)] attrs: Vec<Attribute>,
) -> Element {
    let my_widget = BuiltinWidget::Select;
    let is_invalid = props.is_invalid();
    let class = field_attrs.class(props.classes.part(my_widget, "select"), is_invalid);
    let field_class = props.classes.part_class(my_widget, "div", is_invalid);

    let required = field_attrs.contains(Attr::Required);

    let label = field_label(
        &props.id,
        props.label,
        required,
        &props.classes,
        is_invalid,
        my_widget,
    );

    rsx! {
        div {
            class: field_class,
            // The star annotates the LABEL, so it only appears when there is
            // one. Rendered inside a `VariantSet`'s fieldset there isn't: the
            // legend carries both, and a lone `*` floating in front of the
            // select reads as belonging to nothing.
            {label}
            select {
                class: class,
                id: &props.id,
                onchange: move |e: FormEvent| {
                    let v = e.value();
                    let variant = (!v.is_empty()).then_some(v);
                    on_edit.call(Edit::new_choose_variant(&props.path, variant.as_deref()));
                },
                ..field_attrs.merge_with_attrs(attrs),
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
            FieldErrors { field_id: &props.id, errors: props.errors, classes: props.classes.clone() }
        }
    }
}
