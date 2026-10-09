//! The enum refusal sees through an `Option`, as the rest of `field_kind` does,
//! so an optional enum gets the same reason as a plain one.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Shipping {
    Ground,
    Air,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Order {
    shipping: Option<Shipping>,
}

fn main() {
    let _ = empty_form::<Order>(form! {
        Order {
            shipping => { min_length: 2 },
        }
    });
}
