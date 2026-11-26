//! A `BTreeSet` is already a set, so `set` on it is redundant.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};
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
