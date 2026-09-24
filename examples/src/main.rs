//! The formoxus example gallery — `just serve`, or `dx serve -p formoxus-examples`.
//!
//! Each example is a section in [`examples`], and adding one means writing a
//! component and listing it in `GALLERY`. They render as a real app in a real
//! browser, which is the point: the parts of formoxus that are hard to get
//! right — reactive choices, values crossing the wire — cannot be shown by SSR
//! into a string, and a `#[gtest]` that renders to HTML proves markup rather
//! than behaviour.

use dioxus::prelude::*;

mod examples;

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // One stylesheet, no framework. formoxus ships no CSS and assumes
        // none; `main.css` is plain rules against the class names it emits,
        // and doubles as the worked example of theming it with whatever you
        // already use.
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        main { class: "container",
            header {
                h1 { "formoxus examples" }
                p {
                    "Each section below is one thing formoxus can do, rendered by "
                    "the real library in a real browser. The styling is plain CSS "
                    "against the class names formoxus emits — no framework, and "
                    "none assumed."
                }
            }
            examples::Gallery {}
        }
    }
}
