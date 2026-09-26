//! Widgets that change a form's SHAPE rather than a value: which enum
//! variant is chosen, and the rows of a list.

use dioxus::prelude::*;

use crate::error::FieldError;
use crate::label_case::{LabelCase, ToCase};
use crate::members::Edit;

use super::errors::FieldErrors;
use super::select::ABSENT_DISPLAY;

#[component]
pub fn VariantSelect(
    path: String,
    label: Option<String>,
    required: bool,
    errors: Vec<FieldError>,
    variants: Vec<&'static str>,
    selected: Option<String>,
    /// The form's resolved casing, for the variant names below. A prop rather
    /// than a `defaults()` call, so the per-form tier is not skipped — see
    /// [`crate::buttons::ButtonSpec::label`].
    label_case: LabelCase,
    on_edit: Callback<Edit>,
) -> Element {
    let label_text = label;

    rsx! {
        label { class: "form-field",
            // The star annotates the LABEL, so it only appears when there is
            // one. Rendered inside a `VariantSet`'s fieldset there isn't: the
            // legend carries both, and a lone `*` floating in front of the
            // select reads as belonging to nothing.
            if let Some(text) = label_text {
                span { class: "field-label", "{text}" }
                if required {
                    span { class: "required", " *" }
                }
            }
            select {
                required,
                onchange: move |e: FormEvent| {
                    let v = e.value();
                    let variant = (!v.is_empty()).then_some(v);
                    on_edit.call(Edit::new_choose_variant(&path, variant.as_deref()));
                },
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

/// The widget that appends a row to a list.
///
/// Like [`VariantSelect`], it reads nothing from the value store — adding a row
/// is a change to the form's *shape*, so all it does is put an [`Edit`] on the
/// wire. `type="button"` is load-bearing: inside a `<form>` a bare `<button>`
/// defaults to `type="submit"`, so omitting it would submit the form instead of
/// adding a row.
#[component]
pub fn AddRowButton(path: String, on_edit: Callback<Edit>) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "add-row",
            onclick: move |_| {
                // Append. `before` exists for mid-list insertion, which needs a
                // widget between every pair of rows — a UI question that hasn't
                // been answered yet, not a limitation of the edit.
                on_edit.call(Edit::AddRow { path: path.clone(), before: None });
            },
            "Add"
        }
    }
}

/// The widget that drops one row from a list.
///
/// Addressed by POSITION, not by the row's key: the list is what applies the
/// edit and it works in terms of `rows`, so a position is what it can act on
/// directly. Keys identify a row across edits; a position locates one at an
/// instant, which is all a click needs to say.
#[component]
pub fn RemoveRowButton(path: String, index: usize, on_edit: Callback<Edit>) -> Element {
    // 1-based for humans: this is the only place a row's position is spoken
    // aloud, and it is never used as a path segment.
    let ordinal = index + 1;
    rsx! {
        button {
            r#type: "button",
            class: "remove-row",
            aria_label: "Remove row {ordinal}",
            onclick: move |_| {
                on_edit.call(Edit::RemoveRow { path: path.clone(), index });
            },
            "Remove"
        }
    }
}
