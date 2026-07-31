use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

pub trait Form {
    type Model;

    fn render(&self) -> Element;

    fn validate(self) -> Self;
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Phase {
    #[default]
    Initial,
    Edited,
    Submitted,
}
