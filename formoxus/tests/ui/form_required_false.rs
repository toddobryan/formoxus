//! `required: false` is never allowed, on any field. Presence comes from the
//! model's type alone: a non-`Option` field is always required and an `Option`
//! never is, so an explicit `false` would describe a form the model cannot
//! hold. On a bool, where `required: true` means "must be ticked", `false` is
//! just the default. A bool is used here because it is the one field type where
//! writing `false` looks reasonable.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Terms {
    agreed: bool,
}

fn main() {
    let _ = empty_form::<Terms>(form! {
        Terms {
            agreed => { required: false },
        }
    });
}
