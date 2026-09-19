//! [`FormState<T>`] is the plain data: structure, typed values, errors, and
//! the variant answers that live nowhere else. No Dioxus runtime, no
//! reactivity — testable on its own. [`super::Form`] is the live wrapper a
//! page actually holds.

use std::{collections::HashMap, fmt::Debug, marker::PhantomData};

use dioxus::prelude::*;
use facet::{Facet, Partial, Peek};

use crate::RenderCtx;
use crate::build::{FormMode, members_for};
use crate::buttons::ButtonSpec;
use crate::error::{FieldError, FormAccessError, FormError};
use crate::form::FormErrors;
use crate::members::{Edit, FormMember, no_such_path, owns};

use super::FormSpec;

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

        let mut partial = Partial::alloc::<T>().expect("alloc should never fail for a concrete T");
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

    pub fn push_field_error(&mut self, path: &str, message: &str) -> Result<(), FormAccessError> {
        let Some(idx) = self.members.iter().position(|m| owns(&m.name(), path)) else {
            return Err(no_such_path(path));
        };
        self.members[idx].push_field_error("", path, FieldError(message.to_string()))
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
    pub fn choose_variant(
        &mut self,
        path: &str,
        variant: Option<&str>,
    ) -> Result<(), FormAccessError> {
        self.edit(&Edit::new_choose_variant(path, variant))
    }

    pub fn buttons(&self) -> &[ButtonSpec] {
        self.spec.buttons()
    }

    pub fn render_fragment(&self, ctx: &RenderCtx) -> Element {
        rsx! {
            { self.render_title() }
            { self.render_fields(ctx) }
            { self.render_errors() }

        }
    }

    pub fn render_title(&self) -> Element {
        self.title()
            .as_ref()
            .map(|t| {
                rsx! {
                    h2 { class: "form-title", "{t}" }
                }
            })
            .unwrap_or_else(|| rsx! {})
    }

    pub fn render_fields(&self, ctx: &RenderCtx) -> Element {
        let members_rendered = self.members.iter().map(|m| m.render(ctx));
        rsx! {
            { members_rendered.into_iter() }
        }
    }

    pub fn render_errors(&self) -> Element {
        if !self.errors.is_empty() {
            rsx! {
                div {
                    class: "form-errors",
                    for e in self.errors.clone() {
                        div {
                            class: "form-error",
                            "{e.0}"
                        }
                    }
                }
            }
        } else {
            rsx! {}
        }
    }

    pub fn collect_errors(&self) -> FormErrors {
        let mut field_errors = Vec::new();
        for m in self.members.iter() {
            m.collect_errors("", &mut field_errors);
        }
        FormErrors {
            form: self.errors.clone(),
            fields: field_errors,
        }
    }

    pub fn as_hash_map(&self) -> HashMap<String, String> {
        self.leaves().into_iter().collect()
    }
}

/// Edit mode. Infallible: the value itself pins every variant, so there is
/// nothing left for a caller to choose.
pub fn form_for<T: Clone + Debug + PartialEq + Facet<'static>>(
    value: &T,
    spec: FormSpec<T>,
) -> FormState<T> {
    form_for_impl(Some(value), spec)
}

/// Create mode: no value to seed from, so every field starts empty and any
/// enum in `T` starts [`VariantChoice::Unchosen`](crate::VariantChoice) until
/// the user picks.
pub fn empty_form<T: Clone + Debug + PartialEq + Facet<'static>>(
    spec: FormSpec<T>,
) -> FormState<T> {
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
        members: members_for(
            T::SHAPE,
            value.map(Peek::new),
            mode,
            "",
            /* optional */ false,
        ),
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
