//! `$TrueFalse` alone, the spelling the runtime path uses, is refused: the
//! checks are a `match`, which has to name the enum.
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
            data.$TrueFalse.text => { label: "Prompt" },
        }
    });
}
