//! As `form_range_without_min.rs`, for `max:`. Pending: today `range` is an
//! unknown widget.
use facet::Facet;
use formoxus::form;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Mixer {
    volume: i32,
}

fn main() {
    let _ = form! { Mixer { volume => { widget: range, min: 0 } } };
}
