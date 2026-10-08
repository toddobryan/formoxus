//! A key given twice is refused at the second one, not silently resolved.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            email => { max_length: 10, max_length: 20 },
        }
    });
}
