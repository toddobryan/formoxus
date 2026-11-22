//! A set is sorted, so its items cannot be moved: `move_item` is a list's
//! (`BOUND_PLAN.md` F).
use formoxus::bound_field::{BoundField, Set};

fn move_in_a_set(field: BoundField<Set>) {
    field.move_item(0, 1);
}

fn main() {}
