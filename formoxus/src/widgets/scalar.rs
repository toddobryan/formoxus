//! Dispatch: which widget renders a given (value kind, widget) pair.

use dioxus::prelude::*;

use formoxus_spec::{Attr, BuiltinWidget, FieldType};

use super::input::Input;
use super::radio_group::RadioGroup;
use super::select::Select;
use super::textarea::Textarea;
use super::types::{Choice, FieldProps, WidgetProps, WidgetType, bool_choices};
use crate::fields::FieldAttrs;
use crate::nodes::ValuesStore;

/// Picks the widget for one leaf and hands it the pair every widget takes.
///
/// Which `(field type, widget)` pairs render is [`BuiltinWidget::renders`]'s
/// rule, not this function's. `form!` checks it at compile time, so the
/// asserts here fire only for a `FormSpec` built without `form!` (through
/// `with_custom_widget`). Past them, the `match` only picks the component, and
/// is exhaustive over [`BuiltinWidget`]. `cargo run -p formoxus-examples --bin
/// widget_matrix` prints which pairs render.
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
    let builtin = match widget {
        // Renders ANY value kind, deliberately, so `renders` is not asked. A
        // custom widget exists precisely because the built-in widgets can't
        // serve its type, so gating it on the kinds we happen to enumerate
        // would defeat it — `Markdown` and `Ref<Source>` are `Text` to the
        // parser and nothing to a `<select>`. The author named this input for
        // this field; that IS the evidence.
        WidgetType::Custom { render, .. } => {
            let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(field_type, attrs);
            return render(WidgetProps {
                values,
                props,
                attrs,
            });
        }
        WidgetType::Builtin(builtin) => builtin,
    };

    assert!(
        builtin.is_rendered(),
        "`{}` is not supported yet (field {})",
        builtin.form_name(),
        props.path
    );
    // A chooser over anything but a bool needs a list: nothing to choose is a
    // declaration the author did not finish, so this panics rather than
    // rendering an empty `<select>` or radio group.
    assert!(
        builtin.renders(field_type, choices.is_some()),
        "`{}` cannot render the {field_type:?} field {}{}",
        builtin.form_name(),
        props.path,
        if builtin.takes_choices() && choices.is_none() {
            " without `choices`"
        } else {
            ""
        }
    );

    match builtin {
        // Every `<input type=…>`, checkbox included; `Input` routes on the
        // type. The value crosses as a string either way — the field's type is
        // what parses it back, and it is NOT consulted here on purpose, so that
        // a presentational override cannot change how a value is read.
        //
        // `Int`/`Float` land here too: their default widget is `Text`, because
        // `type="number"` would eat a half-typed value — but `number` is a
        // perfectly good override, and so is `text` on a numeric.
        BuiltinWidget::Input(input_type) => {
            rsx! { Input { input_type, field_type, field_attrs, values, props, attrs } }
        }
        BuiltinWidget::Textarea => {
            rsx! { Textarea { field_type, field_attrs, values, props, attrs } }
        }
        // Any single scalar can be chosen from a list, because a choice's value
        // is just the raw string this field already parses. A bool's choices
        // are derivable, so it is the one kind that gets here without a list
        // (the assert above saw to that) — but an explicit one still wins, for
        // a form that would rather say "Yes"/"No".
        BuiltinWidget::Select => {
            let choices = choices.unwrap_or_else(bool_choices);
            rsx! { Select { field_type, field_attrs, values, choices, props, attrs } }
        }
        BuiltinWidget::RadioGroup => {
            reject_if_not_required(&field_attrs, &props.path);
            let choices = choices.unwrap_or_else(bool_choices);
            rsx! { RadioGroup { field_type, field_attrs, values, choices, props, attrs } }
        }
        BuiltinWidget::SelectMultiple | BuiltinWidget::CheckboxMultiple | BuiltinWidget::File => {
            unreachable!("`is_rendered` is false for these, so the first assert stopped them")
        }
    }
}

fn reject_if_not_required(attrs: &FieldAttrs, path: &str) {
    assert!(
        attrs.contains(Attr::Required),
        "do not use `radio_group` because the field {path} is not required - use `select` instead",
    );
}
