//! A slider cannot be unset, so it cannot say `None`: `range` on an
//! `Option<i32>` should be refused with `optional_refusal`'s sentence for it,
//! as `checkbox` on an `Option<bool>` is. Pending: today `range` is an unknown
//! widget.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Mixer {
    volume: Option<i32>,
}

fn main() {
    let _ = empty_form::<Mixer>(form! {
        Mixer {
            volume? => { widget: range, min: 0, max: 10 },
        }
    });
}
