//! `[i]` names one item of a `Vec`. A set iterates in an order that need not
//! be the order its items are shown in (`HashSet` in none, `BTreeSet` by
//! `Ord`, which `sort_by` can disagree with), so an index into one could put
//! an error on the wrong item. Refused, with the reason.
use facet::Facet;
use formoxus::path;
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: BTreeSet<String>,
}

fn main() {
    let i = 0;
    let _ = path!(Post.tags[i]);
}
