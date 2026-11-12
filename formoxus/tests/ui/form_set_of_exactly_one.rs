//! A set held to exactly one item is one value: the field should be the
//! item's own type. Reported at `max_items`.
use facet::Facet;
use formoxus::{empty_form, form};
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Shelf {
    tags: BTreeSet<String>,
}

fn main() {
    let _ = empty_form::<Shelf>(form! {
        Shelf {
            tags => { min_items: 1, max_items: 1 },
        }
    });
}
