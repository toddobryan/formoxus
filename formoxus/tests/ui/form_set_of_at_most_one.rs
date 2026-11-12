//! A set held to at most one item is an optional value: the field should be
//! an `Option` of the item's type. Reported at `max_items`, on a `Vec`
//! marked `set` as on a real set.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Shelf {
    tags: Vec<String>,
}

fn main() {
    let _ = empty_form::<Shelf>(form! {
        Shelf {
            tags => { set, max_items: 1 },
        }
    });
}
