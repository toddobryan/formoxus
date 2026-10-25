//! The widget-accepts check reaches a field inside a variant: the projection
//! types the field through a `match`, so `checkbox` on a String still fails.
use facet::Facet;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum QuestionData {
    TrueFalse { text: String, answer: bool },
    Short { prompt: String },
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Question {
    data: QuestionData,
}

fn main() {
    let _ = empty_form::<Question>(form! {
        Question {
            data.$QuestionData::TrueFalse.text => { widget: checkbox },
        }
    });
}
