//! A set item's view shows the item; it does not change it. Writing the
//! item's value directly would skip the set's duplicate check, so a
//! `BoundField<SetItem>` reads (`current`) and nothing else (`BOUND_PLAN.md` F).
use formoxus::widgets::BoundField;
use formoxus::widgets::bound_field::SetItem;

fn rewrite_an_item(field: BoundField<SetItem>) {
    field.input("rust".to_string());
}

fn main() {}
