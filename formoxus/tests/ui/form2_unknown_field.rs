//! The witness's whole reason for existing: a spec that names a field the model
//! does not have is a COMPILE error, not a spec that silently matches nothing.
//!
//! The span must land on the author's own `headlne`, inside the `form2!` call —
//! if it ever points at the macro instead, the witness has stopped being useful.
use facet::Facet;
use formoxus::form2;
use formoxus::reflect::empty_form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Article {
    headline: String,
}

fn main() {
    let _ = empty_form::<Article>(form2! {
        Article {
            headlne => { label: "Headline" },
        }
    });
}
