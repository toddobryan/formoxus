//! An `Option<bool>` may be left unanswered, so it cannot also be required to
//! be true.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Survey {
    would_recommend: Option<bool>,
}

fn main() {
    let _ = empty_form::<Survey>(form! {
        Survey {
            would_recommend => { required_true },
        }
    });
}
