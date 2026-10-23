//! A prop of the wrong type is reported at its value, which goes through
//! `Into::into`. A `bool` prop, because rustc lists every `From` impl of the
//! target, and `u32`'s list names types from dependencies.
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

/// A widget of the dev's own, with one prop.
#[derive(Clone, Debug, Default, PartialEq)]
struct Notes {
    wrap: bool,
}

impl Widget for Notes {
    const NAME: &'static str = "notes";
    const PARTS: &'static [Tag] = &[];
    const EXTENDS: AttrSet = AttrSet::EMPTY;
    fn render(&self, _props: WidgetProps) -> Element {
        rsx! {}
    }
}

fn main() {
    let _ = empty_form::<Survey>(form! {
        Survey {
            comments => { widget: Notes { wrap: "yes" } },
        }
    });
}
