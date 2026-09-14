//! A dotted path is checked all the way down, not just at its first segment.
use facet::Facet;
use formoxus::form2;
use formoxus::reflect::empty_form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Venue {
    city: String,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Event {
    venue: Venue,
}

fn main() {
    let _ = empty_form::<Event>(form2! {
        Event {
            venue.zipcode => { label: "ZIP" },
        }
    });
}
