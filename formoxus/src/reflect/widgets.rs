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
/// `ScalarInput` dispatches on this, one widget per kind: `Text` and the two
/// numeric kinds to `TextInput`/`NumericInput`, `Boolean` to `BooleanInput` —
/// a checkbox for a plain `bool`, the tri-state `SelectInput` for an
/// `Option<bool>`, since a checkbox has two states and `Option<bool>` has three.
///
/// **There is deliberately no `Select` variant.** A select needs OPTIONS, and a
/// shape cannot carry them — a picker's choices are render-time data, usually
/// fetched. The only select a shape can imply is the tri-state for
/// `Option<bool>`, which `Boolean { optional: true }` already says. Data-driven
/// pickers therefore belong to the widget registry, chosen at the call site
/// where a provider can be supplied. (A `Select` variant existed briefly, with a
/// `todo!()` arm; it was removed once that argument was made, because a closed
/// enum is a commitment that gets harder to unwind the longer it stands.)
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
    Boolean {
        optional: bool,
    },
    Int {
        min: i128,
        max: i128,
    },
    Float,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldProps {
    pub path: String,
    pub label: Option<String>,
    // whether the field(s) below are considered required in the form,
    // modulo weird things like not being able to mark checkboxes required
    pub required: bool,
    pub errors: Vec<FieldError>,
}

fn get_current(path: &str, values: ValuesByPath) -> String {
    let slot = values.get_unchecked(path.to_string());
    slot.try_read().map(|v| v.clone()).unwrap_or_default()
}

fn write_value(path: &str, mut values: ValuesByPath, raw: String) {
    let populated = values.peek().contains_key(path);
    if populated {
        values.get_unchecked(path.to_string()).set(raw);
    } else {
        values.insert(path.to_string(), raw);
    }
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
    input_kind: InputKind,
    values: ValuesByPath,
    props: FieldProps,
) -> Element {
    match input_kind {
        InputKind::Text => rsx! { TextInput { values, props } },
        InputKind::Boolean { optional } => rsx! { BooleanInput { values, optional, props } },
        InputKind::Int { .. } => rsx! { NumericInput { input_kind, values, props }},
        InputKind::Float => rsx! { NumericInput { input_kind, values, props }},
    }
}

#[component]
pub fn TextInput(
    values: ValuesByPath,
    props: FieldProps,
) -> Element {
    let FieldProps { path, label: label_text, required, errors } = props;

    let current = get_current(&path, values);

    // Present ONLY when there is an error. `aria-invalid="false"` is NOT the
    // neutral value — it asserts "checked, and passed", which Pico duly paints
    // green with a tick, so an untouched form would claim to have validated
    // every field. Absent is the only neutral state. Dioxus omits an attribute
    // whose value is `None`, which is what makes absence expressible at all.
    //
    // Unlike the `small` in `FieldErrors`, this is not a styling choice with a
    // framework behind it: `aria-invalid` is the W3C ARIA attribute assistive
    // technology reads to announce a field as errored, so it belongs here
    // whatever CSS the consumer brings.
    let invalid = (!errors.is_empty()).then_some("true");

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
                aria_invalid: invalid,
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&path, values, raw);
                },
            }
            FieldErrors { errors }
        }
    }
}

#[component]
pub fn BooleanInput(
    mut values: ValuesByPath,
    optional: bool,
    props: FieldProps,
) -> Element {
    // An `Option<bool>` has three states and a checkbox has two, so it needs a
    // select. Delegating rather than inlining one keeps a single implementation
    // of the "no value" option and the required/optional asymmetry.
    if optional {
        return rsx! {
            SelectInput { values, choices: bool_choices(), props }
        };
    }

    let FieldProps { path, label, errors, .. } = props;

    // `required` is deliberately dropped rather than forwarded. HTML `required`
    // on a checkbox means "must be ticked", which is not what a required `bool`
    // field asks for — unticked is a complete answer. For the same reason there
    // is no ` *` marker: it would promise a rule nothing enforces.
    // See `TextInput`. Pico skips a checkbox for the invalid *icon* (there is
    // nowhere to put one), but the border and the adjacent `small` still key off
    // this, and it is what a screen reader announces either way.
    let invalid = (!errors.is_empty()).then_some("true");

    let input_element = rsx! {
        input {
            name: "{path}",
            r#type: "checkbox",
            checked: get_current(&path, values) == "true",
            aria_invalid: invalid,
            onchange: move |e: FormEvent| write_value(&path, values, e.value())
        }
        FieldErrors { errors: errors.clone() }
    };

    if let Some(label_text) = label.clone() {
        rsx! {
            label {
                "{label_text}"
                { input_element }
            }
        }
    } else {
        input_element
    }
}

#[component]
pub fn NumericInput(
    input_kind: InputKind,
    values: ValuesByPath,
    props: FieldProps,
) -> Element {
    let FieldProps { path, label: label_text, required, errors } = props;

    let current = get_current(&path, values);

    // See `TextInput` for why this is `Option` rather than a plain bool.
    let invalid = (!errors.is_empty()).then_some("true");

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
                aria_invalid: invalid,
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&path, values, raw);
                },
            }
            FieldErrors { errors }
        }
    }
}

/// One option in a [`SelectInput`].
#[derive(Clone, Debug, PartialEq)]
pub struct SelectChoice {
    /// The raw string written into the value map, so it has to be exactly what
    /// `parse_scalar` expects for this field's type — `"true"`, not `"True"`.
    /// That the two can differ at all is why this isn't just a `Vec<String>`.
    pub value: String,
    /// What the user reads.
    pub display: String,
}

impl SelectChoice {
    pub fn new(value: impl Into<String>, display: impl Into<String>) -> Self {
        Self { value: value.into(), display: display.into() }
    }
}

/// The three states of an `Option<bool>`, minus the absent one — that comes
/// from `SelectInput`'s own "no value" option, so it is spelled in exactly one
/// place rather than once per caller.
fn bool_choices() -> Vec<SelectChoice> {
    vec![SelectChoice::new("true", "True"), SelectChoice::new("false", "False")]
}

/// A `<select>` over a fixed set of choices, bound to one path in the value map.
///
/// Reads its current value from `values` like every other leaf widget rather
/// than taking it as a prop. That is not just consistency: computing `selected`
/// for a prop would mean reading the store in `FormField::render`, a plain
/// function with no scope of its own, which subscribes *the caller* — so one
/// change here would re-render the whole form. [`VariantSelect`] takes its
/// selection as a prop precisely because a variant choice is NOT a leaf and has
/// no path to read.
///
/// Unlike the derive path's `SelectWidget`, an option's value is the raw string
/// itself rather than an index into `choices`. That path stores an index because
/// its `T` might not survive a round trip through a string; here `T -> String ->
/// T` is a guaranteed identity (see the `roundtrip` tests), so the indirection —
/// and its silent `unwrap_or_default()` when an index doesn't match — is pure
/// loss.
#[component]
pub fn SelectInput(
    values: ValuesByPath,
    choices: Vec<SelectChoice>,
    props: FieldProps,
) -> Element {
    let FieldProps { path, label: label_text, required, errors } = props;

    let current = get_current(&path, values);

    // See `TextInput` for why this is `Option` rather than a plain bool.
    let invalid = (!errors.is_empty()).then_some("true");

    rsx! {
        label { class: "form-field",
            if let Some(text) = label_text {
                span { class: "field-label", "{text}" }
            }
            if required {
                span { class: "required", " *" }
            }
            select {
                aria_invalid: invalid,
                // Unlike `VariantSelect`, this one IS a leaf, so it must carry a
                // `name` or `apply_form_values` would never see it.
                name: "{path}",
                required,
                // No branch on emptiness: the "no value" option's value is `""`,
                // and `""` IS absence at both boundaries, so the same write does
                // for every option.
                onchange: move |e: FormEvent| write_value(&path, values, e.value()),
                // Required and unanswered: an unselectable placeholder, so the
                // browser's own validation blocks submit and the user can't
                // choose their way back to "unanswered". Optional: a real
                // selectable entry, because absent is a legitimate answer.
                if required && current.is_empty() {
                    option { value: "", selected: true, disabled: true, hidden: true, "Choose..." }
                } else if !required {
                    option { value: "", selected: current.is_empty(), "{ABSENT_DISPLAY}" }
                }
                for choice in choices {
                    option {
                        value: "{choice.value}",
                        selected: choice.value == current,
                        "{choice.display}"
                    }
                }
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
///
/// [`SelectInput`] shows the same text, and it *does* carry a `name` — but it is
/// safe there for the same reason by a different route: the option's value is
/// `""`, never this text, and `""` is absence at both boundaries.
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
