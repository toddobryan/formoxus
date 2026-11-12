//! A dev's widget that renders one value, on a set: the field's shape picks
//! the set widget's slot, and the widget is not one.
use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::{AttrSet, Tag};
use formoxus::widgets::WidgetProps;
use formoxus::widgets::{LeafWidget, Widget};
use formoxus::{empty_form, form};
use std::collections::BTreeSet;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Trip {
    cities: BTreeSet<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Notes;

impl Widget for Notes {
    const NAME: &'static str = "notes";
    const PARTS: &'static [Tag] = &[];
    const EXTENDS: AttrSet = AttrSet::EMPTY;
}

impl LeafWidget for Notes {
    fn render(&self, _props: WidgetProps) -> Element {
        rsx! {}
    }
}

fn main() {
    let _ = empty_form::<Trip>(form! {
        Trip {
            cities => { widget: Notes },
        }
    });
}
