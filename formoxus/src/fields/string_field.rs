use formoxus_spec::{Attr, AttrValue};
use regress::Regex;

use crate::{ValidationMessage, field::Field, field_attrs::FieldAttrs};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct StringField {
    /// In characters, not bytes.
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    /// HTML `pattern` syntax: anchored to the whole value and compiled with
    /// the `v` flag, as a browser does. `form!` checks it compiles.
    pub pattern: Option<&'static str>,
}

impl Field for StringField {
    type Value = String;

    type Context = ();

    /// `minlength`, `maxlength` and `pattern`, so the browser checks what
    /// `check` does. The pattern goes out unanchored: a browser anchors it.
    fn widget_attrs(&self) -> FieldAttrs {
        let mut attrs = FieldAttrs::default();
        if let Some(min) = self.min_length {
            attrs = attrs.with_default(Attr::MinLength, AttrValue::Int(min));
        }
        if let Some(max) = self.max_length {
            attrs = attrs.with_default(Attr::MaxLength, AttrValue::Int(max));
        }
        if let Some(patt) = self.pattern {
            attrs = attrs.with_default(Attr::Pattern, AttrValue::Regex(patt));
        }
        attrs
    }

    fn check(&self, value: &Self::Value) -> Vec<ValidationMessage> {
        let mut errors = Vec::new();
        let char_length = value.chars().count();
        if let Some(min) = self.min_length
            && char_length < min
        {
            errors.push(ValidationMessage(format!("length must be at least {min}")));
        }
        if let Some(max) = self.max_length
            && char_length > max
        {
            errors.push(ValidationMessage(format!("length must be at most {max}")));
        }
        if let Some(patt) = self.pattern {
            let re = Regex::with_flags(&format!("^(?:{patt})$"), "v")
                .expect("this regex should have parsed at compile time");
            if re.find(value).is_none() {
                errors.push(ValidationMessage(format!(
                    "input should match the regular expression {patt}"
                )));
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    /// `Field::check`, spelled out: googletest's prelude has a `check` too.
    fn messages(field: &StringField, value: &str) -> Vec<String> {
        Field::check(field, &value.to_string())
            .into_iter()
            .map(|e| e.0)
            .collect()
    }

    fn lengths(min_length: Option<usize>, max_length: Option<usize>) -> StringField {
        StringField {
            min_length,
            max_length,
            ..StringField::default()
        }
    }

    fn pattern(pattern: &'static str) -> StringField {
        StringField {
            pattern: Some(pattern),
            ..StringField::default()
        }
    }

    #[gtest]
    fn no_settings_never_complains() {
        expect_that!(
            messages(&StringField::default(), "anything at all"),
            is_empty()
        );
    }

    #[gtest]
    fn both_lengths_are_inclusive() {
        let field = lengths(Some(2), Some(4));
        expect_that!(
            messages(&field, "a"),
            elements_are![eq("length must be at least 2")]
        );
        expect_that!(messages(&field, "ab"), is_empty());
        expect_that!(messages(&field, "abcd"), is_empty());
        expect_that!(
            messages(&field, "abcde"),
            elements_are![eq("length must be at most 4")]
        );
    }

    /// `José` is four characters and five bytes; an emoji is one character.
    #[gtest]
    fn length_counts_characters_not_bytes() {
        expect_that!(messages(&lengths(Some(4), Some(4)), "José"), is_empty());
        expect_that!(messages(&lengths(None, Some(1)), "🦀"), is_empty());
    }

    /// HTML anchors a `pattern` to the whole value, so this must too, with the
    /// alternation grouped first: `cat|dog` must not accept `catfish`.
    #[gtest]
    fn a_pattern_must_match_the_whole_value() {
        let zip = pattern(r"\d{5}");
        expect_that!(messages(&zip, "90210"), is_empty());
        expect_that!(messages(&zip, "abc12345xyz"), len(eq(1)));
        let pets = pattern("cat|dog");
        expect_that!(messages(&pets, "dog"), is_empty());
        expect_that!(messages(&pets, "catfish"), len(eq(1)));
    }

    /// `regress` gives ECMAScript semantics, which is the point of choosing it
    /// over `regex`: the server cannot disagree with a browser about what a
    /// `pattern` means. Anchors are NOT multiline, `$` does not match before a
    /// trailing newline the way Perl's does, and `.` excludes a newline. All
    /// three matter on a `<textarea>`, whose value legitimately has newlines.
    #[gtest]
    fn anchors_and_dot_follow_javascript_not_perl() {
        let five = pattern(r"\d{5}");
        expect_that!(messages(&five, "12345\n67890"), len(eq(1)));
        expect_that!(messages(&five, "12345\n"), len(eq(1)));
        expect_that!(messages(&pattern(r".+"), "a\nb"), len(eq(1)));
        expect_that!(messages(&pattern(r"(.|\n)+"), "a\nb"), is_empty());
    }

    /// The `v` flag, as a browser compiles `pattern`: `\p{L}` is a letter class.
    #[gtest]
    fn a_pattern_is_compiled_with_the_v_flag() {
        expect_that!(messages(&pattern(r"\p{L}+"), "héllo"), is_empty());
    }

    #[gtest]
    fn a_length_and_a_pattern_both_report_in_that_order() {
        let field = StringField {
            min_length: Some(10),
            pattern: Some(r"\d+"),
            ..StringField::default()
        };
        expect_that!(
            messages(&field, "abc"),
            elements_are![
                eq("length must be at least 10"),
                eq(r"input should match the regular expression \d+"),
            ]
        );
    }
}
