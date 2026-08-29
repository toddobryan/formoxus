/*use serde::{Deserialize, Serialize};
use formoxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Question {
    pub id: u32,
    pub name: String,
    pub source: u32,
    pub data: QuestionData,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum QuestionData {
    MultipleChoice {
        text: String,
        can_shuffle: bool,
        select: SelectType,
        answer_choices: Vec<AnswerChoice>,
        explanation: String,
    },
    TrueFalse {
        text: String,
        answer: bool,
        explanation: String,
    },
    FillBlanks {
        text: String,
        correct_answers: Vec<String>,
        explanation: String,
        partial_correct: Vec<PartialCorrect>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SelectType {
    Single,
    Multiple,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnswerChoice {
    pub id: u8,
    pub text: String,
    pub is_correct: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartialCorrect {
    blank_index: u8,
    answer: String,
    credit: String,
}

#[derive(Form, Clone, Debug, Serialize, Deserialize)]
#[form(
    model = Question,
    button(type = "submit", name = save),
    button(type = "cancel", name = cancel),
)]
pub struct QuestionForm<T>  where T: FieldSet {
    pub name: String,
    pub source: u32,
    pub data: T,
}

#[derive(FieldSet, Clone, Debug, Serialize, Deserialize)]
#[field_set(model = QuestionData)]
pub struct MultChoiceFieldSet {
    pub text: String,
    pub can_shuffle: bool,
    pub is_multi_select: bool,
    #[form(multiple, min = 2, max = 10)]
    pub answer_choices: Vec<AnswerChoiceFieldSet>,
    pub explanation: String,
}

#[derive(FieldSet, Clone, Debug, Serialize, Deserialize)]
#[field_set(model = QuestionData)]
pub struct TrueFalseFieldSet {
    pub text: String,
    pub answer: bool,
    pub explanation: String,
}

#[derive(FieldSet, Clone, Debug, Serialize, Deserialize)]
#[field_set(model = AnswerChoice)]
pub struct AnswerChoiceFieldSet {
    pub text: String,
    pub is_correct: bool,
}

#[derive(FieldSet, Clone, Debug, Serialize, Deserialize)]
#[field_set(model = QuestionData)]
pub struct FillBlanksFieldSet {
    pub text: String,
    pub correct_answers: Vec<String>,
    #[form(multiple, min = 0)]
    pub partial_correct: Vec<PartialCorrectFieldSet>,
}

#[derive(FieldSet, Clone, Debug, Serialize, Deserialize)]
#[field_set(model = PartialCorrect)]
pub struct PartialCorrectFieldSet {
    pub blank_index: u8,
    pub answer: String,
    pub credit: String,
}
*/