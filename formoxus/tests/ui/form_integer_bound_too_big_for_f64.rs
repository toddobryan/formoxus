//! An integer bound on a float field must survive the widening to `f64`.
//! 2^53 + 1 does not: it would be stored as 2^53, and the value one below the
//! stated minimum would get in.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Ledger {
    balance: f64,
}

fn main() {
    let _ = empty_form::<Ledger>(form! {
        Ledger {
            balance => { min: 9_007_199_254_740_993_i64 },
        }
    });
}
