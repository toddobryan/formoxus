//! A bound outside the field type's range excludes no value, so it is almost
//! always a typo or a leftover from a type change. A bound has the field's own
//! type, so this is rustc's out-of-range error, at the value.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Thermostat {
    offset: i8,
}

fn main() {
    let _ = empty_form::<Thermostat>(form! {
        Thermostat {
            offset => { min: -1000 },
        }
    });
}
