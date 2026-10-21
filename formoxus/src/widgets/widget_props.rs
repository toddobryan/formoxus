use std::rc::Rc;

use dioxus::core::Attribute;

use crate::{
    Classes, ValidationMessage, field_attrs::FieldAttrs, nodes::ValuesStore, widgets::Choice,
};

#[derive(Clone, Debug, PartialEq)]
pub struct WidgetProps {
    /// The field's name in the PAGE: the control's `id`, what its `<label for>`
    /// and `aria-describedby` point at. `{form id}-{path}`, built by
    /// `NodeContext::id`, so it differs between two copies of one form on a page.
    pub id: String,
    /// The field's name in the DATA: its `name` attribute, its key in the value
    /// store, and what the server matches a submitted value to. The same for
    /// every copy of the form, and never containing the form id, which the
    /// server does not have.
    pub path: String,
    pub label: Option<String>,
    pub errors: Vec<ValidationMessage>,
    pub attrs: FieldAttrs,
    pub extra_attrs: Vec<Attribute>,
    pub values: ValuesStore,
    /// The form's resolved classes, for the widget to look up its parts' slots
    /// (`classes.part_class(widget, "label", invalid)`). A custom widget gets
    /// them too, so it can borrow a built-in slot and follow the app's preset.
    /// An `Rc` from `NodeContext`; see the rule on `NodeContext::classes`.
    pub classes: Rc<Classes>,
    /// The field's choices, read from it on every render (`Field::choices`),
    /// so a field whose choices change is shown with the new ones. `None` for
    /// a field without any; a chooser over a `bool` then offers true/false.
    pub choices: Option<Vec<Choice>>,
}

impl WidgetProps {
    pub fn is_invalid(&self) -> bool {
        !self.errors.is_empty()
    }
}
