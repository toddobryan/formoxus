use std::fmt::Debug;
use std::rc::Rc;

use crate::ValidationMessage;
use crate::form::FieldSpec;

#[derive(Clone, Debug)]
pub struct Field<T> {
    pub name: String,
    pub optional: bool,
    pub label: Option<String>,
    pub label_suffix: Option<String>,
    pub initial: Option<T>,
    pub widget: Rc<dyn Widget>,
    pub help_text: String,
    pub errors: Vec<ValidationMessage>,
    pub validators: Vec<fn(&T) -> Option<ValidationMessage>>,
    pub disabled: bool,
}

pub trait Widget : Debug {

}