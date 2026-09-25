//! A widget nothing renders for any field is rejected while `form!` parses,
//! before the field's type is known.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Upload {
    document: String,
}

fn main() {
    let _ = empty_form::<Upload>(form! {
        Upload {
            document => { widget: file },
        }
    });
}
