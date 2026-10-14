//! `Classes` from a consumer's side: that each type is reachable at the path
//! the prelude rule promises, which only a dependent crate can prove.
//!
//! What `Classes::fx()` holds and how the builders behave is unit-tested in
//! `src/classes.rs`; what the classes render as is the snapshot's job.

use formoxus::classes::{FixedKey, Slot};
use formoxus::widgets::BuiltinWidget;
use googletest::prelude::*;

#[gtest]
fn classes_is_in_the_prelude_and_at_the_crate_root() {
    use formoxus::prelude::*;
    expect_that!(Classes::default(), eq(&formoxus::Classes::fx()));
}

#[gtest]
fn a_custom_classes_is_built_from_formoxus_classes_and_widgets() {
    let classes = formoxus::Classes::bare()
        .with_fixed(FixedKey::Form, Slot::new("form"))
        .with_part(
            BuiltinWidget::Textarea,
            "textarea",
            Slot::new("input").with_invalid("is-invalid"),
        );
    expect_that!(classes.fixed_class(FixedKey::Form, false), some(eq("form")));
    expect_that!(
        classes.part_class(BuiltinWidget::Textarea, "textarea", true),
        some(eq("input is-invalid"))
    );
    expect_that!(classes.fixed_class(FixedKey::FormTitle, false), none());
}
