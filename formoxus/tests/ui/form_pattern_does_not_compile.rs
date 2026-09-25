//! A `pattern` is compiled by the macro with the same engine and the same
//! `^(?:…)$` wrapping the runtime uses, so one that would fail at validate time
//! fails the build instead — with the caret on the author's string.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    zip: String,
}

fn main() {
    let _ = empty_form::<Address>(form! {
        Address {
            zip => { pattern: "(\\d{5}" },
        }
    });
}
