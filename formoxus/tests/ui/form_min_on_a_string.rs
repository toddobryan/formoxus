//! A numeric bound on a text field is a compile error. `Option` is looked
//! through, so this is caught on an optional field too.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Profile {
    nickname: Option<String>,
}

fn main() {
    let _ = empty_form::<Profile>(form! {
        Profile {
            nickname? => { min: 3 },
        }
    });
}
