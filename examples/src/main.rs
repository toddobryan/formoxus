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
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // Pico, because formoxus's markup is written against its conventions —
        // `aria-invalid` on an errored control, `small` for the error list under
        // a field. Everything formoxus emits carries its own class names too, so
        // any other framework styles it like ordinary markup.
        document::Link {
            rel: "stylesheet",
            href: "https://cdn.jsdelivr.net/npm/@picocss/pico@2/css/pico.min.css",
        }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        main { class: "container",
            header {
                h1 { "formoxus examples" }
                p {
                    "Each section below is one thing formoxus can do, rendered by "
                    "the real library in a real browser."
                }
            }
            examples::Gallery {}
        }
    }
}
