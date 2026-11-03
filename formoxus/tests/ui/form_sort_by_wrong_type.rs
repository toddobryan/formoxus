//! The order's item type is the set's: a comparator over `Tag` does not fit
//! a set of `String`, and rustc says so at the order.
use facet::Facet;
use formoxus::{empty_form, form};
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Tag(String);

fn by_length(a: &Tag, b: &Tag) -> Ordering {
    a.0.len().cmp(&b.0.len())
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: BTreeSet<String>,
}

fn main() {
    let _ = empty_form::<Post>(form! {
        Post {
            tags => { sort_by: by_length },
        }
    });
}
