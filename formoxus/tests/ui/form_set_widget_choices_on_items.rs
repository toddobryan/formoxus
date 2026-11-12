//! A set with a widget takes its choices on its own line, beside the widget
//! that shows them (decision D), not on its items' line.
use facet::Facet;
use formoxus::{empty_form, form};
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    cities: BTreeSet<String>,
}

const CITIES: &[(&str, &str)] = &[("paris", "Paris")];

fn main() {
    let _ = empty_form::<Trip>(form! {
        Trip {
            cities => { widget: checkbox_multiple },
            cities[] => { choices: CITIES },
        }
    });
}
