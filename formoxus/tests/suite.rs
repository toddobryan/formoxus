//! The test suite, one module per concern.
//!
//! One integration target rather than one file per concern, which cargo would
//! turn into a dozen separate binaries: the modules share `render_to_html`, the
//! `Harness`, and the models in [`models`], and a dozen binaries would each
//! link dioxus to compile the same helpers again.
//!
//! Everything here is written from a *consumer's* position — `formoxus::`, not
//! `crate::` — which is the point of it living in `tests/`: it exercises the
//! same public surface a dependent crate gets, so an item that is unreachable
//! or un-nameable from outside fails here rather than passing quietly.
//!
//! Every `mod` needs `#[path]`. This file is the test binary's crate ROOT, and
//! a root resolves `mod foo;` to `foo.rs` beside itself — `tests/foo.rs`, which
//! cargo would then also auto-discover as a target of its own. The submodules
//! live in `tests/suite/` to stay out of that way, which costs one attribute
//! each.

use dioxus::prelude::*;

#[path = "suite/models.rs"]
pub mod models;

#[path = "suite/buttons.rs"]
mod buttons;
#[path = "suite/empty_strings.rs"]
mod empty_strings;
#[path = "suite/enums.rs"]
mod enums;
#[path = "suite/errors.rs"]
mod errors;
#[path = "suite/form_macro.rs"]
mod form_macro;
#[path = "suite/forms.rs"]
mod forms;
#[path = "suite/into_slot.rs"]
mod into_slot;
#[path = "suite/label_case.rs"]
mod label_case;
#[path = "suite/newtypes.rs"]
mod newtypes;
#[path = "suite/optional_containers.rs"]
mod optional_containers;
#[path = "suite/roundtrip.rs"]
mod roundtrip;
#[path = "suite/specs.rs"]
mod specs;
#[path = "suite/submissions.rs"]
mod submissions;
#[path = "suite/vecs.rs"]
mod vecs;
#[path = "suite/widgets.rs"]
mod widgets;

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

// ── Driving a real interaction ───────────────────────────────────────────
//
// Rendering to HTML proves what a form *looks* like. These prove what it
// *does*: a DOM event goes in, an `Edit` travels through `use_form`'s callback
// into `FormState`, the schema rebuilds, and the new subtree comes back out.

use dioxus::core::{ElementId, Mutation, Mutations};
use dioxus_html::{
    PlatformEventData, SerializedFormData, SerializedHtmlEventConverter, SerializedMouseData,
    set_event_converter,
};
use std::any::Any;
use std::rc::Rc;

/// A mounted app you can fire events at more than once.
///
/// The reason this is a struct and not a function: a structural edit rebuilds
/// part of the tree, so the second event's target may not be the element the
/// *first* rebuild registered. Listener ids are therefore accumulated across
/// every render rather than read once — `Harness` tracks registrations and
/// removals so `listeners()` always describes the tree as it stands now.
pub struct Harness {
    dom: VirtualDom,
    listeners: Vec<(String, ElementId)>,
}

impl Harness {
    pub fn mount(app: fn() -> Element) -> Self {
        // A platform (web, desktop) normally installs this; a bare `VirtualDom`
        // has none, and without it a listener's `PlatformEventData -> FormData`
        // conversion has nothing to convert with. Idempotent, so every mount
        // can call it.
        set_event_converter(Box::new(SerializedHtmlEventConverter));
        let mut dom = VirtualDom::new(app);
        let mutations = dom.rebuild_to_vec();
        let mut harness = Harness {
            dom,
            listeners: Vec::new(),
        };
        harness.absorb(&mutations);
        harness
    }

    fn absorb(&mut self, mutations: &Mutations) {
        for edit in mutations.edits.iter() {
            match edit {
                Mutation::NewEventListener { name, id } => {
                    self.listeners.push((name.to_string(), *id))
                }
                Mutation::RemoveEventListener { name, id } => {
                    self.listeners.retain(|(n, i)| !(n == name && i == id))
                }
                // A torn-down element takes its listeners with it, and dioxus
                // recycles the id — so dropping these matters for correctness,
                // not tidiness: a stale entry could name an element that is now
                // something else entirely.
                Mutation::Remove { id } => self.listeners.retain(|(_, i)| i != id),
                _ => {}
            }
        }
    }

    /// Every element currently listening for `event`, in registration order —
    /// which for a fresh subtree is document order.
    pub fn listeners(&self, event: &str) -> Vec<ElementId> {
        self.listeners
            .iter()
            .filter(|(name, _)| name == event)
            .map(|(_, id)| *id)
            .collect()
    }

    /// The sole listener for `event`, asserting there is exactly one. Keeps a
    /// test from silently firing at the wrong control when the markup grows.
    pub fn only_listener(&self, event: &str) -> ElementId {
        let found = self.listeners(event);
        assert_eq!(
            found.len(),
            1,
            "expected exactly one `{event}` listener, found {}",
            found.len()
        );
        found[0]
    }

    /// Fire `event` at `id` with `value` as the control's value, then flush the
    /// re-render so the next assertion sees the result. Returns how many DOM
    /// edits that re-render produced — see
    /// [`switching_variants_actually_edits_the_dom`] for why the count matters.
    ///
    /// Listeners register against `PlatformEventData`, not `FormData` — the
    /// event attribute macro does that conversion itself, inside the handler.
    /// Handing it a `FormData` directly fails the downcast at dispatch.
    pub fn fire(&mut self, event: &str, id: ElementId, value: &str) -> usize {
        let payload = PlatformEventData::new(Box::new(SerializedFormData::new(
            value.to_string(),
            Vec::new(),
        )));
        let dom_event: Event<dyn Any> = Event::new(Rc::new(payload), true);
        self.dom.runtime().handle_event(event, dom_event, id);
        let mutations = self.dom.render_immediate_to_vec();
        self.absorb(&mutations);
        mutations.edits.len()
    }

    /// Click the element at `id`, then flush. Returns the edit count, like
    /// [`fire`](Self::fire).
    ///
    /// Separate from `fire` because the payload type has to match what the
    /// handler will convert to: a listener registers against `PlatformEventData`
    /// and the attribute macro downcasts inside, so handing an `onclick` a
    /// `SerializedFormData` fails the downcast at dispatch rather than at
    /// compile time.
    pub fn click(&mut self, id: ElementId) -> usize {
        let payload = PlatformEventData::new(Box::new(SerializedMouseData::default()));
        let dom_event: Event<dyn Any> = Event::new(Rc::new(payload), true);
        self.dom.runtime().handle_event("click", dom_event, id);
        let mutations = self.dom.render_immediate_to_vec();
        self.absorb(&mutations);
        mutations.edits.len()
    }

    pub fn html(&self) -> String {
        dioxus_ssr::render(&self.dom)
    }
}

/// The listeners in `after` that weren't in `before`.
///
/// How a test identifies a control that an edit just revealed. Position won't
/// do it: registration order is the order dioxus creates dynamic nodes, not
/// document order, so "the second `change` listener" is not "the nested select."
pub fn new_since(before: &[ElementId], after: Vec<ElementId>) -> Vec<ElementId> {
    after
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect()
}
