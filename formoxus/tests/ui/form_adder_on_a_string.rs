//! `adder` applies only to list or set fields.
use facet::Facet;
use formoxus::widgets::AddItemButton;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Profile {
    nickname: String,
}

fn main() {
    let _ = empty_form::<Profile>(form! {
        Profile {
            nickname => { adder: AddItemButton },
        }
    });
}
