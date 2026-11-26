//! A list's order is the user's, so `sort_by` on one needs `set`.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: Vec<String>,
}

fn main() {
    let _ = empty_form::<Post>(form! {
        Post {
            tags => { sort_by: String::cmp },
        }
    });
}
