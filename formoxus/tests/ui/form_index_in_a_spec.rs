//! A `form!` setting applies to every item, so `[0]` is refused, and the
//! message says where one item CAN be named: `path!`, for an error about it.
use facet::Facet;
use formoxus::form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    answers: Vec<String>,
}

fn main() {
    let _ = form! { Quiz { answers[0] => { label: "First" } } };
}
