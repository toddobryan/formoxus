//! A literal `accept` is parsed by `form!`, as the browser will read it, so a
//! malformed one is refused at the string.
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
            essay => { accept: ".pdf, docx" },
        }
    });
}
