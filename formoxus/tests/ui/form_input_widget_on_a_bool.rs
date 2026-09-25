//! An `<input>` widget cannot render a bool field.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Settings {
    subscribed: bool,
}

fn main() {
    let _ = empty_form::<Settings>(form! {
        Settings {
            subscribed => { widget: text },
        }
    });
}
