//! Which element a class slot is for. Here rather than in `formoxus` so that
//! `form!`'s `classes_plus:` and the `Classes` it builds key slots the same
//! way. `formoxus` uses it internally and does NOT re-export it: a consumer
//! names slots through `Classes::with_part` and `with_fixed`, and through
//! `form!`.

use crate::attrs::str_eq;
use crate::fixed_keys::FixedKey;
use crate::widgets::BuiltinWidget;

/// One class slot: a part of a built-in widget, or an element no widget
/// renders.
///
/// A part is keyed by the `&'static str` name from [`BuiltinWidget::parts`].
/// The variant's fields cannot be private, so `Part(widget, "lable")` can be
/// written, and keys a slot nothing renders. Every route formoxus takes checks
/// the name instead: [`SlotKey::part`], `Classes::with_part`, `ClassEdit::part`
/// and `form!` at compile time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotKey {
    Part(BuiltinWidget, &'static str),
    Fixed(FixedKey),
}

impl SlotKey {
    /// The key for `widget`'s part called `name`, holding `parts()`'s own
    /// `&'static str` for it, or `None` if `widget` has no such part.
    pub const fn part(widget: BuiltinWidget, name: &str) -> Option<Self> {
        let parts = widget.parts();
        let mut i = 0;
        while i < parts.len() {
            if str_eq(name, parts[i].name) {
                return Some(Self::Part(widget, parts[i].name));
            }
            i += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::SlotKey;
    use crate::widgets::BuiltinWidget;
    use googletest::prelude::*;

    #[gtest]
    fn a_real_part_gets_a_key() {
        expect_that!(
            SlotKey::part(BuiltinWidget::Select, "label"),
            some(eq(SlotKey::Part(BuiltinWidget::Select, "label")))
        );
    }

    /// The name in the key is `parts()`'s, not the caller's string, which is
    /// what makes a key `'static` whatever it was looked up with.
    #[gtest]
    fn the_key_holds_the_parts_own_name() {
        let looked_up = String::from("legend");
        let Some(SlotKey::Part(_, name)) = SlotKey::part(BuiltinWidget::RadioGroup, &looked_up)
        else {
            panic!("a radio group has a legend");
        };
        expect_that!(
            std::ptr::eq(name, BuiltinWidget::RadioGroup.parts()[1].name),
            eq(true)
        );
    }

    #[gtest]
    fn a_part_the_widget_lacks_gets_none() {
        expect_that!(SlotKey::part(BuiltinWidget::Select, "lable"), none());
        expect_that!(SlotKey::part(BuiltinWidget::Textarea, "input"), none());
    }
}
