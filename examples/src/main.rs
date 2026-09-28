//! The formoxus example gallery — `just serve`, or `dx serve -p formoxus-examples`.
//!
//! Two kinds of page, for two audiences:
//!
//! - **`/` — the gallery.** Each example is a section in [`examples`], and adding
//!   one means writing a component and listing it in that module. Realistic forms
//!   showing several features at once, for someone deciding whether to use
//!   formoxus. They render as a real app in a real browser, which is the point:
//!   the parts of formoxus that are hard to get right — reactive choices, values
//!   crossing the wire — cannot be shown by SSR into a string, and a `#[gtest]`
//!   that renders to HTML proves markup rather than behaviour.
//! - **`/t/<slug>` — the test forms.** One feature each, as small as possible,
//!   for `just e2e` to drive. NOT linked from the gallery; see [`test_forms`].
//!
//! A router exists only to keep those two apart. The gallery itself is still a
//! flat list, not a set of routes.

use dioxus::prelude::*;

mod examples;
mod test_forms;

use test_forms::TestForm;

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[derive(Routable, Clone, PartialEq, Debug)]
enum Route {
    #[layout(Shell)]
    #[route("/")]
    Home {},
    /// One dynamic segment rather than a variant per form — nothing links to
    /// these, so the type-safe `Link` that variants would buy has no user, and
    /// adding a form stays a one-line change in [`test_forms`].
    #[route("/t/:slug")]
    TestForm { slug: String },
}

fn main() {
    launch(App);
}

#[component]
fn App() -> Element {
    rsx! { Router::<Route> {} }
}

/// Everything both kinds of page share: the stylesheet and the hydration marker.
#[component]
fn Shell() -> Element {
    rsx! {
        // One stylesheet, no framework. formoxus ships no CSS and assumes
        // none; `main.css` is plain rules against the class names it emits,
        // and doubles as the worked example of theming it with whatever you
        // already use.
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Outlet::<Route> {}
        HydrationMarker {}
    }
}

#[component]
fn Home() -> Element {
    rsx! {
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

/// Renders a hidden `#app-ready` element once the client has hydrated — the
/// effect only runs on the client, after mount — giving the e2e tests a
/// deterministic signal to wait for before interacting.
///
/// Without it a click can land on server-rendered markup Dioxus has not attached
/// handlers to yet, and a submit in that state triggers a native form GET instead
/// of `onsubmit`. That looks like a formoxus bug and is not one.
///
/// Compiles to nothing without `e2e-testing`, so it never ships.
#[cfg(feature = "e2e-testing")]
#[component]
fn HydrationMarker() -> Element {
    let mut hydrated = use_signal(|| false);
    use_effect(move || hydrated.set(true));
    rsx! {
        if hydrated() {
            div { id: "app-ready", hidden: true }
        }
    }
}

#[cfg(not(feature = "e2e-testing"))]
#[component]
fn HydrationMarker() -> Element {
    rsx! {}
}
