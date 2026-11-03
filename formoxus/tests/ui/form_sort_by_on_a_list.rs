//! A list's order is the user's, so `sort_by` on one needs `set`.
use facet::Facet;
use formoxus::{empty_form, form};

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
