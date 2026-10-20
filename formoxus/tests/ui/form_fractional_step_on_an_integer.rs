//! `step` is a pass-through attribute, so rustc does not type it as the
//! field's `T` the way it does `min`. An integer field can only ever hold
//! whole numbers, so a fractional step on one is a mistake. (A whole one,
//! `2.0`, is fine.)
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Order {
    quantity: u32,
}

fn main() {
    let _ = empty_form::<Order>(form! {
        Order {
            quantity => { widget: number, step: 0.5 },
        }
    });
}
