//! A list held to at most one item is an optional value: the field should be
//! an `Option` of the item's type. Reported at `max_items`.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Poll {
    answers: Vec<String>,
}

fn main() {
    let _ = empty_form::<Poll>(form! {
        Poll {
            answers => { max_items: 1 },
        }
    });
}
