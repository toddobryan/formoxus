//! A `select` with nothing to choose from is a declaration the author did not
//! finish. It used to render as nothing, silently, because Dioxus contains the
//! render-time panic to the one component. A bool is the exception: its
//! choices can be derived.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Address {
    state: String,
}

fn main() {
    let _ = empty_form::<Address>(form! {
        Address {
            state => { widget: select },
        }
    });
}
