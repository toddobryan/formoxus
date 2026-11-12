//! A widget that renders one value on a set is refused at compile time, at
//! the widget, saying what a set takes instead. Before, this compiled and
//! panicked at the first render.
use facet::Facet;
use formoxus::{empty_form, form};
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    cities: BTreeSet<String>,
}

fn main() {
    let _ = empty_form::<Trip>(form! {
        Trip {
            cities => { widget: textarea },
        }
    });
}
