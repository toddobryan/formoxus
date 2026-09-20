//! Dispatch: which widget renders a given (value kind, widget) pair.

use dioxus::prelude::*;

use super::checkbox::Checkbox;
use super::input::Input;
use super::select::{Select, bool_choices};
use super::textarea::Textarea;
use super::types::{FieldProps, WidgetProps, WidgetType};
use crate::ValuesByPath;
use crate::fields::ValueKind;

/// Picks the widget for one leaf and hands it the pair every widget takes.
///
/// The match below is the only place that decides which `(value kind, widget)`
/// combinations actually work — `form!` accepts every widget name against
/// every field, so a pair with no arm here reaches the fallthrough and panics.
/// `cargo run -p formoxus-examples --bin widget_matrix` prints which ones do.
///
/// `values` + `path` rather than a pre-lensed child store, because a path that
/// the schema has but the map doesn't is a normal state, not an error: a
/// variant chosen after mount reveals leaves that were never populated. A missing
/// key reads as `""`, which is the same "empty IS absence" rule `apply_leaves`
/// already follows when a path is absent from submitted values.
#[component]
pub fn ScalarWidget(
    value_kind: ValueKind,
    widget: WidgetType,
    values: ValuesByPath,
    props: FieldProps,
) -> Element {
    match (&value_kind, &widget) {
        // One arm for every `<input type=…>`, over any value kind that is a
        // single scalar. The value crosses as a string either way — `ValueKind`
        // is what parses it back, and it is NOT consulted here on purpose, so
        // that a presentational override cannot change how a value is read.
        //
        // `Int`/`Float` land here too: their default widget is `Text`, because
        // `type="number"` would eat a half-typed value — but `number` is a
        // perfectly good override, and so is `text` on a numeric.
        (
            ValueKind::Text { .. } | ValueKind::Int { .. } | ValueKind::Float,
            WidgetType::Input(input_type),
        ) => {
            rsx! { Input { input_type: input_type.clone(), values, props } }
        }
        (ValueKind::Text { .. }, WidgetType::Textarea) => {
            rsx! { Textarea { values, props } }
        }
        (ValueKind::Bool, WidgetType::Checkbox) => {
            rsx! { Checkbox { values, props } }
        }
        (ValueKind::Bool, WidgetType::Select) => {
            rsx! { Select { values, choices: bool_choices(), props } }
        }
        // Matches ANY value kind, deliberately. A custom widget exists precisely
        // because the built-in widgets can't serve its type, so gating it on
        // the kinds we happen to enumerate would defeat it — `Markdown` and
        // `Ref<Source>` are `Text` to the parser and nothing to a `<select>`.
        // The author named this input for this field; that IS the evidence.
        (_, WidgetType::Custom { render, .. }) => render(WidgetProps { values, props }),
        _ => panic!(
            "{widget:?} cannot render a {value_kind:?} (field {})",
            props.path
        ),
    }
}
