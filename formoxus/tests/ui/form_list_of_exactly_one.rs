//! A list held to exactly one item is one value: the field should be the
//! item's own type. Reported at `max_items`.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Poll {
    answers: Vec<String>,
}

fn main() {
    let _ = empty_form::<Poll>(form! {
        Poll {
            answers => { min_items: 1, max_items: 1 },
        }
    });
}
