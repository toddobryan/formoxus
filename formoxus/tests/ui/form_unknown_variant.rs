//! A misspelled variant is a compile error at the variant, the same as a
//! misspelled field: the witness is a `match` whose pattern names it.
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
            data.$QuestionData::TrueFals.text => { label: "Prompt" },
        }
    });
}
