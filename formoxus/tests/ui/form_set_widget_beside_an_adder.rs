//! A widget renders the whole set, adder included, so `adder:` beside one
//! would do nothing (decision H).
use dioxus::prelude::*;
use facet::Facet;
use formoxus::widgets::AddItemProps;
use formoxus::{empty_form, form};
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    cities: BTreeSet<String>,
}

const CITIES: &[(&str, &str)] = &[("paris", "Paris")];

#[component]
fn Adder(props: AddItemProps) -> Element {
    let _ = props;
    rsx! {}
}

fn main() {
    let _ = empty_form::<Trip>(form! {
        Trip {
            cities => { widget: checkbox_multiple, choices: CITIES, adder: Adder },
        }
    });
}
