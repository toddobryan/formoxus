//! A bound outside the field type's range excludes no value, so it is almost
//! always a typo or a leftover from a type change. Named consts are evaluated.
use facet::Facet;
use formoxus::{empty_form, form};

const LOWEST: i32 = -1000;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Thermostat {
    offset: i8,
}

fn main() {
    let _ = empty_form::<Thermostat>(form! {
        Thermostat {
            offset => { min: LOWEST },
        }
    });
}
