//! [`FormState<T>`] is the plain data: structure, typed values, errors, and
//! the variant answers that live nowhere else. No Dioxus runtime, no
//! reactivity — testable on its own. [`super::Form`] is the live wrapper a
//! page actually holds.

use std::collections::HashMap;
use std::{fmt::Debug, marker::PhantomData};

use dioxus::prelude::*;
use facet::{Facet, Partial, Peek};

use crate::ValidationError;
use crate::build::{FormMode, members_for};
use crate::buttons::ButtonSpec;
use crate::error::{FormAccessError, ValidationMessage};
use crate::form::{ErrorsByPath, FormErrors};
use crate::label_case::LabelCase;
use crate::members::RenderCtx;
use crate::members::{Edit, FormMember, ValuesByPath, no_such_path, owns};

use super::FormSpec;

#[derive(Clone, Debug)]
pub struct FormState<T: Clone + Debug + Facet<'static>> {
    pub spec: FormSpec<T>,
    pub members: Vec<Box<dyn FormMember>>,
    pub errors: Vec<ValidationMessage>,

    pub _type: PhantomData<T>,
}

impl<T: Clone + Debug + PartialEq + Facet<'static>> FormState<T> {
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.members.iter().any(|m| m.has_errors_within())
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
        for m in &mut self.members {
            m.distribute_specs("", fields);
        }
    }

    /// What this form *stated* about label casing, if anything.
    ///
    /// `None` means "not stated", not "Title" — resolving it needs the
    /// app-level tier, which lives in Dioxus context and so needs a runtime.
    /// A `FormState` rebuilt on the server by [`crate::Submission`] has none,
    /// which is why the resolution is [`crate::Form::label_case`] and not here.
    pub fn label_case(&self) -> Option<LabelCase> {
        self.spec.label_case
    }

    /// What this form *stated* about the browser's own validation, if
    /// anything. `None` means "not stated" — see [`Self::label_case`] for why
    /// resolving it cannot happen here.
    pub fn use_browser_validation(&self) -> Option<bool> {
        self.spec.use_browser_validation
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
        for m in &mut self.members {
            m.validate();
        }
        if self.has_errors() {
            return None;
        }

        let mut partial = Partial::alloc::<T>().expect("alloc should never fail for a concrete T");
        for m in &self.members {
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
            let mut errors_by_path = ErrorsByPath::default();
            for ValidationError { path, message } in check(&model) {
                match path {
                    None => self.errors.push(message),
                    Some(p) => errors_by_path
                        .entry(p.to_string())
                        .or_default()
                        .push(message),
                }
            }
            let unclaimed = self.distribute_errors(errors_by_path);
            let messages_for_unclaimed: Vec<ValidationMessage> = unclaimed
                .into_iter()
                .map(|(path, messages)| {
                    let combined_messages = messages
                        .into_iter()
                        .map(|vm| vm.0)
                        .collect::<Vec<_>>()
                        .join("; ");
                    ValidationMessage(format!("At path: {path}, {combined_messages}"))
                })
                .collect();
            self.errors.extend(messages_for_unclaimed);
        }

        // One exit for both kinds of failure, so a form-wide error cannot be
        // recorded and then returned as success.
        if self.has_errors() { None } else { Some(model) }
    }

    /// Every leaf input in the form, keyed by qualified path, in the order the
    /// member tree declares them — the inverse of
    /// [`distribute_values`](Self::distribute_values), and the list the widget
    /// layer turns into one store entry apiece.
    pub fn collect_values(&self) -> ValuesByPath {
        let mut out = Vec::new();
        for m in &self.members {
            m.collect_values("", &mut out);
        }
        out.into_iter().collect()
    }

    /// A [`ValuesStore`](crate::ValuesStore)'s contents as a [`ValuesByPath`],
    /// ordered by the member tree rather than by the map's hashing.
    ///
    /// The bridge every read of the live values crosses, because the store's map
    /// and [`ValuesByPath`] cannot be the same type — see
    /// [`ValuesStore`](crate::ValuesStore). Takes its KEYS and their order from
    /// [`collect_values`](Self::collect_values) and its VALUES from the store, which is the only
    /// way to get both: the schema knows the order, the store knows what the
    /// user typed.
    ///
    /// Two things follow from taking the keys from the schema. A path the store
    /// holds but the schema does not is dropped — those are the stale keys an
    /// undone variant choice leaves behind, which nothing reads. And a path the
    /// schema has but the store does not arrives as `""`, which is the same
    /// "absent IS empty" rule [`distribute_values`](Self::distribute_values) already follows.
    pub(crate) fn ordered_values(&self, store: &HashMap<String, String>) -> ValuesByPath {
        self.collect_values()
            .into_keys()
            .map(|path| {
                let raw = store.get(&path).cloned().unwrap_or_default();
                (path, raw)
            })
            .collect()
    }

    /// Take raw input values back in, keyed by the same qualified paths
    /// [`collect_values`](Self::collect_values) hands out. Call this on submit, before
    /// `validate()`.
    pub fn distribute_values(&mut self, values: &ValuesByPath) {
        for m in &mut self.members {
            m.distribute_values("", values);
        }
    }

    pub fn distribute_errors(&mut self, mut errors: ErrorsByPath) -> ErrorsByPath {
        for m in &mut self.members {
            m.distribute_errors("", &mut errors);
        }
        errors
    }

    /// Take values straight off a submitted `<form>`. Dioxus's
    /// `FormData::values()` hands back `(name, FormValue)` pairs keyed by each
    /// input's `name` attribute — which is exactly the qualified path
    /// [`collect_values`](Self::collect_values) emitted — so no per-field signal is needed to
    /// track edits: the DOM already did it.
    pub fn distribute_form_values(&mut self, values: &[(String, String)]) {
        let map: ValuesByPath = values.iter().cloned().collect();
        self.distribute_values(&map);
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
        // row added after mount renders with derived labels and widgets while
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
        let mut this_error = ErrorsByPath::default();
        this_error.insert(path.to_string(), vec![ValidationMessage::from(message)]);
        let unclaimed = self.distribute_errors(this_error);
        if unclaimed.is_empty() {
            Ok(())
        } else {
            Err(no_such_path(path))
        }
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
    /// unwinds aren't caught there. The `Result` is the transport; the input
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
        self.title().as_ref().map_or_else(
            || rsx! {},
            |t| {
                rsx! {
                    h2 { class: "fx-form-title", "{t}" }
                }
            },
        )
    }

    pub fn render_fields(&self, ctx: &RenderCtx) -> Element {
        let members_rendered = self.members.iter().map(|m| m.render(ctx));
        rsx! {
            { members_rendered.into_iter() }
        }
    }

    pub fn render_errors(&self) -> Element {
        if self.errors.is_empty() {
            rsx! {}
        } else {
            rsx! {
                ul {
                    class: "fx-form-errors",
                    for e in self.errors.clone() {
                        li {
                            class: "fx-form-error",
                            "{e.0}"
                        }
                    }
                }
            }
        }
    }

    pub fn collect_errors(&self) -> FormErrors {
        let mut field_errors = ErrorsByPath::default();
        for m in &self.members {
            m.collect_errors("", &mut field_errors);
        }
        FormErrors {
            form: self.errors.clone(),
            fields: field_errors,
        }
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
/// enum in `T` starts [`VariantChoice::Unchosen`](crate::members::VariantChoice) until
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
