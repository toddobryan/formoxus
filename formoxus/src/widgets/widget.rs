use std::fmt::Debug;
use std::rc::Rc;

use crate::ValidationMessage;

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub optional: bool,
    pub label: Option<String>,
    pub widget: Rc<dyn Widget>,
    pub help_text: String,
    pub errors: Vec<ValidationMessage>,

}

trait Widget : Debug {

}