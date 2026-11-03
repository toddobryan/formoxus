//! `item` shows a set's items, so it applies only to a set.
use facet::Facet;
use formoxus::widgets::ItemText;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Profile {
    nickname: String,
}

fn main() {
    let _ = empty_form::<Profile>(form! {
        Profile {
            nickname => { item: ItemText },
        }
    });
}
