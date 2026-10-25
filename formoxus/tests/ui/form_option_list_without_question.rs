//! `[]` on an `Option<Vec<T>>` without the `?` would type-check, because
//! `Option` has an `.iter()` of its own; the const check refuses it.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    city: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Contact {
    name: String,
    nickname: Option<String>,
    address: Option<Address>,
    phones: Option<Vec<String>>,
}

fn main() {
    let _ = empty_form::<Contact>(form! {
        Contact {
            phones[] => { widget: email },
        }
    });
}
