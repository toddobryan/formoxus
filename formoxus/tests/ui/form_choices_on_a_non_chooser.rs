//! Choices are gated by widget, so a list handed to something that cannot show
//! one is a compile error naming the widgets that would have worked — rather
//! than a setting silently dropped at render.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    state: String,
}

const STATES: &[(&str, &str)] = &[("AL", "Alabama")];

fn main() {
    let _ = empty_form::<Address>(form! {
        Address {
            state => { widget: text { choices: STATES } },
        }
    });
}
