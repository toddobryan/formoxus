//! A dev's widget that sets `ACCEPTS_OPTIONAL = false` is refused on an
//! `Option<_>` field, at the widget, with the generic message naming it.
use dioxus::prelude::*;
use facet::Facet;
use formoxus::attrs::AttrSet;
use formoxus::classes::Slot;
use formoxus::widgets::Widget;
use formoxus::widgets::WidgetProps;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Survey {
    comments: Option<String>,
}

/// A widget of the dev's own with no way to say "no answer".
#[derive(Clone, Debug, Default, PartialEq)]
struct Stars;

impl Widget for Stars {
    const NAME: &'static str = "stars";
    const EXTENDS: AttrSet = AttrSet::EMPTY;
    const ACCEPTS_OPTIONAL: bool = false;
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
            comments => { widget: Stars },
        }
    });
}
