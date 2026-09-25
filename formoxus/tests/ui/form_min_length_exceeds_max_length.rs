//! Length bounds that no value could satisfy are a compile error.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    title: String,
}

fn main() {
    let _ = empty_form::<Post>(form! {
        Post {
            title => { min_length: 20, max_length: 10 },
        }
    });
}
