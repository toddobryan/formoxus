//! A quoted key passes through unchecked, so a name the table knows must be
//! written as its key, which is checked.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            email => { "maxlength": "10" },
        }
    });
}
