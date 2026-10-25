//! A path through an `Option<Struct>` without its `?`: rustc's own error, since
//! there is no `city` on an `Option`.
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
            address.city => { label: "Town" },
        }
    });
}
