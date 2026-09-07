//! `Form<T>` and the three public constructors — one per mode, so the illegal
//! combination (a value AND variant choices) cannot be written.

use facet::{Facet, Partial, Peek};
use std::{collections::HashMap, fmt::Debug, marker::PhantomData};
use dioxus::prelude::*;
use crate::reflect::{RenderCtx, ValuesByPath};
use crate::reflect::build::{FormMode, members_for};
use crate::error::{FormAccessError, FormError};
use crate::reflect::members::{Edit, FormMember, no_such_path, owns};

pub fn use_form_values<T>(form: &Form<T>) -> ValuesByPath
where
    T: Clone + Debug + PartialEq + Facet<'static>,
{
    let leaves = form.leaves();
    use_store(move || leaves.into_iter().collect())
}

#[derive(Clone, Debug)]
pub struct Form<T: Clone + Debug + Facet<'static>> {
    pub title: Option<String>,
    pub members: Vec<Box<dyn FormMember>>,
    pub errors: Vec<FormError>,

    pub _type: PhantomData<T>,
}

impl<T: Clone + Debug + PartialEq + Facet<'static>> Form<T> {
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

    pub fn render(&self, values: ValuesByPath) -> Element {
        let title = self.title.as_ref().map(|t| {
            rsx! {
                h2 { class: "form-title", "{t}" }
            }
        });
           
        let ctx = RenderCtx::root(values);
        let members_rendered = self.members.iter().map(|m| m.render(&ctx));
        
        rsx! {
            { title }
            { members_rendered.into_iter() }
        }
    }
}

/// Edit mode. Infallible: the value itself pins every variant, so there is
/// nothing left for a caller to choose.
pub fn form_for<T: Clone + Debug + PartialEq + Facet<'static>>(value: &T) -> Form<T> {
    form_for_impl(Some(value))
}

/// Create mode with no choices supplied — fails with [`MissingVariants`] if `T`
/// contains any enum at all.
pub fn empty_form<T: Clone + Debug + PartialEq + Facet<'static>>()
-> Form<T> {
    form_for_impl(None)
}

fn form_for_impl<T: Clone + Debug + PartialEq + Facet<'static>>(
    value: Option<&T>,
) -> Form<T> {
    // The mode is fixed HERE, once, by which constructor the caller reached for —
    // and threaded down untouched. Deriving it further down from whether some
    // peek happens to be present is the bug `FormMode`'s docs describe.
    let mode = match value {
        Some(_) => FormMode::Populated,
        None => FormMode::Blank,
    };

    Form {
        title: None,
        members: members_for(T::SHAPE, value.map(Peek::new), mode, ""),
        errors: Vec::new(),
        _type: PhantomData,
    }
}
