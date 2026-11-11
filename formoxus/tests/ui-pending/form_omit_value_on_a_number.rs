//! `omit_value` leaves out a range's `<output>` (decision B), so on any other
//! widget it would do nothing and should be refused, as `no_trim` on a number
//! is. Pending: today `omit_value` is an unknown key.
use facet::Facet;
use formoxus::form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Mixer {
    volume: i32,
}

fn main() {
    let _ = form! { Mixer { volume => { widget: number, min: 0, max: 10, omit_value } } };
}
