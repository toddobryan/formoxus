//! A comma inside a style value ends the declaration, so `serif` reads as the
//! next property. The error lands on `serif` and says to quote the value.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            email => { style: { color: red, font_family: Inter, serif } },
        }
    });
}
