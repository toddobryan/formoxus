//! A real tag the widget does not render: rustc's `E0080` from the `has_part`
//! assert, with the caret on the part. The macro cannot list the parts the
//! widget has, since they live only in `W::PARTS`.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Signup {
    name: String,
}

fn main() {
    let _ = empty_form::<Signup>(form! {
        Signup {
            classes_plus: {
                textarea.legend: [form_legend],
            },
        }
    });
}
