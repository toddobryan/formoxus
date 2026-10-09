//! Dispatch: which widget renders a given (value kind, widget) pair.

use dioxus::prelude::*;

use formoxus_attrs::{Attr, FieldType};

use super::checkbox::Checkbox;
use super::input::Input;
use super::radio_group::RadioGroup;
use super::select::Select;
use super::textarea::Textarea;
use super::types::{Choice, FieldProps, WidgetProps, WidgetType, bool_choices};
use crate::fields::FieldAttrs;
use crate::members::ValuesStore;

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
/// key reads as `""`, which is the same "empty IS absence" rule `distribute_values`
/// already follows when a path is absent from submitted values.
#[component]
pub fn ScalarWidget(
    field_type: FieldType,
    field_attrs: FieldAttrs,
    widget: WidgetType,
    choices: Option<Vec<Choice>>,
    values: ValuesStore,
    props: FieldProps,
    #[props(extends = input)] attrs: Vec<Attribute>,
) -> Element {
    match (&field_type, &widget) {
        // One arm for every `<input type=…>`, over any value kind that is a
        // single scalar. The value crosses as a string either way — the field's
        // type is what parses it back, and it is NOT consulted here on purpose, so
        // that a presentational override cannot change how a value is read.
        //
        // `Int`/`Float` land here too: their default widget is `Text`, because
        // `type="number"` would eat a half-typed value — but `number` is a
        // perfectly good override, and so is `text` on a numeric.
        (FieldType::Text | FieldType::Int | FieldType::Float, WidgetType::Input(input_type)) => {
            rsx! { Input { input_type: *input_type, field_type, field_attrs, values, props, attrs } }
        }
        (FieldType::Text, WidgetType::Textarea) => {
            rsx! { Textarea { field_type, field_attrs, values, props, attrs } }
        }
        (FieldType::Bool, WidgetType::Checkbox) => {
            rsx! { Checkbox { field_type, field_attrs, values, props, attrs } }
        }
        // Any single scalar can be chosen from a list, because a choice's value
        // is just the raw string this field already parses. The list is the only
        // thing that makes the pair renderable, hence the panic rather than an
        // empty `<select>` — a chooser with nothing to choose is a declaration
        // the author did not finish.
        (FieldType::Text | FieldType::Int | FieldType::Float, WidgetType::Select) => {
            let Some(choices) = choices else {
                panic!(
                    "in field {}, `select` needs choices — add `widget: select {{ choices: … }}`",
                    props.path
                )
            };
            rsx! { Select { field_type, field_attrs, values, choices, props, attrs } }
        }
        // Any single scalar can be chosen from a list of radio buttons, because a
        // choice's value is just the raw string this field already parses. The list
        // is the only thing that makes the pair renderable, hence the panic rather
        // than an empty radio group — a group with nothing to choose is a declaration
        // the author did not finish.
        (FieldType::Text | FieldType::Int | FieldType::Float, WidgetType::RadioGroup) => {
            let Some(choices) = choices else {
                panic!(
                    "in field {}, `radio_group` needs choices — add `widget: radio_group {{ choices: … }}`",
                    props.path
                )
            };
            reject_if_not_required(&field_attrs, &props.path);
            rsx! { RadioGroup { field_type, field_attrs, values, choices, props, attrs } }
        }

        // A bool's choices are derivable, so it is the one kind that renders
        // without a list — but an explicit one still wins, for a form that would
        // rather say "Yes"/"No".
        (FieldType::Bool, WidgetType::Select) => {
            rsx! { Select { field_type, field_attrs, values, choices: choices.unwrap_or_else(bool_choices), props, attrs } }
        }
        (FieldType::Bool, WidgetType::RadioGroup) => {
            reject_if_not_required(&field_attrs, &props.path);
            rsx! { RadioGroup { field_type, field_attrs, values, choices: choices.unwrap_or_else(bool_choices), props, attrs }}
        }
        // Matches ANY value kind, deliberately. A custom widget exists precisely
        // because the built-in widgets can't serve its type, so gating it on
        // the kinds we happen to enumerate would defeat it — `Markdown` and
        // `Ref<Source>` are `Text` to the parser and nothing to a `<select>`.
        // The author named this input for this field; that IS the evidence.
        (_, WidgetType::Custom { render, .. }) => {
            let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(field_type, attrs);

            render(WidgetProps {
                values,
                props,
                attrs,
            })
        }
        _ => panic!(
            "{widget:?} cannot render a {field_type:?} (field {})",
            props.path
        ),
    }
}

fn reject_if_not_required(attrs: &FieldAttrs, path: &str) {
    assert!(
        attrs.contains(Attr::Required),
        "do not use `radio_group` because the field {path} is not required - use `select` instead",
    );
}
