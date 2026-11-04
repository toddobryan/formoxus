//! `no_trim` is a text field's setting. A number is always trimmed (no number
//! has a meaningful space) and a `bool` is read as sent, so neither has one,
//! and `no_trim` on either does not compile.
use facet::Facet;
use formoxus::form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Order {
    quantity: u8,
}

fn main() {
    let _ = form! { Order { quantity => { no_trim } } };
}
