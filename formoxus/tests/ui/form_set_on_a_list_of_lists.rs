//! A set holds what a `BTreeSet` field holds, structs included, but not a
//! list: a list writes its own item keys at its path, so a set would show and
//! compare its items by keys. Refused once, by `set`, not again by the order
//! check.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Grid {
    rows: Vec<Vec<String>>,
}

fn main() {
    let _ = empty_form::<Grid>(form! {
        Grid {
            rows => { set },
        }
    });
}
