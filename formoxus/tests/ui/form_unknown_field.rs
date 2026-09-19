//! The witness's whole reason for existing: a spec that names a field the model
//! does not have is a COMPILE error, not a spec that silently matches nothing.
//!
//! The span must land on the author's own `headlne`, inside the `form!` call —
//! if it ever points at the macro instead, the witness has stopped being useful.
use facet::Facet;
use formoxus::form;
use formoxus::empty_form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Article {
    headline: String,
}

fn main() {
    let _ = empty_form::<Article>(form! {
        Article {
            headlne => { label: "Headline" },
        }
    });
}
