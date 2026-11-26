//! `accept` is the file types a file field takes, so it applies to nothing else.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Assignment {
    essay: String,
}

fn main() {
    let _ = empty_form::<Assignment>(form! {
        Assignment {
            essay => { accept: ".pdf" },
        }
    });
}
