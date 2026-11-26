//! Only `file` renders a `FileUpload` field.
#![allow(unused_imports)] use facet::Facet;
use formoxus::file::FileUpload;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Assignment {
    essay: FileUpload,
}

fn main() {
    let _ = empty_form::<Assignment>(form! {
        Assignment {
            essay => { widget: text },
        }
    });
}
