//! A dev's set widget on a field that is not a set: the field's shape picks
//! the leaf widget's slot, and the widget is not one.
use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::{AttrSet, Tag};
use formoxus::widgets::SetWidgetProps;
use formoxus::widgets::{SetWidget, Widget};
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    city: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Chips;

impl Widget for Chips {
    const NAME: &'static str = "chips";
    const PARTS: &'static [Tag] = &[];
    const EXTENDS: AttrSet = AttrSet::EMPTY;
}

impl SetWidget for Chips {
    fn render(&self, _props: SetWidgetProps) -> Element {
        rsx! {}
    }
}

fn main() {
    let _ = empty_form::<Trip>(form! {
        Trip {
            city => { widget: Chips },
        }
    });
}
