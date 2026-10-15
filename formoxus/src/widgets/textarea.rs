//! The `<textarea>`.

use dioxus::prelude::*;
use formoxus_spec::{Attr, BuiltinWidget, FieldType};

use super::errors::FieldErrors;
use super::label::field_label;
use super::types::FieldProps;
use super::values::{get_current, write_value};
use crate::fields::FieldAttrs;
use crate::members::ValuesStore;

/// A multi-line text input bound to one path in the value map.
///
/// Otherwise identical to [`Input`] — same controlled-value binding, same
/// `aria-invalid` rule, same label layout. There is no hidden-input branch to
/// mirror: `Textarea` is only ever chosen as an override on a `Text` field, and
/// nothing here needs the extra `InputType` cases (`Password`'s masking,
/// `Hidden`'s bare markup) that make `Input` carry one.
#[component]
pub fn Textarea(
    field_type: FieldType,
    field_attrs: FieldAttrs,
    values: ValuesStore,
    props: FieldProps,
    #[props(extends = textarea)] attrs: Vec<Attribute>,
) -> Element {
    let my_widget = BuiltinWidget::Textarea;
    let is_invalid = props.is_invalid();

    let class = field_attrs.class(props.classes.part(my_widget, "textarea"), is_invalid);
    let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(field_type, attrs);

    let field_class = props
        .classes
        .part_class(BuiltinWidget::Textarea, "div", props.is_invalid());

    let current = get_current(&props.path, values);

    let label = field_label(
        &props.id,
        props.label,
        field_attrs.contains(Attr::Required),
        &props.classes,
        is_invalid,
        my_widget,
    );

    rsx! {
        div {
            class: field_class,
            {label}
            textarea {
                class: class,
                id: &props.id,
                name: "{props.path}",
                value: "{current}",
                oninput: move |e: FormEvent| {
                    let raw = e.value();
                    write_value(&props.path, values, raw);
                },
                // TODO: delete pattern, since not allowed in HTML
                ..attrs,
            }
            FieldErrors { field_id: &props.id, errors: props.errors, classes: props.classes.clone() }
        }
    }
}
