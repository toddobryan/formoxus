//! `required_true` means "must be true", so it applies only to a bool. Every
//! other non-`Option` field is already required by its type.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Profile {
    name: String,
}

fn main() {
    let _ = empty_form::<Profile>(form! {
        Profile {
            name => { required_true },
        }
    });
}
