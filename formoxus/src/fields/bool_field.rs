use crate::{ValidationMessage, field::Field};

/// A `bool`, parsed from `"true"`/`"false"` by facet's default `parse`.
///
/// **An empty input is not this field's business.** `FieldNode` turns `""`
/// into `FieldValue::Empty` before `parse` is called, and what an empty bool
/// means depends on the WIDGET: an unticked checkbox is a present `false`, an
/// unanswered select is missing. `FieldNode` decides that (`is_unticked_checkbox`),
/// and hands an unticked box to `check` as `false`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoolField {
    /// The value must be `true` ("I agree to the terms").
    pub required_true: bool,
}

impl Field for BoolField {
    type Value = bool;

    type Context = ();

    fn check(&self, value: &bool) -> Vec<ValidationMessage> {
        if self.required_true && !value {
            vec![ValidationMessage("this value must be true".to_string())]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    /// `Field::check`, spelled out: googletest's prelude has a `check` too.
    fn messages(field: &BoolField, value: bool) -> Vec<String> {
        Field::check(field, &value)
            .into_iter()
            .map(|e| e.0)
            .collect()
    }

    /// Without `required_true`, `false` is a complete answer.
    #[gtest]
    fn a_plain_bool_accepts_either_value() {
        expect_that!(messages(&BoolField::default(), true), is_empty());
        expect_that!(messages(&BoolField::default(), false), is_empty());
    }

    /// "I agree to the terms" (issue #6).
    #[gtest]
    fn a_required_true_bool_rejects_false() {
        let must_agree = BoolField {
            required_true: true,
        };
        expect_that!(messages(&must_agree, true), is_empty());
        expect_that!(
            messages(&must_agree, false),
            elements_are![eq("this value must be true")]
        );
    }
}
