//! An item view takes `SetItemProps`; a component taking anything else is
//! reported where it is named.
use std::collections::BTreeSet;

use dioxus::prelude::*;
use facet::Facet;
use formoxus::widgets::AddItemProps;
use formoxus::{empty_form, form};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Post {
    tags: BTreeSet<String>,
}

#[component]
fn NotAView(props: AddItemProps) -> Element {
    rsx! { "{props.path}" }
}

fn main() {
    let _ = empty_form::<Post>(form! {
        Post {
            tags => { item: NotAView },
        }
    });
}
