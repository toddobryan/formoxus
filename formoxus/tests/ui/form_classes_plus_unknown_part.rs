//! A part that is no element at all: a parse error, with the caret on the part
//! and the nearest tag suggested. A real tag the widget lacks is
//! `form_classes_plus_part_the_widget_lacks`.
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
                select.lable: [form_label],
            },
        }
    });
}
