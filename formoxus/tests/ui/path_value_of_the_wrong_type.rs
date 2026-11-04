//! The value has to be one of the set's items, or something they borrow as
//! (`&str` for a set of `String`s): a number is neither.
use facet::Facet;
use formoxus::path;
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: BTreeSet<String>,
}

fn main() {
    let _ = path!(Post.tags[= &7]);
}
