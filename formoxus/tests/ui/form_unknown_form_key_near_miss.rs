//! A misspelled form key suggests the nearest one, at the key, rather than
//! being read as a field missing its `=>`.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            live_validaton: off,
        }
    });
}
