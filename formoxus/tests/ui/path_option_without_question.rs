//! `path!` requires the `?` on an `Option` the same way `form!` does.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, path};

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
    let _ = path!(Contact.nickname);
}
