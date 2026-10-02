//! A checkbox has two states and an `Option<bool>` has three, so an unticked
//! box could mean either `None` or `Some(false)`. Before this check, which one
//! you got depended on whether the box had ever been touched. The default
//! widget for an `Option<bool>` is a `select`, and asking for a `checkbox` is
//! an error, as `radio_group` on an `Option` is.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Survey {
    would_recommend: Option<bool>,
}

fn main() {
    let _ = empty_form::<Survey>(form! {
        Survey {
            would_recommend => { widget: checkbox },
        }
    });
}
