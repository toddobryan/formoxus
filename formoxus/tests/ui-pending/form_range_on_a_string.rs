//! A slider produces a number, so `range` renders an integer or float field
//! only (`ACCEPTS` is `Int` and `Float`). On a `String` it should be refused
//! with `cannot_render`'s sentence for `range`, the way `checkbox` on a
//! `String` is. Pending: today `range` is an unknown widget.
use facet::Facet;
use formoxus::form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Profile {
    nickname: String,
}

fn main() {
    let _ = form! { Profile { nickname => { widget: range, min: 0, max: 10 } } };
}
