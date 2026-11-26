//! `file` renders only a `FileUpload` field.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Assignment {
    essay: String,
}

fn main() {
    let _ = empty_form::<Assignment>(form! {
        Assignment {
            essay => { widget: file },
        }
    });
}
