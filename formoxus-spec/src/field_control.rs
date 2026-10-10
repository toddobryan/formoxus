use crate::InputType;

/// The HTML tag in a field (including the label, errors, etc.) that the
/// user actually attaches input to. (Ignore `FieldSet` for the moment--
/// `RadioGroup` is weird)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldControl {
    Input(InputType),
    Textarea,
    Select,
    Fieldset,
}
