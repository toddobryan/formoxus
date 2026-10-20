use std::rc::Rc;

use dioxus::core::Attribute;

use crate::{
    Classes, ValidationMessage, field_attrs::FieldAttrs, nodes::ValuesStore, widgets::Choice,
};

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
    /// The field's choices, read from it on every render (`Field::choices`),
    /// so a field whose choices change is shown with the new ones. `None` for
    /// a field without any; a chooser over a `bool` then offers true/false.
    pub choices: Option<Vec<Choice>>,
}
