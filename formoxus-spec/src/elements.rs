use crate::InputType;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HtmlElement {
    Div,
    Fieldset,
    Label,
    Legend,
    Control(FieldControl),
}

impl HtmlElement {
    pub const fn tag(&self) -> &'static str {
        match self {
            HtmlElement::Div => "div",
            HtmlElement::Fieldset => "fieldset",
            HtmlElement::Label => "label",
            HtmlElement::Legend => "legend",
            HtmlElement::Control(field_control) => match field_control {
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

impl From<FieldControl> for HtmlElement {
    fn from(value: FieldControl) -> Self {
        Self::Control(value)
    }
}
