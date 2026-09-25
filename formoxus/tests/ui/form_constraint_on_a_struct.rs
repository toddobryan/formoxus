//! A constraint on a path that is not a single input is a compile error. A
//! field set has no one value to measure, and the runtime used to drop it.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    city: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Order {
    address: Address,
}

fn main() {
    let _ = empty_form::<Order>(form! {
        Order {
            address => { max_length: 100 },
        }
    });
}
