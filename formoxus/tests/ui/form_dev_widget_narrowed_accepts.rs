//! A dev's widget is checked by its own consts, like a built-in: one that
//! narrows `ACCEPTS` to `Text` is refused on a number field, at the widget,
//! with the generic message naming it.
use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::{AttrSet, FieldType, FieldTypeSet};
use formoxus::classes::Slot;
use formoxus::widgets::Widget;
use formoxus::widgets::WidgetProps;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Survey {
    score: u32,
}

/// A widget of the dev's own that renders only text.
#[derive(Clone, Debug, Default, PartialEq)]
struct Notes;

impl Widget for Notes {
    const NAME: &'static str = "notes";
    const EXTENDS: AttrSet = AttrSet::EMPTY;
    const ACCEPTS: FieldTypeSet = FieldTypeSet::EMPTY.with(FieldType::Text);
    const TAKES_CHOICES: bool = false;
    fn render(&self, _props: WidgetProps) -> Element {
        rsx! {}
    }
    fn default_classes() -> Vec<(&'static str, Slot)> {
        Vec::new()
    }
}

fn main() {
    let _ = empty_form::<Survey>(form! {
        Survey {
            score => { widget: Notes },
        }
    });
}
