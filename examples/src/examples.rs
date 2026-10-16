//! One module per example. To add one: write a component, then list it below.

use dioxus::prelude::*;

mod basics;
mod select;

/// Every example, in the order they appear on the page.
///
/// A plain list rather than a router: an example is a few dozen lines and the
/// whole value is seeing several at once, so paging between them would cost
/// more than it saves. Add a route later if the page ever gets long.
#[component]
pub fn Gallery() -> Element {
    rsx! {
        Example {
            title: "A basic form",
            note: "Scalars, a widget override, and buttons — the parts that already work.",
            basics::Basics {},
        }

        Example {
            title: "An address form",
            note: "Select for state (which I hate), and some formatting",
            select::AddressForm {},
        }
    }
}

/// The frame every example sits in, so they line up without each one repeating
/// the markup.
#[component]
fn Example(title: String, note: String, children: Element) -> Element {
    rsx! {
        section { class: "example",
            h2 { "{title}" }
            p { class: "note", "{note}" }
            {children}
        }
    }
}
