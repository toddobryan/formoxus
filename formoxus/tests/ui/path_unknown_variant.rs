//! `path!` checks a variant segment the same way `form!` does.
use facet::Facet;
use formoxus::path;

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
    let _ = path!(Question.data.$QuestionData::Shrot.prompt);
}
