//! An attribute the variant `<select>` cannot carry is refused on an enum
//! field, even though formoxus does not validate it.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

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
            shipping => { placeholder: "Pick one" },
        }
    });
}
