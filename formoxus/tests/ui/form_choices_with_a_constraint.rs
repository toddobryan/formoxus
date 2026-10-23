//! `choices` fix the values a field can take, so a validated key beside them is
//! refused: the dev makes each choice meet the constraint instead. Reported at
//! the constraint's key.
use facet::Facet;
use formoxus::{empty_form, form};

const STATES: &[(&str, &str)] = &[("AL", "Alabama")];

#[derive(Facet, Clone, Debug, PartialEq)]
struct Zip {
    code: String,
}

fn main() {
    let _ = empty_form::<Zip>(form! {
        Zip {
            code => { max_length: 10, widget: select, choices: STATES },
        }
    });
}
