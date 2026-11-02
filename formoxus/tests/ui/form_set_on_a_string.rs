//! `set` applies only to list fields.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Profile {
    nickname: String,
}

fn main() {
    let _ = empty_form::<Profile>(form! {
        Profile {
            nickname => { set },
        }
    });
}
