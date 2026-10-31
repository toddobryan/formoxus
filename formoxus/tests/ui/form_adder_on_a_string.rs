//! `adder` applies only to a list field.
use facet::Facet;
use formoxus::widgets::AddRowButton;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Profile {
    nickname: String,
}

fn main() {
    let _ = empty_form::<Profile>(form! {
        Profile {
            nickname => { adder: AddRowButton },
        }
    });
}
