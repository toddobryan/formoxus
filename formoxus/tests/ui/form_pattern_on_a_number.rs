//! A `pattern` on a field with no text is a compile error, with the caret on
//! the pattern. Asked of the attribute table as `Attr::Pattern`, not borrowed
//! from the length check.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Order {
    quantity: u32,
}

fn main() {
    let _ = empty_form::<Order>(form! {
        Order {
            quantity => { pattern: r"\d+" },
        }
    });
}
