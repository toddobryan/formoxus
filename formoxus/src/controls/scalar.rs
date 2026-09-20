//! Dispatch: which control renders a given (value kind, control) pair.

use dioxus::prelude::*;

use super::boolean::BooleanInput;
use super::html::{HtmlInput, TextareaInput};
use super::select::{SelectInput, bool_choices};
use super::types::{ControlProps, ControlType, FieldProps};
use crate::ValuesByPath;
use crate::fields::ValueKind;

/// Picks the input for one leaf and hands it the pair every input takes.
///
/// The match below is the only place that decides which `(value kind, control)`
/// combinations actually work — `form!` accepts every control name against
/// every field, so a pair with no arm here reaches the fallthrough and panics.
/// `cargo run -p formoxus-examples --bin control_matrix` prints which ones do.
///
/// `values` + `path` rather than a pre-lensed child store, because a path that
/// the schema has but the map doesn't is a normal state, not an error: a
/// variant chosen after mount reveals leaves that were never populated. A missing
/// key reads as `""`, which is the same "empty IS absence" rule `apply_leaves`
/// already follows when a path is absent from submitted values.
#[component]
pub fn ScalarInput(
    value_kind: ValueKind,
    control: ControlType,
    values: ValuesByPath,
    props: FieldProps,
) -> Element {
    match (&value_kind, &control) {
        // One arm for every `<input type=…>`, over any value kind that is a
        // single scalar. The value crosses as a string either way — `ValueKind`
        // is what parses it back, and it is NOT consulted here on purpose, so
        // that a presentational override cannot change how a value is read.
        //
        // `Int`/`Float` land here too: their default control is `Text`, because
        // `type="number"` would eat a half-typed value — but `number` is a
        // perfectly good override, and so is `text` on a numeric.
        (
            ValueKind::Text { .. } | ValueKind::Int { .. } | ValueKind::Float,
            ControlType::Input(input_type),
        ) => {
            rsx! { HtmlInput { input_type: input_type.clone(), values, props } }
        }
        (ValueKind::Text { .. }, ControlType::Textarea) => {
            rsx! { TextareaInput { values, props } }
        }
        (ValueKind::Bool, ControlType::Checkbox) => {
            rsx! { BooleanInput { values, props } }
        }
        (ValueKind::Bool, ControlType::Select) => {
            rsx! { SelectInput { values, choices: bool_choices(), props } }
        }
        // Matches ANY value kind, deliberately. A custom control exists precisely
        // because the built-in controls can't serve its type, so gating it on
        // the kinds we happen to enumerate would defeat it — `Markdown` and
        // `Ref<Source>` are `Text` to the parser and nothing to a `<select>`.
        // The author named this input for this field; that IS the evidence.
        (_, ControlType::Custom { render, .. }) => render(ControlProps { values, props }),
        _ => panic!(
            "{control:?} cannot render a {value_kind:?} (field {})",
            props.path
        ),
    }
}
