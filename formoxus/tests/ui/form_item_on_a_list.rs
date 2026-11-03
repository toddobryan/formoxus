//! A list's items are edited in place, not shown as text, so `item` on one
//! needs `set`.
use facet::Facet;
use formoxus::widgets::ItemText;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: Vec<String>,
}

fn main() {
    let _ = empty_form::<Post>(form! {
        Post {
            tags => { item: ItemText },
        }
    });
}
