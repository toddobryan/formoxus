//! A bare class name cannot hold a `-`. The caret spans the whole name, and the
//! message reads right even if `-mt-4` was meant as a second class.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    email: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            email => { class_plus: [wide, text-mt-4] },
        }
    });
}
