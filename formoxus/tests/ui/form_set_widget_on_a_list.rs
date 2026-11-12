//! A `Vec` is a list, whose order and duplicates a set widget cannot show:
//! it takes one only with `set` beside it.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    cities: Vec<String>,
}

const CITIES: &[(&str, &str)] = &[("paris", "Paris")];

fn main() {
    let _ = empty_form::<Trip>(form! {
        Trip {
            cities => { widget: select_multiple, choices: CITIES },
        }
    });
}
