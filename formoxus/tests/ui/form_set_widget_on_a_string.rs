//! A set widget on a field that is not a set is refused, at the widget.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    city: String,
}

const CITIES: &[(&str, &str)] = &[("paris", "Paris")];

fn main() {
    let _ = empty_form::<Trip>(form! {
        Trip {
            city => { widget: checkbox_multiple, choices: CITIES },
        }
    });
}
