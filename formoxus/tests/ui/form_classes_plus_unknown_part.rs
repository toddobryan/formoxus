//! A part the widget does not have, with the caret on the part and the list of
//! parts it does have.
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
                select.lable: [form_label],
            },
        }
    });
}
