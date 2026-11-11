//! A range needs both `min:` and `max:` (decision A): an empty range starts at
//! the midpoint between them, and without them that would be the browser's
//! 0 to 100, which nobody chose. The error should point at `range` and name
//! the missing `min:`. Pending: today `range` is an unknown widget.
use facet::Facet;
use formoxus::form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Mixer {
    volume: i32,
}

fn main() {
    let _ = form! { Mixer { volume => { widget: range, max: 10 } } };
}
