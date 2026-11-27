//! `set` takes a list of structs, as a `BTreeSet` field does, but a struct
//! has no default order, so the set needs `sort_by`.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Answer {
    text: String,
    correct: bool,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    answers: Vec<Answer>,
}

fn main() {
    let _ = empty_form::<Quiz>(form! {
        Quiz {
            answers => { set },
        }
    });
}
