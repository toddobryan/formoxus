//! `max_size` is a file field's largest file, so it applies to nothing else.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Assignment {
    essay: String,
}

fn main() {
    let _ = empty_form::<Assignment>(form! {
        Assignment {
            essay => { max_size: 2_000_000 },
        }
    });
}
