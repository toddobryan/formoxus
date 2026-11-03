//! Only a set of scalars has a default order; a set of newtypes needs
//! `sort_by`. Reported at the path.
use facet::Facet;
use formoxus::{empty_form, form};
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Tag(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Shelf {
    tags: BTreeSet<Tag>,
}

fn main() {
    let _ = empty_form::<Shelf>(form! {
        Shelf {
            tags => { label: "Tags" },
        }
    });
}
