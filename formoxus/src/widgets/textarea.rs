//! The `<textarea>`.

use dioxus::prelude::*;
use formoxus_spec::{Attr, FieldType};

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
    let class = field_attrs.class("fx-control fx-textarea");
    let attrs: Vec<Attribute> = field_attrs.merge_with_attrs(field_type, attrs);

    let field_class = props.field_class();

    let current = get_current(&props.path, values);

    rsx! {
        div {
            class: field_class,
            {field_label(&props.id, props.label, field_attrs.contains(Attr::Required), &props.classes)}
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
