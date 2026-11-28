//! The `InputType` re-export, and the two choices a `bool` offers.

/// Defined in `formoxus-spec`, because the attribute table and `form!` both
/// key on it and `formoxus-macros` cannot reach this crate. Re-exported here so
/// that `formoxus::widgets::InputType` is still its address.
pub use formoxus_spec::InputType;

/// The two choices for a `bool` field. If used in a [`super::Select`]
/// for an `Option<bool>` field, the [`super::Select`] provides the
/// "no value" choice itself.
pub(super) fn bool_choices() -> Vec<crate::fields::Choice> {
    vec![
        crate::fields::Choice::new("true", "True"),
        crate::fields::Choice::new("false", "False"),
    ]
}
