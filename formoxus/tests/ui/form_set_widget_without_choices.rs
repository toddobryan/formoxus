//! A set widget offers the set's items as choices, so a set of strings needs
//! `choices:` (decision E); only an enum without data brings its own.
use facet::Facet;
use formoxus::{empty_form, form};
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    cities: BTreeSet<String>,
}

fn main() {
    let _ = empty_form::<Trip>(form! {
        Trip {
            cities => { widget: checkbox_multiple },
        }
    });
}
