//! `field:` beside a key formoxus checks by setting it on the default field
//! (`min:` here). The named field replaces that field, so there is nothing for
//! the setting to go on: refused at the key, saying to give the setting to the
//! field itself.
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
struct CapacityField;

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
            capacity => { field: CapacityField, min: 1 },
        }
    });
}
