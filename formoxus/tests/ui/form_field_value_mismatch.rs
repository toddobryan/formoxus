//! `field:` names a `Field` whose `Value` is not the path's type: a field for
//! `u32` on a `String`. The error should point at the field, name both types,
//! and peel an `Option` first, so a `u32` field fits an `Option<u32>`.
#![allow(unused_imports)] use facet::Facet;
use formoxus::attrs::FieldType;
use formoxus::field::Field;
use formoxus::widgets::{AnyWidget, TextInput};
use formoxus::{FormoxusTypes, empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Venue {
    city: String,
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
            city => { field: CapacityField },
        }
    });
}
