//! `answers[]` is every ITEM of the list, each a `String`, so it takes no
//! adder; the adder goes on the list's own path, `answers`.
use facet::Facet;
use formoxus::widgets::AddItemButton;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    answers: Vec<String>,
}

fn main() {
    let _ = empty_form::<Quiz>(form! {
        Quiz {
            answers[] => { adder: AddItemButton },
        }
    });
}
