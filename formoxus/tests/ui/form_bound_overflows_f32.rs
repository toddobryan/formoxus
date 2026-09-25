//! A bound beyond what the field's float type can hold is a compile error.
//! `max: 1e50` on an `f32` looks satisfiable and is not: parsing saturates, so
//! `1e39` arrives as `inf` and is refused by a bound it is visibly inside.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Reading {
    level: f32,
}

fn main() {
    let _ = empty_form::<Reading>(form! {
        Reading {
            level => { max: 1e50 },
        }
    });
}
