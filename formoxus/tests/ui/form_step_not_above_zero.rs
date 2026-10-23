//! HTML ignores a `step` that is not greater than zero, leaving the implicit
//! step in force, so a zero or negative one is refused rather than emitted to
//! do nothing.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Reading {
    level: f64,
}

fn main() {
    let _ = empty_form::<Reading>(form! {
        Reading {
            level => { widget: number, step: 0 },
        }
    });
}
