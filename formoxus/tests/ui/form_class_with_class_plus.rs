//! `class` replaces formoxus's classes and `class_plus` appends to them, so
//! a field takes one or the other, never both.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            email => { class: [wide], class_plus: [dark] },
        }
    });
}
