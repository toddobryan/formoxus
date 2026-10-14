//! `button.<type>` checks the type as `buttons:` does.
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
                button.submitt: [btn],
            },
        }
    });
}
