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
use indexmap::IndexMap;
use std::{collections::HashMap, fmt::Debug, marker::PhantomData};
use dioxus::prelude::*;
use crate::reflect::widgets::ControlType;
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

    /// Add a form-level error — one that belongs to the form as a whole rather
    /// than to any field.
    ///
    /// The case this exists for is an answer that only the server has: "invalid
    /// credentials", "that username is taken". A field validator cannot produce
    /// it, because nothing local is wrong.
    ///
    /// **Cleared by the next [`validate`](Self::validate).** That is the intended
    /// lifetime, not a caveat: an error from the last round trip should not
    /// outlive the next submit. It also means a pushed error never blocks
    /// `validate`, since the clear happens first.
    ///
    /// Writing through a copy of the `Signal` for the same reason `validate`
    /// does: it keeps `&self` here, which is what lets `Form` stay `Copy` and
    /// drop into an event handler.
    pub fn push_error(&self, error: FormError) {
        let mut state = self.state;
        state.write().errors.push(error);
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
/// ```ignore
/// use_form(|| empty_form(question_form()))       // create
/// use_form(|| form_for(&question, question_form()))  // edit
/// ```
///
/// **A thunk, not a value.** Building a `FormState` means walking `T`'s facet
/// shape and allocating a `Vec<Box<dyn FormMember>>`, and `use_signal`'s
/// initializer runs exactly once — so a value argument would be constructed on
/// every render of the calling component and thrown away every time but the
/// first. Deferring it costs one `||` and means the walk happens once.
///
/// One hook rather than a create/edit pair, because the branch happens *inside*
/// the closure, which isn't a hook call, so the rules of hooks don't care:
///
/// ```ignore
/// let form = use_form(|| match existing.as_ref() {
///     Some(model) => form_for(model, question_form()),
///     None => empty_form(question_form()),
/// });
/// ```
///
/// **The state is built once and never tracked.** The closure running once looks
/// like the form follows its input, and it does not. For a model arriving from a
/// `use_server_future`, put the form in its own component that takes the model
/// as a prop — it then can't mount before the data exists — and give it a `key:`
/// so a *different* model remounts it.
pub fn use_form<T: Clone + Debug + PartialEq + Facet<'static> + 'static>(
    state: impl FnOnce() -> FormState<T>,
) -> Form<T> {
    let state = use_signal(state);
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
pub struct FormSpec<T: Clone + Debug + Facet<'static>> {
    title: Option<String>,
    fields: IndexMap<String, FieldSpec>,
    validator: Option<fn(&T) -> Vec<FormError>>,
    _type: PhantomData<T>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FieldSpec {
    pub label: Option<String>,
    pub custom_control: Option<ControlType>,
}

impl<T:  Clone + Debug + Facet<'static>> FormSpec<T> {
    pub fn new() -> Self {
        Self { title: None, fields: IndexMap::new(), validator: None, _type: PhantomData }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_custom_control(mut self, path: &str, c: ControlType) -> Self {
        self.field(path).custom_control = Some(c);
        self
    }

    pub fn with_label(mut self, path: &str, l: &str) -> Self {
        self.field(path).label = Some(l.to_string());
        self
    }

    pub fn with_validator(mut self, f: fn(&T) -> Vec<FormError>) -> Self {
        self.validator = Some(f);
        self
    }

    fn field(&mut self, path: &str) -> &mut FieldSpec {
        self.fields.entry(path.to_string()).or_default()
    }
}

impl<T: Clone + Debug + Facet<'static>> Default for FormSpec<T> {
    fn default() -> Self { Self::new() }
}

#[derive(Clone, Debug)]
pub struct FormState<T: Clone + Debug + Facet<'static>> {
    pub spec: FormSpec<T>,
    pub members: Vec<Box<dyn FormMember>>,
    pub errors: Vec<FormError>,

    pub _type: PhantomData<T>,
}

impl<T: Clone + Debug + PartialEq + Facet<'static>> FormState<T> {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.members.iter().any(|m| m.has_errors())
    }

    /// Lay the spec's per-field overrides onto the built tree.
    ///
    /// Every member is visited and asks the map about itself — unlike
    /// [`edit`](FormMember::edit), which dispatches to the single member owning a
    /// path. A spec is a set of statements, not an instruction.
    ///
    /// The prefix starts empty because each member qualifies its own name onto
    /// whatever it is handed.
    pub(crate) fn apply_specs(&mut self) {
        let fields = &self.spec.fields;
        for m in self.members.iter_mut() {
            m.apply_specs("", fields);
        }
    }

    pub fn title(&self) -> Option<String> {
        self.spec.title.clone()
    }

    /// Validate every field, build the model, then run the form-wide check.
    ///
    /// Returns `None` if anything failed, having left the reasons where they
    /// render: field errors on their own members, form-wide ones in
    /// [`errors`](Self::errors).
    ///
    /// **The spec's validator runs LAST, on the built model.** It cannot run
    /// sooner: a cross-field check is a statement about the whole value ("new and
    /// confirm must match", "the end date follows the start"), which does not
    /// exist until every field has parsed. So a form whose fields are individually
    /// fine is built, then judged, and a rejected model is discarded — the work is
    /// wasted only on the path that was going to fail anyway.
    ///
    /// Also why it cannot be a member: a member sees one subtree, and the whole
    /// point of this check is that it sees across them.
    pub fn validate(&mut self) -> Option<T> {
        self.errors.clear();
        for m in self.members.iter_mut() {
            m.validate();
        }
        if self.has_errors() {
            return None;
        }

        let mut partial =
            Partial::alloc::<T>().expect("alloc should never fail for a concrete T");
        for m in self.members.iter() {
            partial = m
                .write_into(partial)
                .expect("write_into should succeed once validate() found no errors");
        }
        let model = partial
            .build()
            .expect("build should succeed once every field was written")
            .materialize::<T>()
            .expect("materialized shape should match T — write_into wrote the wrong thing if not");

        // Copied out rather than borrowed: `Option<fn(..)>` is `Copy`, so this
        // holds no borrow of `self.spec` while `self.errors` is extended.
        if let Some(check) = self.spec.validator {
            self.errors.extend(check(&model));
        }

        // One exit for both kinds of failure, so a form-wide error cannot be
        // recorded and then returned as success.
        if self.has_errors() { None } else { Some(model) }
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
        // The owner is located before anything is mutated, so the spec can be
        // re-applied afterwards without holding a borrow of `members`.
        let Some(idx) = self.members.iter().position(|m| owns(&m.name(), path)) else {
            return Err(no_such_path(path));
        };
        self.members[idx].edit("", edit)?;

        // A structural edit can CREATE members that did not exist when the spec
        // was first applied: `AddRow` builds a fresh row from the shape alone,
        // and choosing a variant reveals that variant's fields. Without this, a
        // row added after mount renders with derived labels and controls while
        // its siblings carry the spec's — visibly inconsistent, and only for the
        // rows the user happened to add.
        //
        // Re-applying wholesale is safe because `apply_specs` is idempotent, and
        // that is precisely why `Edit` is NOT a variant of the spec: an edit
        // replayed twice would add two rows.
        self.apply_specs();
        Ok(())
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
        let title = self.title().as_ref().map(|t| {
            rsx! {
                h2 { class: "form-title", "{t}" }
            }
        });
        let members_rendered = self.members.iter().map(|m| m.render(ctx));
        let errors: Element = if !self.errors.is_empty() {
            rsx! {
                ul {
                    class: "form-errors",
                    for e in self.errors.clone() {
                        li {
                            class: "form-error",
                            "{e.0}"
                        }
                    }
                }
            } 
        } else {
            rsx! {}
        };
        
        rsx! {
            { title }
            { members_rendered.into_iter() }
            { errors }
            
        }
    }
}

/// Edit mode. Infallible: the value itself pins every variant, so there is
/// nothing left for a caller to choose.
pub fn form_for<T: Clone + Debug + PartialEq + Facet<'static>>(value: &T, spec: FormSpec<T>) -> FormState<T> {
    form_for_impl(Some(value), spec)
}

/// Create mode with no choices supplied — fails with [`MissingVariants`] if `T`
/// contains any enum at all.
pub fn empty_form<T: Clone + Debug + PartialEq + Facet<'static>>(spec: FormSpec<T>)
-> FormState<T> {
    form_for_impl(None, spec)
}

fn form_for_impl<T: Clone + Debug + PartialEq + Facet<'static>>(
    value: Option<&T>,
    spec: FormSpec<T>,
) -> FormState<T> {
    // The mode is fixed HERE, once, by which constructor the caller reached for —
    // and threaded down untouched. Deriving it further down from whether some
    // peek happens to be present is the bug `FormMode`'s docs describe.
    let mode = match value {
        Some(_) => FormMode::Populated,
        None => FormMode::Blank,
    };

    let mut state = FormState {
        spec,
        members: members_for(T::SHAPE, value.map(Peek::new), mode, "", /* optional */ false),
        errors: Vec::new(),
        _type: PhantomData,
    };
    // The walk builds the tree from the SHAPE alone; the spec's per-field
    // overrides are laid on afterwards. Separating the two is what lets one spec
    // serve both constructors, and what keeps the walk from taking a seventh
    // parameter.
    state.apply_specs();
    state
}
