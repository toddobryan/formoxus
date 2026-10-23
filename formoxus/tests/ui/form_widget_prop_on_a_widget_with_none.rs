//! A built-in widget takes no props yet, so any prop is rustc's "no field".
//! `rows` belongs beside `widget:`, as an attribute.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Survey {
    comments: String,
}

fn main() {
    let _ = empty_form::<Survey>(form! {
        Survey {
            comments => { widget: textarea { rows: 5 } },
        }
    });
}
