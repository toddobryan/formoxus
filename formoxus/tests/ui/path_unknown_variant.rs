//! `path!` checks a variant segment the same way `form!` does.
#![allow(unused_imports)] use facet::Facet;
use formoxus::{FormoxusTypes, path};

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
