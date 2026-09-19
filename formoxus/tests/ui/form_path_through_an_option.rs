//! A KNOWN LIMITATION, pinned so it stays visible.
//!
//! `notes.city` is a perfectly good *member* path at runtime — `OptionMember`
//! peels the `Option` and the `FieldSet` below it is addressed exactly that way.
//! But there is no Rust access spelled `s.notes.city` for an `Option<Venue>`, so
//! the witness cannot express it and this will not compile.
//!
//! If the witness ever learns to see through an `Option` (an `if let`, say), this
//! test is the one to delete.
use facet::Facet;
use formoxus::form;
use formoxus::empty_form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Venue {
    city: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Event {
    notes: Option<Venue>,
}

fn main() {
    let _ = empty_form::<Event>(form! {
        Event {
            notes.city => { label: "City" },
        }
    });
}
