//! `file` renders only a `FileUpload` field, not a struct of a dev's own,
//! even one shaped like a file.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Attachment {
    name: String,
    bytes: Vec<u8>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Assignment {
    essay: Attachment,
}

fn main() {
    let _ = empty_form::<Assignment>(form! {
        Assignment {
            essay => { widget: file },
        }
    });
}
