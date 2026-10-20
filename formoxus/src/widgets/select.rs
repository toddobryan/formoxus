//! `<select>` over a fixed set of choices.

use dioxus::prelude::*;
use formoxus_spec::{Attr, BuiltinWidget};

use super::errors::FieldErrors;
use super::label::field_label;
use super::types::{Choice, FieldProps};
use super::values::{get_current, write_value};
use crate::field_attrs::FieldAttrs;
use crate::nodes::ValuesStore;

/// A `<select>` over a fixed set of choices, bound to one path in the value map.
///
/// Reads its current value from `values` like every other leaf input rather
/// than taking it as a prop. That is not just consistency: computing `selected`
/// for a prop would mean reading the store in `Field::render`, a plain
/// function with no scope of its own, which subscribes *the caller* — so one
/// change here would re-render the whole form. [`VariantSelect`] takes its
/// selection as a prop precisely because a variant choice is NOT a leaf and has
/// no path to read.
///
/// An option's value is the raw string itself, NOT an index into `choices`.
/// Indexing is the usual dodge for a `T` that might not survive a round trip
/// through a string, but here `T -> String -> T` is a guaranteed identity for
/// every builtin scalar (see the `roundtrip` tests), so the indirection — and
/// the silent `unwrap_or_default()` it needs when an index doesn't match — buys
/// nothing and loses the value.
#[component]
pub fn Select(
    field_attrs: FieldAttrs,
    values: ValuesStore,
    choices: Vec<Choice>,
    props: FieldProps,
    #[props(extends = select)] attrs: Vec<Attribute>,
) -> Element {
    let my_widget = BuiltinWidget::Select;
    let is_invalid = props.is_invalid();

    let class = field_attrs.class(props.classes.part(my_widget, "select"), is_invalid);
    let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(attrs);

    let field_class = props.classes.part_class(my_widget, "div", is_invalid);

    let current = get_current(&props.path, values);
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
            {label}
            select {
                class: class,
                id: props.id,
                // Unlike `VariantSelect`, this one IS a leaf, so it must carry a
                // `name` or `distribute_form_values` would never see it.
                name: "{props.path}",
                // No branch on emptiness: the "no value" option's value is `""`,
                // and `""` IS absence at both boundaries, so the same write does
                // for every option.
                onchange: move |e: FormEvent| write_value(&props.path, values, e.value()),
                // Last among attributes, before the options: rsx! reads
                // anything after a spread as children, and the options ARE
                // children, so this sits between the two.
                ..attrs,
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
            FieldErrors { field_id: &props.id, errors: props.errors, classes: props.classes.clone() }
        }
    }
}

/// The "leave this out" entry in an optional enum's picker.
///
/// Display only, and it stays that way for a structural reason rather than a
/// cosmetic one: the `<select>` carries no `name`, so nothing it holds is ever
/// collected by `FormData::values()` and this text cannot come back as a value.
/// That is what keeps it from reintroducing the sentinel problem
/// [`VariantChoice`](crate::nodes::VariantChoice) exists to avoid — a model
/// with a genuine `None` variant would otherwise be indistinguishable from an
/// unanswered optional field. What the select actually emits is `""`, which
/// `VariantSelect` turns into `ChooseVariant { variant: None }`.
///
/// [`Select`] shows the same text, and it *does* carry a `name` — but it is
/// safe there for the same reason by a different route: the option's value is
/// `""`, never this text, and `""` is absence at both boundaries.
/// **Not localizable yet, and it should be.** This is English punctuation baked
/// into a library: a form rendered in French or Japanese still reads
/// `--none--`. The eventual shape is configuration — most likely alongside
/// whatever mechanism makes error rendering overridable, since both are "text
/// formoxus emits on the consumer's behalf" — rather than a second const.
/// Deliberately deferred; nothing depends on it being fixed soon.
pub const ABSENT_DISPLAY: &str = "--none--";
