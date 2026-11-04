//! An index is a `usize`, checked where it is written, so a wrong type is
//! an error on the author's expression rather than inside `format!`.
use facet::Facet;
use formoxus::path;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    answers: Vec<String>,
}

fn main() {
    let which = "first";
    let _ = path!(Quiz.answers[which]);
}
