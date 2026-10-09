//! A constraint key on an enum field is a compile error with its own reason:
//! the field renders a variant picker, which submits no value to check.
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
    shipping: Shipping,
}

fn main() {
    let _ = empty_form::<Order>(form! {
        Order {
            shipping => { pattern: "[A-Z]+" },
        }
    });
}
