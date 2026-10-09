//! The `<select>` that chooses which enum variant a value takes.

use dioxus::prelude::*;
use formoxus_attrs::{Attr, FieldType};

use crate::fields::FieldAttrs;
use crate::label_case::{LabelCase, ToCase};
use crate::members::Edit;
use crate::widgets::FieldProps;

use super::errors::FieldErrors;
use super::select::ABSENT_DISPLAY;

/// The `<select>` a [`VariantSet`](crate::members::VariantSet) renders to ask which
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
    let class = field_attrs.class("fx-control fx-select");
    let field_class = props.field_class();

    let FieldProps {
        path,
        label: label_text,
        errors,
    } = props;

    let required = field_attrs.contains(Attr::Required);

    rsx! {
        label { class: field_class,
            // The star annotates the LABEL, so it only appears when there is
            // one. Rendered inside a `VariantSet`'s fieldset there isn't: the
            // legend carries both, and a lone `*` floating in front of the
            // select reads as belonging to nothing.
            if let Some(text) = label_text {
                // Inside the label span, not beside it: a consumer who makes
                // `.fx-field-label` a block would otherwise push a lone asterisk
                // onto its own line. `aria-hidden` because the asterisk is a
                // VISUAL convention — `required` on the control is what a screen
                // reader reads, so this would only add noise to the name.
                span { class: "fx-field-label",
                    "{text}"
                    if required {
                        span { class: "fx-required", aria_hidden: "true", " *" }
                    }
                }
            }
            select {
                class: class,
                onchange: move |e: FormEvent| {
                    let v = e.value();
                    let variant = (!v.is_empty()).then_some(v);
                    on_edit.call(Edit::new_choose_variant(&path, variant.as_deref()));
                },
                ..field_attrs.merge_with_attrs(FieldType::Text, attrs),
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
            FieldErrors { errors }
        }
    }
}
