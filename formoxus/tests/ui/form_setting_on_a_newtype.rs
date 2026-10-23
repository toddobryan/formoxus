//! A newtype has no default field, so a setting on one does not compile:
//! formoxus no longer guesses that a `Blurb(String)` is a `String` field.
//! Its field will be declared with `field:` once that exists.
use facet::Facet;
use formoxus::{empty_form, form};

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
