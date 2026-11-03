//! One module per example. To add one: write a component, then list it below.

use dioxus::prelude::*;

mod basics;
mod lists;
mod select;
mod sets;

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

        Example {
            title: "A set the user types into",
            note: "Tags: no order of their own and no duplicates (a BTreeSet). Type one and press Add or Enter; Edit puts one back in the box, Replace swaps it. Shown as chips by one class.",
            sets::FreeTags {},
        }

        Example {
            title: "A set chosen from a list",
            note: "Meeting days: a unit enum, so the box is a select over the week. Each day once, kept in week order.",
            sets::ChosenDays {},
        }

        Example {
            title: "A list the user types into",
            note: "Recipe steps: in the user's order, duplicates allowed (a Vec). Add a step, then move it with ↑/↓.",
            lists::FreeSteps {},
        }

        Example {
            title: "A list chosen from a list",
            note: "Trip stops: each picked from the cities, in travel order. Unlike a set, the same city can come twice.",
            lists::ChosenStops {},
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
