//! `name` belongs to formoxus: it is the path the value submits under, so an
//! author who set it would detach the field from its value.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            email => { name: "hijack" },
        }
    });
}
