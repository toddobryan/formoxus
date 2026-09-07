//! The two layers of a form.
//!
//! [`FormState<T>`] is the plain data: structure, typed values, errors, and the
//! variant answers that live nowhere else. No Dioxus runtime, no reactivity —
//! testable on its own.
//!
//! [`Form<T>`] is what a page holds: a `Signal` of that state, the live value
//! store beside it, and the callback a structural edit travels on. Built by
//! [`use_form`].

use facet::{Facet, Partial, Peek};
use std::{collections::HashMap, fmt::Debug, marker::PhantomData};
use dioxus::prelude::*;
use crate::reflect::{RenderCtx, ValuesByPath};
use crate::reflect::build::{FormMode, members_for};
use crate::error::{FormAccessError, FormError};
use crate::reflect::members::{Edit, FormMember, no_such_path, owns};

/// The value store on its own, seeded from a state's leaves.
///
/// The low-level half of [`use_form`], which is what a page normally wants.
pub fn use_form_values<T>(form: &FormState<T>) -> ValuesByPath
where
    T: Clone + Debug + PartialEq + Facet<'static>,
{
    let leaves = form.leaves();
    use_store(move || leaves.into_iter().collect())
}

/// A live form: the state, the values beside it, and where a structural edit
/// goes.
///
/// `Copy`, so it drops into event handlers without ceremony.
pub struct Form<T: Clone + Debug + PartialEq + Facet<'static> + 'static> {
    /// Structure, typed values, errors — and *answers*. A fieldless enum
    /// variant contributes nothing to `leaves()`, so a variant choice lives
    /// nowhere but here. That's why it's `FormState` and not `FormSchema`, and
    /// why it's a `Signal`: a structural edit rebuilds it and the page has to
    /// re-render.
    state: Signal<FormState<T>>,
    /// Live raw values by qualified path. Changes on every keystroke, and
    /// survives a schema rebuild because paths are stable under one.
    values: ValuesByPath,
    /// Where a structural edit goes. Closes over `state`, which is what lets the
    /// widget layer stay ignorant of `T` — [`RenderCtx`] can't be generic
    /// without making every member type generic too.
    on_edit: Callback<Edit>,
}

// Hand-written, not derived: `#[derive(Copy)]` would add a spurious `T: Copy`
// bound — the same trap `crate::fields` documents for `Default` — even though
// `Signal`, `Store` and `Callback` are each `Copy` regardless of what they wrap.
impl<T: Clone + Debug + PartialEq + Facet<'static> + 'static> Clone for Form<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Clone + Debug + PartialEq + Facet<'static> + 'static> Copy for Form<T> {}

impl<T: Clone + Debug + PartialEq + Facet<'static> + 'static> Form<T> {
    /// The live value store, keyed by qualified path.
    pub fn values(&self) -> ValuesByPath {
        self.values
    }

    /// The state as it currently stands. Reading this subscribes the caller to
    /// *structural* changes.
    pub fn state(&self) -> ReadSignal<FormState<T>> {
        self.state.into()
    }

    pub fn render(&self) -> Element {
        self.state
            .read()
            .render(&RenderCtx::root(self.values, self.on_edit))
    }

    /// Push the live values into the state, then build the model.
    ///
    /// Lives here because this is the only place that knows both halves — a
    /// caller never has to remember that `apply` comes first.
    pub fn validate(&self) -> Option<T> {
        // `Signal` is a `Copy` handle to shared reactive state, so copy it for a
        // mutable binding and write through the copy — same idiom as
        // `FormStoreExt::validate` on the derive path. Keeps `&self` here, which
        // is what lets `Form` stay `Copy` and drop into event handlers.
        let mut state = self.state;
        let mut state = state.write();
        state.apply(&self.values.read().clone());
        state.validate()
    }
}

/// Make a [`FormState`] live.
///
///     use_form(empty_form::<Question>())   // create
///     use_form(form_for(&question))        // edit
///
/// One hook rather than a create/edit pair, because taking the state as a
/// *value* lets one component serve both modes — the branch happens while
/// building the argument, which isn't a hook call, so the rules of hooks don't
/// care:
///
///     let form = use_form(match existing.as_ref() {
///         Some(model) => form_for(model),
///         None => empty_form::<Question>(),
///     });
///
/// **The argument is rebuilt on every render and dropped after the first**,
/// since `use_signal`'s initializer runs once. How often that is depends on what
/// the calling component reads: reading only the state (via [`Form::render`])
/// means structural edits alone, which are rare; reading the whole value map
/// subscribes deeply and so means every keystroke.
///
/// **The state is read once and never tracked.** Passing a value looks like the
/// form follows it, and it does not. For a model arriving from a
/// `use_server_future`, put the form in its own component that takes the model
/// as a prop — it then can't mount before the data exists — and give it a `key:`
/// so a *different* model remounts it.
pub fn use_form<T: Clone + Debug + PartialEq + Facet<'static> + 'static>(
    state: FormState<T>,
) -> Form<T> {
    let state = use_signal(|| state);
    // `peek`, not `read`: seeding the store must not subscribe this component to
    // the state, or every structural edit would re-run the initializer's scope
    // for nothing.
    let values = use_store(move || state.peek().leaves().into_iter().collect());
    let on_edit = use_callback(move |edit: Edit| {
        let mut state = state;
        if let Err(e) = state.write().edit(&edit) {
            // An unknown variant name has already been recorded against the
            // `VariantSet`, so it renders beside the select. Anything else is a
            // bug or a forged event — log rather than panic, since on wasm a
            // panic aborts instead of reaching an `ErrorBoundary`.
            warn!("rejected form edit: {e}");
        }
    });
    Form { state, values, on_edit }
}

#[derive(Clone, Debug)]
pub struct FormState<T: Clone + Debug + Facet<'static>> {
    pub title: Option<String>,
    pub members: Vec<Box<dyn FormMember>>,
    pub errors: Vec<FormError>,

    pub _type: PhantomData<T>,
}

impl<T: Clone + Debug + PartialEq + Facet<'static>> FormState<T> {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.members.iter().any(|m| m.has_errors())
    }

    pub fn validate(&mut self) -> Option<T> {
        self.errors.clear();
        for m in self.members.iter_mut() {
            m.validate();
        }
        if self.has_errors() {
            None
        } else {
            let mut partial =
                Partial::alloc::<T>().expect("alloc should never fail for a concrete T");
            for m in self.members.iter() {
                partial = m
                    .write_into(partial)
                    .expect("write_into should succeed once validate() found no errors");
            }
            Some(
                partial
                    .build()
                    .expect("build should succeed once every field was written")
                    .materialize::<T>()
                    .expect("materialized shape should match T — write_into wrote the wrong thing if not"),
            )
        }
    }

    /// Every leaf input in the form, as `(qualified_path, raw_value)` — the
    /// list the widget layer turns into one signal apiece.
    pub fn leaves(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for m in self.members.iter() {
            m.collect_leaves("", &mut out);
        }
        out
    }

    /// Take raw widget values back in, keyed by the same qualified paths
    /// [`leaves`](Self::leaves) hands out. Call this on submit, before
    /// `validate()`.
    pub fn apply(&mut self, values: &HashMap<String, String>) {
        for m in self.members.iter_mut() {
            m.apply_leaves("", values);
        }
    }

    /// Take values straight off a submitted `<form>`. Dioxus's
    /// `FormData::values()` hands back `(name, FormValue)` pairs keyed by each
    /// input's `name` attribute — which is exactly the qualified path
    /// [`leaves`](Self::leaves) emitted — so no per-field signal is needed to
    /// track edits: the DOM already did it.
    pub fn apply_form_values(&mut self, values: &[(String, String)]) {
        let map: HashMap<String, String> = values.iter().cloned().collect();
        self.apply(&map);
    }

    pub fn edit(&mut self, edit: &Edit) -> Result<(), FormAccessError> {
        let path = edit.path();
        for m in self.members.iter_mut() {
            if owns(&m.name(), path) {
                return m.edit("", edit);
            }
        }
        Err(no_such_path(path))
    }

    /// Answer the enum at `path`, rebuilding that subtree from the chosen
    /// variant's fields. The schema-rebuild a reactive `<select>` triggers;
    /// add/remove-row will be its sibling.
    ///
    /// **Both failure modes are caller bugs, not user input.** The path comes
    /// from our own member tree and the variant name from an `<option>` we
    /// generated, so neither crosses the wire as a structural instruction. It
    /// still returns `Result` rather than panicking, because on wasm a panic
    /// aborts instead of reaching an `ErrorBoundary` — dioxus-core notes that
    /// unwinds aren't caught there. The `Result` is the transport; the widget
    /// layer is expected to push it into the boundary rather than recover.
    pub fn choose_variant(&mut self, path: &str, variant: Option<&str>) -> Result<(), FormAccessError> {
        self.edit(&Edit::new_choose_variant(path, variant))
    }

    pub fn render(&self, ctx: &RenderCtx) -> Element {
        let title = self.title.as_ref().map(|t| {
            rsx! {
                h2 { class: "form-title", "{t}" }
            }
        });
        let members_rendered = self.members.iter().map(|m| m.render(ctx));
        
        rsx! {
            { title }
            { members_rendered.into_iter() }
        }
    }
}

/// Edit mode. Infallible: the value itself pins every variant, so there is
/// nothing left for a caller to choose.
pub fn form_for<T: Clone + Debug + PartialEq + Facet<'static>>(value: &T) -> FormState<T> {
    form_for_impl(Some(value))
}

/// Create mode with no choices supplied — fails with [`MissingVariants`] if `T`
/// contains any enum at all.
pub fn empty_form<T: Clone + Debug + PartialEq + Facet<'static>>()
-> FormState<T> {
    form_for_impl(None)
}

fn form_for_impl<T: Clone + Debug + PartialEq + Facet<'static>>(
    value: Option<&T>,
) -> FormState<T> {
    // The mode is fixed HERE, once, by which constructor the caller reached for —
    // and threaded down untouched. Deriving it further down from whether some
    // peek happens to be present is the bug `FormMode`'s docs describe.
    let mode = match value {
        Some(_) => FormMode::Populated,
        None => FormMode::Blank,
    };

    FormState {
        title: None,
        members: members_for(T::SHAPE, value.map(Peek::new), mode, ""),
        errors: Vec::new(),
        _type: PhantomData,
    }
}
