use std::rc::Rc;

use dioxus::core::Attribute;

use crate::{Classes, ValidationMessage, field_attrs::FieldAttrs, nodes::ValuesStore};

#[derive(Debug)]
pub struct WidgetProps {
    pub id: String,
    pub path: String,
    pub label: Option<String>,
    pub errors: Vec<ValidationMessage>,
    pub attrs: FieldAttrs,
    pub extra_attrs: Vec<Attribute>,
    pub values: ValuesStore,
    pub classes: Rc<Classes>,
}
