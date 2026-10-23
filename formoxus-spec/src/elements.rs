use crate::{InputType, attrs::str_eq};

/// An element a widget part can be: the wrappers (`div`, `fieldset`,
/// `label`, `legend`) and the bare form controls. What attribute validity is
/// keyed on, so an `<input>` carries its `type`, which HTML's rules depend on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HtmlElement {
    Div,
    Fieldset,
    Label,
    Legend,
    Textarea,
    Select,
    Input(InputType),
}

impl HtmlElement {
    pub const fn tag(&self) -> &'static str {
        match self {
            HtmlElement::Div => "div",
            HtmlElement::Fieldset => "fieldset",
            HtmlElement::Label => "label",
            HtmlElement::Legend => "legend",
            HtmlElement::Input(_) => "input",
            HtmlElement::Textarea => "textarea",
            HtmlElement::Select => "select",
        }
    }
}

/// An element a widget renders a class on: one of its PARTS
/// (`Widget::PARTS`), and with the widget's name the key of a slot in
/// `Classes`.
///
/// Not [`HtmlElement`], which carries the input type because attribute
/// validity depends on it. Classes do not: within one widget the `input`
/// part is just its `<input>`, whatever its `type`. A closed set, because a
/// class can only go on an element a widget actually renders, so `form!`
/// can refuse a misspelled part while parsing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tag {
    Div,
    Fieldset,
    Label,
    Legend,
    Input,
    Textarea,
    Select,
}

impl Tag {
    /// Every tag, in declaration order: what `form!` suggests from when a
    /// `classes_plus:` part is misspelled.
    pub const ALL: &'static [Tag] = &[
        Tag::Div,
        Tag::Fieldset,
        Tag::Label,
        Tag::Legend,
        Tag::Input,
        Tag::Textarea,
        Tag::Select,
    ];

    /// The element's tag name, as HTML writes it and `classes_plus:` spells
    /// a part: `"div"`, `"select"`.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Tag::Div => "div",
            Tag::Fieldset => "fieldset",
            Tag::Label => "label",
            Tag::Legend => "legend",
            Tag::Input => "input",
            Tag::Textarea => "textarea",
            Tag::Select => "select",
        }
    }

    /// The tag named `tag` ([`Self::as_str`]'s spelling), or `None` if no
    /// widget part is called that. `const`, so `form!`'s parse and a `const`
    /// both can use it; which is also why it is not the `FromStr` trait.
    pub const fn from_str(tag: &str) -> Option<Self> {
        if str_eq(tag, "div") {
            Some(Tag::Div)
        } else if str_eq(tag, "fieldset") {
            Some(Tag::Fieldset)
        } else if str_eq(tag, "label") {
            Some(Tag::Label)
        } else if str_eq(tag, "legend") {
            Some(Tag::Legend)
        } else if str_eq(tag, "input") {
            Some(Tag::Input)
        } else if str_eq(tag, "textarea") {
            Some(Tag::Textarea)
        } else if str_eq(tag, "select") {
            Some(Tag::Select)
        } else {
            None
        }
    }

    /// Whether `self` and `other` are the same tag: `==` for `const` code,
    /// where a derived `PartialEq` cannot be called. What
    /// `has_part::<W>` checks `W::PARTS` with.
    pub const fn same(self, other: Tag) -> bool {
        self as u8 == other as u8
    }
}
