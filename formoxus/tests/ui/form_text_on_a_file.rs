//! Only `file` renders a `FileUpload` field.
use facet::Facet;
use formoxus::file::FileUpload;
use formoxus::{empty_form, form};

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
