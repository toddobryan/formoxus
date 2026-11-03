//! A list does not say what its edits did, so `feedback` on one needs `set`.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: Vec<String>,
}

fn main() {
    let _ = empty_form::<Post>(form! {
        Post {
            tags => { feedback: |_| None },
        }
    });
}
