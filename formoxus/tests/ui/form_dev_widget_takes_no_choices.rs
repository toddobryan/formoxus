//! A dev's widget that sets `TAKES_CHOICES = false` is refused `choices`, at
//! the key, like a built-in that takes none; before, a dev's widget took them
//! all.
use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::{AttrSet, Tag};
use formoxus::widgets::Widget;
use formoxus::widgets::WidgetProps;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Survey {
    comments: String,
}

const MOODS: &[(&str, &str)] = &[("ok", "OK")];

/// A widget of the dev's own with nothing to choose from.
#[derive(Clone, Debug, Default, PartialEq)]
struct Notes;

impl Widget for Notes {
    const NAME: &'static str = "notes";
    const PARTS: &'static [Tag] = &[];
    const EXTENDS: AttrSet = AttrSet::EMPTY;
    const TAKES_CHOICES: bool = false;
    fn render(&self, _props: WidgetProps) -> Element {
        rsx! {}
    }
}

fn main() {
    let _ = empty_form::<Survey>(form! {
        Survey {
            comments => { widget: Notes, choices: MOODS },
        }
    });
}
