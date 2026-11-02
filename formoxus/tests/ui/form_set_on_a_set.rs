//! A `BTreeSet` is already a set, so `set` on it is redundant.
use facet::Facet;
use formoxus::{empty_form, form};
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: BTreeSet<String>,
}

fn main() {
    let _ = empty_form::<Post>(form! {
        Post {
            tags => { set },
        }
    });
}
