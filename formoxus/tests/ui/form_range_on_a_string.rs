//! A slider produces a number, so `range` renders an integer or float field
//! only (`ACCEPTS` is `Int` and `Float`). On a `String` it is refused with
//! `cannot_render`'s sentence for `range`, the way `checkbox` on a `String` is.
//! Its `min:` and `max:` are refused as well, since they apply only to a number
//! field, and a range cannot be written without them, so this one golden shows
//! five errors rather than one.
use facet::Facet;
use formoxus::form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Profile {
    nickname: String,
}

fn main() {
    let _ = form! { Profile { nickname => { widget: range, min: 0, max: 10 } } };
}
