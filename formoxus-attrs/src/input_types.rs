//! The `<input type=…>` values formoxus can render.

/// The `type` of an `<input>` that formoxus can render.
///
/// Here rather than in `formoxus`, because which attributes are valid on an
/// `<input>` depends on its type (`placeholder` on `text` but not `date`, `min`
/// on `date` but not `text`). Both the attribute table and `form!` need to name
/// these, and `formoxus-macros` cannot reach `formoxus`. `formoxus` re-exports
/// it as `formoxus::widgets::InputType`.
///
/// `Copy`, `Eq` and `Hash` so that the attribute table can key on it and
/// compare it in `const fn`s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputType {
    Text,
    Password,
    Hidden,
    Number,
    Email,
    Telephone,
    Url,
    Search,
    Color,
    Date,
    Time,
    DatetimeLocal,
    Month,
    Week,
}

impl InputType {
    /// The `type=` attribute this renders as.
    ///
    /// HTML's spelling, which is not always the variant's: `tel`, and
    /// `datetime-local` with the hyphen an ident could not carry. `form!`'s
    /// vocabulary spells these `tel` and `datetime_local`, so `InputType` is the
    /// pivot with HTML's names on both sides of it.
    pub const fn html_type(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Password => "password",
            Self::Hidden => "hidden",
            // Selectable, but NOT the default for a numeric field — see
            // `FormField::default_widget`. `type="number"` hands back `""` for
            // anything the browser dislikes, so a half-typed value vanishes
            // mid-keystroke. A numeric renders as text and `ValueKind` parses it.
            // Anyone who wants the spinner and the mobile keypad can ask.
            Self::Number => "number",
            Self::Email => "email",
            Self::Telephone => "tel",
            Self::Url => "url",
            Self::Search => "search",
            Self::Color => "color",
            Self::Date => "date",
            Self::Time => "time",
            Self::DatetimeLocal => "datetime-local",
            Self::Month => "month",
            Self::Week => "week",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InputType;
    use googletest::prelude::*;

    const ALL: [InputType; 14] = [
        InputType::Text,
        InputType::Password,
        InputType::Hidden,
        InputType::Number,
        InputType::Email,
        InputType::Telephone,
        InputType::Url,
        InputType::Search,
        InputType::Color,
        InputType::Date,
        InputType::Time,
        InputType::DatetimeLocal,
        InputType::Month,
        InputType::Week,
    ];

    /// The `type` keywords the WHATWG spec defines for `<input>`, checked
    /// 2026-10-02. A typo in `html_type` would otherwise render an input the
    /// browser silently treats as `text`.
    const SPEC_KEYWORDS: [&str; 22] = [
        "hidden",
        "text",
        "search",
        "tel",
        "url",
        "email",
        "password",
        "date",
        "month",
        "week",
        "time",
        "datetime-local",
        "number",
        "range",
        "color",
        "checkbox",
        "radio",
        "file",
        "submit",
        "image",
        "reset",
        "button",
    ];

    #[gtest]
    fn every_type_renders_as_a_keyword_the_spec_defines() {
        for t in ALL {
            expect_that!(SPEC_KEYWORDS, contains(eq(t.html_type())), "{t:?}");
        }
    }

    /// Two variants sharing a keyword would make them indistinguishable in the
    /// markup, and in the attribute table keyed on them.
    #[gtest]
    fn no_two_types_render_the_same() {
        let mut seen: Vec<&str> = ALL.iter().map(|t| t.html_type()).collect();
        seen.sort_unstable();
        seen.dedup();
        expect_that!(seen.len(), eq(ALL.len()));
    }
}
