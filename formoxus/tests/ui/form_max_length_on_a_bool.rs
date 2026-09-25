//! A length constraint on a field with no length is a compile error, with the
//! caret on the value, rather than a setting the runtime silently ignores.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Settings {
    subscribed: bool,
}

fn main() {
    let _ = empty_form::<Settings>(form! {
        Settings {
            subscribed => { max_length: 10 },
        }
    });
}
