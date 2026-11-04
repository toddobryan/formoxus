//! `[= value]` names a set's item by its value. A `Vec` can hold a value
//! twice, so a value would not name one item of it; a `Vec`'s items are named
//! by index, `[i]`, and the error says so.
use facet::Facet;
use formoxus::path;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    answers: Vec<String>,
}

fn main() {
    let _ = path!(Quiz.answers[= "alpha"]);
}
