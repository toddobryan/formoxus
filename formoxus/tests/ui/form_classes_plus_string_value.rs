//! A class value is a list, as a field's `class:` is, not a string.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    name: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            classes_plus: {
                select.select: "form-select",
            },
        }
    });
}
