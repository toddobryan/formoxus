//! A `textarea` renders only a String field. `Option` is looked through.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Survey {
    score: Option<u32>,
}

fn main() {
    let _ = empty_form::<Survey>(form! {
        Survey {
            score => { widget: textarea },
        }
    });
}
