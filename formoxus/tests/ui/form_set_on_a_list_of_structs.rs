//! A set's items are shown, not edited, so each must be one value; an
//! `Answer` is a struct, several.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Answer {
    text: String,
    correct: bool,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    answers: Vec<Answer>,
}

fn main() {
    let _ = empty_form::<Quiz>(form! {
        Quiz {
            answers => { set },
        }
    });
}
