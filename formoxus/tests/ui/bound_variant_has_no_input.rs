//! An enum's variant select chooses a variant with `choose`, which rebuilds
//! the form beneath it. `input` would write a raw string over the enum's
//! structure entry, so a `BoundField<Variant>` has none (`BOUND_PLAN.md` F).
use formoxus::bound_field::{BoundField, Variant};

fn type_into_a_variant(field: BoundField<Variant>) {
    field.input("Circle".to_string());
}

fn main() {}
