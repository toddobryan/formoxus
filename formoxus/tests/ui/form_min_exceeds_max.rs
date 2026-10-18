//! Bounds that no value could satisfy are a compile error. Named consts are
//! evaluated, not just literals. A bound has the field's own type, so both are
//! `f64` here; an integer const on a float field is a type error before this.
use facet::Facet;
use formoxus::{empty_form, form};

const FLOOR: f64 = 18.0;
const CEILING: f64 = 12.5;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    age: f64,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            age => { min: FLOOR, max: CEILING },
        }
    });
}
