//! A fractional bound on an integer field would have to be rounded, and
//! rounding either way changes which values pass. (A whole one, `2.0`, is fine.)
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Order {
    quantity: u32,
}

fn main() {
    let _ = empty_form::<Order>(form! {
        Order {
            quantity => { min: 1.5 },
        }
    });
}
