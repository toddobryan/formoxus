//! A field the variant lacks, even one another variant has, is a compile error
//! at the field.
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
            data.$QuestionData::TrueFalse.prompt => { label: "Prompt" },
        }
    });
}
