use crate::InputType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Element {
    Div,
    Fieldset,
    Label,
    Legend,
    Control(FieldControl),
}

impl Element {
    pub const fn tag(&self) -> &'static str {
        match self {
            Element::Div => "div",
            Element::Fieldset => "fieldset",
            Element::Label => "label",
            Element::Legend => "legend",
            Element::Control(field_control) => match field_control {
                FieldControl::Input(_) => "input",
                FieldControl::Textarea => "textarea",
                FieldControl::Select => "select",
            },
        }
    }
}

/// The HTML tag in a field (including the label, errors, etc.) that the
/// user actually attaches input to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldControl {
    Input(InputType),
    Textarea,
    Select,
}

impl From<FieldControl> for Element {
    fn from(value: FieldControl) -> Self {
        Self::Control(value)
    }
}
