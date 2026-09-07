//! The test suite, one module per concern. These live inside the crate rather
//! than in `tests/` because several reach crate-private items.

use crate::reflect::{RenderCtx, ValuesByPath};
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
/// Every render assertion has to come through here now. `Form::render` takes a
/// `ValuesByPath`, which is a `Store`, which only `use_store` can mint — and
/// that's a hook, so it needs a live runtime. Rendering stopped being a pure
/// function of the form when the values moved into a store.
pub fn render_to_html(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

/// A root [`RenderCtx`] for tests that only inspect rendered markup.
///
/// The callback is a deliberate no-op, and a **silent** one — which is worth
/// knowing, because it's indistinguishable from a working edit transport right
/// up until a test drives the `<select>` and passes while achieving nothing.
///
/// Making it panic instead does NOT help: dioxus catches panics raised during a
/// component's render (that's what `CapturedPanic` is), so the panic never
/// reaches the test. Verified, not assumed.
///
/// The first test that actually fires an edit wants a callback that records
/// what it received, and to assert on that — not this.
pub fn markup_ctx(values: ValuesByPath) -> RenderCtx {
    RenderCtx::root(values, Callback::default())
}
