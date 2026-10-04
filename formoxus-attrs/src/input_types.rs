//! Every `<input type=…>` value HTML defines.

/// Every `type` an `<input>` can have, per the WHATWG spec: all 22, whether
/// or not formoxus renders it.
///
/// This is a set of FACTS, not a list of what formoxus can draw. The attribute
/// table needs every type, so that a row can say where `checked` is valid
/// (`Checkbox | Radio`) or that `accept` belongs to `File`. Which of these a
/// field can actually be rendered as is a separate, smaller enum on the
/// `formoxus` side, so that a field rendered as `Radio` or `Submit` cannot even
/// be written (see `.claude/memory/attribute_rules_design.md`, decision 6).
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
    // Not rendered by the `Input` widget. A checkbox has its own state
    // (`checked`, not `value`), a lone radio is meaningless outside a group,
    // and the last five are buttons or files, not field values.
    Checkbox,
    Radio,
    File,
    Range,
    Submit,
    Image,
    Reset,
    Button,
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
            // mid-keystroke. A numeric renders as text and formoxus parses it.
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
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::File => "file",
            Self::Range => "range",
            Self::Submit => "submit",
            Self::Image => "image",
            Self::Reset => "reset",
            Self::Button => "button",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::InputType;
    use googletest::prelude::*;

    const ALL: [InputType; 22] = [
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
        InputType::Checkbox,
        InputType::Radio,
        InputType::File,
        InputType::Range,
        InputType::Submit,
        InputType::Image,
        InputType::Reset,
        InputType::Button,
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
    /// markup, and in the attribute table keyed on them. Together with the test
    /// above and `ALL` having 22 entries, this means every spec keyword has
    /// exactly one variant.
    /// `ALL` has to name every variant, or the two tests above check a subset.
    /// The `match` has no wildcard, so adding a variant to `InputType` stops
    /// this test from COMPILING until it is listed here, which is the prompt
    /// to add it to `ALL` too. The length check then ties `ALL` to the spec's
    /// 22 keywords, and `no_two_types_render_the_same` rules out duplicates.
    #[gtest]
    fn all_lists_every_variant() {
        for t in ALL {
            match t {
                InputType::Text
                | InputType::Password
                | InputType::Hidden
                | InputType::Number
                | InputType::Email
                | InputType::Telephone
                | InputType::Url
                | InputType::Search
                | InputType::Color
                | InputType::Date
                | InputType::Time
                | InputType::DatetimeLocal
                | InputType::Month
                | InputType::Week
                | InputType::Checkbox
                | InputType::Radio
                | InputType::File
                | InputType::Range
                | InputType::Submit
                | InputType::Image
                | InputType::Reset
                | InputType::Button => {}
            }
        }
        expect_that!(ALL.len(), eq(SPEC_KEYWORDS.len()));
    }

    #[gtest]
    fn no_two_types_render_the_same() {
        let mut seen: Vec<&str> = ALL.iter().map(|t| t.html_type()).collect();
        seen.sort_unstable();
        seen.dedup();
        expect_that!(seen.len(), eq(ALL.len()));
    }
}
