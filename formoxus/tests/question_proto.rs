use serde::{Deserialize, Serialize};
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

#[derive(Form, Clone, Debug, Serialize, Deserialize)]
#[form(
    model = Question,
    button(type = "submit", name = save),
    button(type = "cancel", name = cancel),
)]
pub struct QuestionForm<T: FieldSet> {
    pub name: String,
    pub source: u32,
    #[form(field_set)]
    pub data: T,
}

impl<T: FieldSet + From<QuestionData>> From<Question> for QuestionForm<T> {
    fn from(value: Question) -> Self {
        QuestionForm {
            name: value.name,
            source: value.source,
            data: T::from(value.data),
        }
    }
}

impl<T: FieldSet + Into<QuestionData>> From<QuestionForm<T>> for Question {
    fn from(value: QuestionForm<T>) -> Self {
        Question {
            id: 42, // still need a real story for this — see below
            name: value.name,
            source: value.source,
            data: value.data.into(),
        }
    }
}

#[derive(FieldSet, Clone, Debug, Serialize, Deserialize)]
#[field_set(model = QuestionData)]
pub struct TrueFalseFieldSet {
    pub text: String,
    pub answer: bool,
    pub explanation: String,
}

impl From<QuestionData> for TrueFalseFieldSet {
    fn from(value: QuestionData) -> Self {
        match value {
            QuestionData::TrueFalse { text, answer, explanation } =>
                TrueFalseFieldSet { text, answer, explanation },
            _ => unreachable!("shouldn't be called except with TrueFalse variant")
        }
    }
}

impl From<TrueFalseFieldSet> for QuestionData {
    fn from(value: TrueFalseFieldSet) -> Self {
        QuestionData::TrueFalse { 
            text: value.text, 
            answer: value.answer, 
            explanation: value.explanation
        }
    }
}

#[derive(FieldSet, Clone, Debug, Serialize, Deserialize)]
#[field_set(model = QuestionData)]
pub struct MultChoiceFieldSet {
    pub text: String,
    pub can_shuffle: bool,
    pub is_multi_select: bool,
    #[form(field_set)]
    pub answer_choices: Vec<AnswerChoiceFieldSet>,
    pub explanation: String,
}

impl From<MultChoiceFieldSet> for QuestionData {
    fn from(value: MultChoiceFieldSet) -> Self {
        QuestionData::MultipleChoice { 
            text:value.text, 
            can_shuffle: value.can_shuffle, 
            select: match value.is_multi_select {
                true => SelectType::Multiple,
                false => SelectType::Single,
            }, 
            answer_choices: value.answer_choices.iter().enumerate().map(|i_ac| {
                AnswerChoice {
                    id: i_ac.0 as u8,
                    text: i_ac.1.text.clone(),
                    is_correct: i_ac.1.is_correct,
                }
            }).collect(),
            explanation: value.explanation,
         }
    }
}

impl From<QuestionData> for MultChoiceFieldSet {
    fn from(value: QuestionData) -> Self {
        match value {
            QuestionData::MultipleChoice { text, can_shuffle, select, answer_choices, explanation } =>
                MultChoiceFieldSet { 
                    text, 
                    can_shuffle, 
                    is_multi_select: select == SelectType::Multiple, 
                    answer_choices: answer_choices.iter().map(|ac: &AnswerChoice| {
                        AnswerChoiceFieldSet { text: ac.text.clone(), is_correct: ac.is_correct }
                    }).collect(), 
                    explanation, 
                },
            _ => unreachable!("should only be called with QuestionData::MultipleChoice variant"),
        }
    }
}

#[derive(FieldSet, Clone, Debug, Serialize, Deserialize)]
pub struct AnswerChoiceFieldSet {
    pub text: String,
    pub is_correct: bool,
}

#[derive(FieldSet, Clone, Debug, Serialize, Deserialize)]
#[field_set(model = QuestionData)]
pub struct FillBlanksFieldSet {
    pub text: String,
    //pub correct_answers: Vec<String>,
    //#[form(field_set)]
    //pub partial_correct: Vec<PartialCorrectFieldSet>,
}

impl From<QuestionData> for FillBlanksFieldSet {
    fn from(value: QuestionData) -> Self {
        match value {
            QuestionData::FillBlanks { text, .. } =>
                FillBlanksFieldSet { text },
            _ => unreachable!("should only be called with FillBlanks variant"),
        }
    }
}

impl From<FillBlanksFieldSet> for QuestionData {
    fn from(value: FillBlanksFieldSet) -> Self {
        QuestionData::FillBlanks { 
            text: value.text, 
            correct_answers: Vec::new(), 
            explanation: String::from("explanation"), 
            partial_correct: Vec::new(), 
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PartialCorrect {
    blank_index: u8,
    answer: String,
    credit: String,
}


#[derive(FieldSet, Clone, Debug, Serialize, Deserialize)]
#[field_set(model = PartialCorrect)]
pub struct PartialCorrectFieldSet {
    pub blank_index: u8,
    pub answer: String,
    pub credit: String,
}

impl From<PartialCorrect> for PartialCorrectFieldSet {
    fn from(value: PartialCorrect) -> Self {
        PartialCorrectFieldSet { 
            blank_index: value.blank_index, 
            answer: value.answer, 
            credit: value.credit,
        }
    }
}

impl From<PartialCorrectFieldSet> for PartialCorrect {
    fn from(value: PartialCorrectFieldSet) -> Self {
        PartialCorrect {
            blank_index: value.blank_index,
            answer: value.answer,
            credit: value.credit,
        }
    }
}