//! `set` makes a list of newtypes a set, which then needs `sort_by`.
//! Reported at `set`.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Tag(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Shelf {
    tags: Vec<Tag>,
}

fn main() {
    let _ = empty_form::<Shelf>(form! {
        Shelf {
            tags => { set },
        }
    });
}
