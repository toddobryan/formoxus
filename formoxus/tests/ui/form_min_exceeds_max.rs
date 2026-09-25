//! Bounds that no value could satisfy are a compile error. Named consts are
//! evaluated, not just literals, and an integer and a float compare.
use facet::Facet;
use formoxus::{empty_form, form};

const FLOOR: u32 = 18;
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
