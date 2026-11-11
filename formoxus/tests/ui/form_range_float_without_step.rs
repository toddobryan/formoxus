//! A range steps by 1 unless given a `step:`, a float range too, so 0.0 to 1.0
//! with no `step:` would offer only 0 and 1. A float field exists to hold
//! fractions, so `form!` requires `step:` on a float range (Todd, 2026-11-11).
//! The error points at `range` and names the missing `step:`.
use facet::Facet;
use formoxus::form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Dial {
    level: f64,
}

fn main() {
    let _ = form! { Dial { level => { widget: range, min: 0.0, max: 1.0 } } };
}
