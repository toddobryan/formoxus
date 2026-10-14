//! A selector given twice: the second would silently undo the first.
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
                select.select: [a],
                select.select: + [b],
            },
        }
    });
}
