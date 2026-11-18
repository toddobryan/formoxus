//! `accept` is the file types a file field takes, so it applies to nothing else.
use facet::Facet;
use formoxus::{empty_form, form};

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
