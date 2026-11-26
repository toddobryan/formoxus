//! A newtype no list holds has no field, so a setting on one does not
//! compile: formoxus ignores that `Blurb` wraps a `String`. Listing it in a
//! `types!` (`Blurb => newtype String`), or a `field:` on the path, gives it
//! one.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Blurb(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Profile {
    bio: Blurb,
}

fn main() {
    let _ = empty_form::<Profile>(form! {
        Profile {
            bio => { max_length: 280 },
        }
    });
}
