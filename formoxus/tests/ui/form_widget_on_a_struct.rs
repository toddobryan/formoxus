//! A widget needs a single-value field. On a struct it used to be a runtime
//! panic, and on an enum it was silently ignored.
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
            address => { widget: textarea },
        }
    });
}
