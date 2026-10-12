use crate::InputType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Element {
    Div,
    Fieldset,
    Label,
    Legend,
    Control(FieldControl),
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
