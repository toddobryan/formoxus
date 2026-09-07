//! The test suite, one module per concern. These live inside the crate rather
//! than in `tests/` because several reach crate-private items.

use dioxus::prelude::*;

pub mod models;

mod empty_strings;
mod enums;
mod forms;
mod optional_containers;
mod vecs;
mod widgets;
mod roundtrip;

/// Render a component to HTML, with a real Dioxus runtime behind it.
///
/// Every render assertion has to come through here now. `FormState::render` takes a
/// `ValuesByPath`, which is a `Store`, which only `use_store` can mint — and
/// that's a hook, so it needs a live runtime. Rendering stopped being a pure
/// function of the form when the values moved into a store.
pub fn render_to_html(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}
