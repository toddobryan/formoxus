//! `answers[]` is every ROW of the list, each a `String`, so it cannot be
//! reordered; the flag goes on the list's own path, `answers`.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Quiz {
    answers: Vec<String>,
}

fn main() {
    let _ = empty_form::<Quiz>(form! {
        Quiz {
            answers[] => { allow_reordering },
        }
    });
}
