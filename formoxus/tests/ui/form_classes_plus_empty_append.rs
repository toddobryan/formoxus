//! `+ []` appends nothing, so it is refused; `[]` alone clears instead.
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
                button_row: + [],
            },
        }
    });
}
