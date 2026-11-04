//! `min_items` counts a list's or set's items, so on a string it does not
//! compile. A string's length is `min_length`.
use facet::Facet;
use formoxus::form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    title: String,
}

fn main() {
    let _ = form! { Post { title => { min_items: 1 } } };
}
