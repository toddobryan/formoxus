//! A widget name in a class selector is checked as `widget:` checks it, minus
//! the `custom(…)` suggestion: a custom widget has no class slots.
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
                selct.select: [form_select],
            },
        }
    });
}
