//! The widget boundary for the reflection path.
//!
//! One component per leaf, and that is the load-bearing part. A component is
//! the unit of reactivity in Dioxus: a store read inside one subscribes *that*
//! scope. `FormMember::render` is a plain function with no scope of its own, so
//! reading a value there would subscribe whoever called it — and a single
//! keystroke would re-render the entire form. Spawning a component per leaf is
//! what keeps a write to one path local to one input.
//!
//! This mirrors the derive path, where `TextInput::render` doesn't inline its
//! markup either — it spawns `InputWidget`, for exactly this reason.

use dioxus::prelude::*;

use crate::error::FieldError;
use crate::label_case::{LabelCase, ToCase};
use crate::reflect::{Edit, ValuesByPath};
use crate::widgets::FieldErrors;

/// Which control a scalar leaf renders as.
///
/// Assigned in `scalar_member`'s `dispatch!` macro, where the concrete type is
/// still known — `FormField<T>::render` can't reach a `DefaultWidget`-style
/// trait without bounding every `Facet` type in the crate.
///
/// **`ScalarInput` does not branch on this yet**: every kind still renders as
/// `type="text"`, so a `bool` shows the literal `true`. That branch is the next
/// piece of work.
///
/// `Int`'s bounds are for the error message ("must be between 0 and 255"), not
/// for HTML `min`/`max`, which do nothing on a text input — `parse_scalar`
/// already rejects out-of-range values. They are also where a user-specified
/// `#[form(min = …)]` would land. `Int`/`Float` stay `type="text"` deliberately:
/// `type="number"` hands back `""` for anything the browser dislikes, so a
/// half-typed value disappears.
#[derive(Clone, Debug, PartialEq)]
pub enum InputKind {
    Text,
    Checkbox,
    Select,
    Int {
        min: i128,
        max: i128,
    },
    Float,
}

/// A single-line text input bound to one path in the value map.
///
/// `values` + `path` rather than a pre-lensed child store, because a path that
/// the schema has but the map doesn't is a normal state, not an error: a
/// variant chosen after mount reveals leaves that were never populated. A missing
/// key reads as `""`, which is the same "empty IS absence" rule `apply_leaves`
/// already follows when a path is absent from submitted values.
#[component]
pub fn ScalarInput(
    path: String,
    label: Option<String>,
    input_kind: InputKind,
    errors: Vec<FieldError>,
    /// A presentation hint only. It is deliberately false for every leaf under
    /// an `Option`, including the leaves of an optional *struct* — HTML5
    /// `required` is per-input and can't express "all of these or none", so
    /// marking them would block a deliberately blank one. `validate()` stays
    /// the authority on the all-or-nothing rule. See [`crate::reflect::RenderCtx`].
    required: bool,
    values: ValuesByPath,
) -> Element {
    // `get_unchecked`, not `get`: `get` calls `contains_key`, which tracks the
    // map *shallowly* — this input would then re-render whenever any key is
    // added anywhere. `get_unchecked` builds the child selector without
    // reading, so the only subscription is the `try_read` below, on this key
    // alone. It never panics here because we never `read()` it directly.
    let slot = values.get_unchecked(path.clone());
    let current = slot.try_read().map(|v| v.clone()).unwrap_or_default();

    let label_text = label;
    let write_path = path.clone();
    let mut values = values;

    rsx! {
        label { class: "form-field",
            if let Some(text) = label_text {
                span { class: "field-label", "{text}" }
            }
            if required {
                span { class: "required", " *" }
            }
            input {
                r#type: "text",
                name: "{path}",
                value: "{current}",
                required,
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    // Write *through the child store* when the key exists: that
                    // marks only this key dirty. `insert` would call
                    // `mark_dirty_shallow` and re-render every other input, so
                    // it's the fallback for a never-populated path only — one
                    // coarse re-render on the first keystroke into a freshly
                    // revealed field, fine-grained from then on.
                    // `peek`, not `contains_key`: the store's own
                    // `contains_key` tracks shallowly, and an event handler has
                    // no business adding subscriptions. Bound to a `let` so the
                    // read guard is definitely released before the write below.
                    let populated = values.peek().contains_key(&write_path);
                    if populated {
                        values.get_unchecked(write_path.clone()).set(raw);
                    } else {
                        values.insert(write_path.clone(), raw);
                    }
                },
            }
            FieldErrors { errors }
        }
    }
}

/// The "leave this out" entry in an optional enum's picker.
///
/// Display only, and it stays that way for a structural reason rather than a
/// cosmetic one: the `<select>` carries no `name`, so nothing it holds is ever
/// collected by `FormData::values()` and this text cannot come back as a value.
/// That is what keeps it from reintroducing the sentinel problem
/// [`VariantChoice`](crate::reflect::VariantChoice) exists to avoid — a model
/// with a genuine `None` variant would otherwise be indistinguishable from an
/// unanswered optional field. What the select actually emits is `""`, which
/// `VariantSelect` turns into `ChooseVariant { variant: None }`.
pub(crate) const ABSENT_DISPLAY: &str = "--none--";

#[component]
pub fn VariantSelect(
    path: String,
    label: Option<String>,
    required: bool,
    errors: Vec<FieldError>,
    variants: Vec<&'static str>,
    selected: Option<String>,
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
                        "{v.to_case(LabelCase::Title)}"
                    }
                }
            }
            FieldErrors { errors }
        }
    }
}

/// The control that appends a row to a list.
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
                // control between every pair of rows — a UI question that hasn't
                // been answered yet, not a limitation of the edit.
                on_edit.call(Edit::AddRow { path: path.clone(), before: None });
            },
            "Add"
        }
    }
}

/// The control that drops one row from a list.
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
