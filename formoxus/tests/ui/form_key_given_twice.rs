//! A form key given twice is refused at the second one, like a field's key.
//! The error once pointed past the last entry, at the end of the form.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            live_validation: on,
            email => { label: "Email" },
            live_validation: off,
        }
    });
}
