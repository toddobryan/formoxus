//! `+ []` appends nothing, so it is refused; `[]` alone clears instead.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    name: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            classes_plus: {
                button_row: + [],
            },
        }
    });
}
