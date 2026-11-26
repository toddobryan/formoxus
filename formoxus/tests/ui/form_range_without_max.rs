//! As `form_range_without_min.rs`, for `max:`.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Mixer {
    volume: i32,
}

fn main() {
    let _ = form! { Mixer { volume => { widget: range, min: 0 } } };
}
