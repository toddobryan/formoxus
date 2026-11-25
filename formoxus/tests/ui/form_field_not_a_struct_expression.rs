//! `field:` written as a call. `form!` reads the field's type off a struct
//! expression or a unit struct, which the checks on a `widget:` beside it
//! need, and cannot from a call: refused at the expression.
use facet::Facet;
use formoxus::attrs::FieldType;
use formoxus::field::Field;
use formoxus::widgets::{AnyWidget, TextInput};
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Venue {
    capacity: u32,
}

#[derive(Clone, Debug, PartialEq)]
struct CapacityField {
    most: u32,
}

impl CapacityField {
    fn new(most: u32) -> Self {
        Self { most }
    }
}

impl Field for CapacityField {
    type Value = u32;
    type Context = ();
    const KIND: FieldType = FieldType::Int;

    fn default_widget(&self, _optional: bool) -> AnyWidget {
        TextInput.into()
    }
}

fn main() {
    let _ = empty_form::<Venue>(form! {
        Venue {
            capacity => { field: CapacityField::new(500) },
        }
    });
}
