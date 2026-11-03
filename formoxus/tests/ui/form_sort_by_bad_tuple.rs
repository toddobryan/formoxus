//! A tuple is `(order, name)` and nothing else.
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
            tags => { sort_by: (String::cmp, "bytes", "extra") },
        }
    });
}
