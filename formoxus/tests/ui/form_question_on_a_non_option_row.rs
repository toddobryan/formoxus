//! `?` mid-path on a field that is not an `Option`: here, a row of the list.
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
            phones?[]? => { widget: email },
        }
    });
}
