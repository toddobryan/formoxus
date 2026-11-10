//! [`FormState<T>`] is the plain data: structure, typed values, errors, and
//! the variant answers that live nowhere else. No Dioxus runtime, no
//! reactivity — testable on its own. [`super::Form`] is the live wrapper a
//! page actually holds.

use std::collections::{HashMap, HashSet};
use std::{fmt::Debug, marker::PhantomData};

use dioxus::prelude::*;
use facet::{Facet, Partial, Peek};
use formoxus_spec::FixedKey;

use crate::build::{FormMode, Walk, nodes_for};
use crate::buttons::ButtonSpec;
use crate::error::{FormAccessError, ValidationMessage};
use crate::form::{ErrorsByPath, FormErrors};
use crate::label_case::LabelCase;
use crate::nodes::{Drafts, Edit, Node, ValuesByPath, no_such_path};
use crate::nodes::{NodeContext, RawValue};
use crate::path::{ModelPath, NodePath};
use crate::{classes::Classes, error::ValidationError};

use super::FormSpec;

#[derive(Clone, Debug)]
pub struct FormState<T: Clone + Debug + Facet<'static>> {
    pub spec: FormSpec<T>,
    pub nodes: Vec<Box<dyn Node>>,
    pub errors: Vec<ValidationMessage>,
    pub field_errors: ErrorsByPath,

    pub _type: PhantomData<T>,
}

impl<T: Clone + Debug + PartialEq + Facet<'static>> FormState<T> {
    pub fn has_errors(&self) -> bool {
        let errors = self.collect_errors();
        !errors.form.is_empty() || !errors.fields.is_empty()
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

    /// What this form *stated* about checking fields as the user goes, if
    /// anything. `None` means "not stated", resolved by
    /// [`Form::live_validation`](crate::form::Form::live_validation).
    pub fn live_validation(&self) -> Option<bool> {
        self.spec.live_validation
    }

    pub fn classes(&self) -> Option<Classes> {
        self.spec.classes.clone()
    }

    /// The form's `classes_plus:` edits, in source order. Applied by
    /// [`crate::Form::classes`] over whatever the form starts from.
    pub fn class_edits(&self) -> &[crate::classes::ClassEdit] {
        &self.spec.edits
    }

    pub fn title(&self) -> Option<String> {
        self.spec.title.clone()
    }

    /// Validate every field, build the model, then run the form-wide check.
    ///
    /// Returns `None` if anything failed, having left the reasons where they
    /// render: field errors on their own nodes, form-wide ones in
    /// [`errors`](Self::errors).
    ///
    /// **The spec's validator runs LAST, on the built model.** It cannot run
    /// sooner: a cross-field check is a statement about the whole value ("new and
    /// confirm must match", "the end date follows the start"), which does not
    /// exist until every field has parsed. So a form whose fields are individually
    /// fine is built, then judged, and a rejected model is discarded — the work is
    /// wasted only on the path that was going to fail anyway.
    ///
    /// Also why it cannot be a node: a node sees one subtree, and the whole
    /// point of this check is that it sees across them.
    pub fn validate(&mut self) -> Option<T> {
        self.errors.clear();
        self.field_errors.clear();
        let mut is_valid = true;
        for n in &self.nodes {
            is_valid = n.validate(&NodePath::root(), &mut self.field_errors) && is_valid;
        }
        if !is_valid {
            return None;
        }

        let mut partial = Partial::alloc::<T>().expect("alloc should never fail for a concrete T");
        for m in &self.nodes {
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
            // Read only if an error names an item by index, and then once.
            let mut item_keys = None;
            for ValidationError { path, message } in check(&model) {
                match path {
                    None => self.errors.push(message),
                    Some(p) => {
                        let keys = item_keys.get_or_insert_with(|| self.item_keys());
                        errors_by_path
                            .entry(keyed_or_as_written(&p, keys))
                            .or_default()
                            .push(message);
                    }
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

    pub(crate) fn check_live(&self, path: &NodePath, values: &ValuesByPath) -> LiveCheck {
        let owning_node = self
            .nodes
            .iter()
            .find(|n| path.starts_with(&NodePath::root().join(&n.name())));
        match owning_node {
            None => LiveCheck {
                errors_at_path: None,
                paths_to_clear: Vec::new(),
            },
            Some(n) => {
                let mut owning_node = n.clone();
                owning_node.distribute_values(&NodePath::root(), &self.spec.fields, values);
                let mut scratch_errors: ErrorsByPath = ErrorsByPath::default();
                owning_node.validate(&NodePath::root(), &mut scratch_errors);
                let paths_to_clear = match owning_node.absent_owner(&NodePath::root(), path) {
                    None => Vec::new(),
                    Some(owner) => {
                        let mut all_paths = Vec::new();
                        owning_node.paths(&NodePath::root(), Drafts::Exclude, &mut all_paths);
                        all_paths
                            .into_iter()
                            .filter(|p| p != path && p.starts_with(&owner))
                            .collect()
                    }
                };
                LiveCheck {
                    errors_at_path: scratch_errors.shift_remove(path),
                    paths_to_clear,
                }
            }
        }
    }

    fn raw_values(&self) -> Vec<RawValue> {
        let mut out = Vec::new();
        for m in &self.nodes {
            m.collect_values(&NodePath::root(), &mut out);
        }
        out
    }

    /// The leaf entries alone: what a [`ValuesStore`](crate::nodes::ValuesStore)
    /// is seeded with. Structure stays out, because the state owns it. A set's
    /// draft is in, so `reset` clears what was typed into it.
    pub(crate) fn leaf_values(&self) -> ValuesByPath {
        self.raw_values()
            .into_iter()
            .filter_map(|v| match v {
                RawValue::Leaf { path, data } | RawValue::Draft { path, data } => {
                    Some((path, data))
                }
                RawValue::Structure { .. } => None,
            })
            .collect()
    }

    /// Everything a server needs to rebuild this form, keyed by qualified path,
    /// in the order the node tree declares them — the inverse of
    /// [`distribute_values`](Self::distribute_values), and exactly what goes on
    /// the wire.
    ///
    /// Two kinds of entry (see [`RawValue`]): every leaf input's raw value, and
    /// every structure value, which decides which nodes exist at all: an enum's
    /// chosen variant name at the enum's own path (`""` when unchosen), and a
    /// list's item keys in order at the list's own path (`tags` = `#0,#1`). A
    /// client that builds a payload without a form (a script, a test) has to
    /// send both, or the server rebuilds no variants and no items.
    ///
    /// Only the leaves belong in a [`ValuesStore`](crate::nodes::ValuesStore): the
    /// state owns structure, so seeding a store takes the leaves alone.
    ///
    /// A set's draft is left out: until Add commits it, it is not in the set.
    pub fn collect_values(&self) -> ValuesByPath {
        self.raw_values()
            .into_iter()
            .filter(|v| !matches!(v, RawValue::Draft { .. }))
            .map(RawValue::into_pair)
            .collect()
    }

    /// The live form as a [`ValuesByPath`]: what the user typed, plus the
    /// structure the state holds, in the node tree's order. What `Form::to_wire`
    /// sends and what `Form::validate` distributes back.
    ///
    /// The bridge every read of the live values crosses, because the store's map
    /// and [`ValuesByPath`] cannot be the same type — see
    /// [`ValuesStore`](crate::nodes::ValuesStore). Takes its PATHS and their
    /// order from the node tree's own walk (the [`RawValue`]s behind
    /// [`collect_values`](Self::collect_values)), and each VALUE from whichever
    /// side owns it, which the entry's kind decides:
    ///
    /// - A [`RawValue::Leaf`]'s value comes from the STORE, which is what the
    ///   user has typed since the state last saw it. The state's own copy is
    ///   stale by every keystroke since.
    /// - A [`RawValue::Structure`]'s value comes from the ENTRY ITSELF, which is
    ///   the state: a variant chosen in the select or an item added with a button
    ///   lands in the state, through an `Edit`, and never in the store. Looking
    ///   it up in the store would send `""` for a choice made on an empty form,
    ///   and the old choice after the user switched variants.
    ///
    /// Two things follow from taking the paths from the tree. A path the store
    /// holds but the tree does not is dropped — those are the stale keys an
    /// undone variant choice leaves behind, which nothing reads. And a leaf the
    /// tree has but the store does not arrives as `""`, which is the same
    /// "absent IS empty" rule [`distribute_values`](Self::distribute_values) already follows.
    ///
    /// A set's draft is left out, as [`collect_values`](Self::collect_values)
    /// leaves it out: neither `to_wire` nor `validate` sees what was typed into
    /// an adder and never added. [`draft_values`](Self::draft_values) is the
    /// one read of it.
    pub(crate) fn ordered_values(&self, store: &HashMap<NodePath, String>) -> ValuesByPath {
        self.raw_values()
            .into_iter()
            .filter_map(|v| match v {
                RawValue::Leaf { path, .. } => {
                    let raw = store.get(&path).cloned().unwrap_or_default();
                    Some((path, raw))
                }
                RawValue::Structure { path, data } => Some((path, data)),
                RawValue::Draft { .. } => None,
            })
            .collect()
    }

    pub(crate) fn ordered_errors(
        &self,
        store: &HashMap<NodePath, Option<Vec<ValidationMessage>>>,
    ) -> FormErrors {
        self.errors_from(|path| store.get(path)?.as_ref())
    }

    /// The leaves of every set draft under `path`, read from the store: what
    /// committing a draft distributes first (`SETS_PLAN.md` 7c), since the
    /// state never sees a keystroke.
    pub(crate) fn draft_values(
        &self,
        path: &NodePath,
        store: &HashMap<NodePath, String>,
    ) -> ValuesByPath {
        self.raw_values()
            .into_iter()
            .filter_map(|v| match v {
                RawValue::Draft { path: draft, .. } if draft.starts_with(path) => {
                    let raw = store.get(&draft).cloned().unwrap_or_default();
                    Some((draft, raw))
                }
                _ => None,
            })
            .collect()
    }

    /// Whether `path` names a set: a `Def::Set`, or a list `treat_as_set`
    /// marked. For `use_form`, which focuses the item an Add made, except in a
    /// set, whose Add leaves focus in its draft.
    pub(crate) fn is_set(&self, path: &NodePath) -> bool {
        let key = path.spec_key();
        crate::build::spec_key_shape(T::SHAPE, &key).is_ok_and(|shape| {
            crate::field_kind::is_set(shape)
                || (crate::field_kind::is_list(shape)
                    && self.spec.fields.get(&key).is_some_and(|s| s.treat_as_set))
        })
    }

    /// Take raw input values back in, keyed by the same qualified paths
    /// [`collect_values`](Self::collect_values) hands out. Call this on submit, before
    /// `validate()`.
    pub fn distribute_values(&mut self, values: &ValuesByPath) {
        for m in &mut self.nodes {
            m.distribute_values(&NodePath::root(), &self.spec.fields, values);
        }
    }

    /// Every entry `collect_values` sends, by path: each list's and set's
    /// item keys in the order shown (`"answers"` → `"#0,#5"`), and each
    /// item's own text, which a set's item is found by. What
    /// [`ModelPath::resolve`](crate::path::ModelPath::resolve) reads.
    fn item_keys(&self) -> HashMap<NodePath, String> {
        self.collect_values().into_iter().collect()
    }

    /// Add `errors` to the paths this form has, and return the ones it has no
    /// path for. A draft's path is not one: a draft is no part of what was
    /// submitted, so an error aimed at it comes back unclaimed.
    pub fn distribute_errors(&mut self, errors: ErrorsByPath) -> ErrorsByPath {
        let paths: HashSet<NodePath> = self.paths(Drafts::Exclude).into_iter().collect();
        let mut unclaimed = ErrorsByPath::default();
        for (path, messages) in errors {
            if paths.contains(&path) {
                self.field_errors.entry(path).or_default().extend(messages);
            } else {
                unclaimed.insert(path, messages);
            }
        }
        unclaimed
    }

    /// Take values straight off a submitted `<form>`. Dioxus's
    /// `FormData::values()` hands back `(name, FormValue)` pairs keyed by each
    /// input's `name` attribute — which is exactly the qualified path
    /// [`collect_values`](Self::collect_values) emitted — so no per-field signal is needed to
    /// track edits: the DOM already did it.
    pub fn distribute_form_values(&mut self, values: &[(NodePath, String)]) {
        let map: ValuesByPath = values.iter().cloned().collect();
        self.distribute_values(&map);
    }

    pub fn edit(&mut self, edit: &Edit) -> Result<Option<NodePath>, FormAccessError> {
        let path = edit.path();
        let Some(idx) = self
            .nodes
            .iter()
            .position(|n| path.starts_with(&NodePath::root().join(&n.name())))
        else {
            return Err(no_such_path(path));
        };
        // A structural edit can CREATE nodes: `AddItem` builds a fresh item and
        // choosing a variant reveals that variant's fields. Handing the edit the
        // spec is what builds them the way the walk built their siblings, so a
        // item added after mount carries the spec's labels and widgets too.
        let result = self.nodes[idx].edit(
            &NodePath::root(),
            edit,
            &self.spec.fields,
            &mut self.field_errors,
        );
        // An error on a path the edit took away goes with it: a removed item's,
        // or the fields of a variant no longer chosen. This runs on `Err` too,
        // which is why the line above has no `?`: a refused edit records its
        // message before it returns. Drafts count here, so a refusal on one
        // stays until the draft is judged again.
        let paths: HashSet<NodePath> = self.paths(Drafts::Include).into_iter().collect();
        self.field_errors.retain(|path, _| paths.contains(path));
        result
    }

    /// Put `message` on the node at `path`, or say there is none.
    ///
    /// `path` may name one item as a [`path!`](macro@crate::path) spells it:
    /// by index, `answers[2]`, the third item in the order shown, which for a
    /// `Vec` is the model's own; or by value, `tags[=rust]`, the item holding
    /// that text. It is resolved to that item's key (`answers.#5`) here, so
    /// an index past the end or a value no item holds is no path at all.
    pub fn push_field_error(
        &mut self,
        path: &ModelPath<T>,
        message: &str,
    ) -> Result<(), FormAccessError> {
        let keyed = self.key_for(path);
        let mut this_error = ErrorsByPath::default();
        this_error.insert(keyed, vec![ValidationMessage::from(message)]);
        let unclaimed = self.distribute_errors(this_error);
        if unclaimed.is_empty() {
            Ok(())
        } else {
            Err(no_such_path(path))
        }
    }

    pub(crate) fn key_for(&self, path: &ModelPath<T>) -> NodePath {
        keyed_or_as_written(path, &self.item_keys())
    }

    /// Answer the enum at `path`, rebuilding that subtree from the chosen
    /// variant's fields. The schema-rebuild a reactive `<select>` triggers;
    /// add/remove-item will be its sibling.
    ///
    /// **Both failure modes are caller bugs, not user input.** The path comes
    /// from our own node tree and the variant name from an `<option>` we
    /// generated, so neither crosses the wire as a structural instruction. It
    /// still returns `Result` rather than panicking, because on wasm a panic
    /// aborts instead of reaching an `ErrorBoundary` — dioxus-core notes that
    /// unwinds aren't caught there. The `Result` is the transport; the input
    /// layer is expected to push it into the boundary rather than recover.
    pub fn choose_variant(
        &mut self,
        path: &NodePath,
        variant: Option<&str>,
    ) -> Result<(), FormAccessError> {
        self.edit(&Edit::new_choose_variant(path, variant))
            .map(|_| ())
    }

    pub fn buttons(&self) -> &[ButtonSpec] {
        self.spec.buttons()
    }

    /// The title, as a `div` the `<form>` names itself by, NOT a heading.
    ///
    /// Until 2026-10-15 it was an `<h2>`, which picked a level for a page
    /// formoxus cannot see: a form in a section under an `<h2>` wants an
    /// `<h3>`, and a form that IS the page may want an `<h1>` or none. The
    /// examples page put its own `<h2>` above each form, so every one read as
    /// two headings in a row. A plain `div` would lose the title for assistive
    /// technology instead; `aria-labelledby` on the `<form>` (see
    /// `Form::render`) keeps it, by making the form a named landmark, which a
    /// screen reader lists as it lists headings. An app that wants a heading
    /// writes its own and leaves the title off.
    pub(crate) fn render_title(&self, ctx: &NodeContext) -> Element {
        self.title().as_ref().map_or_else(
            || rsx! {},
            |t| {
                rsx! {
                    div {
                        class: ctx.classes.fixed_class(FixedKey::FormTitle, false),
                        id: ctx.title_id(),
                        "{t}"
                    }
                }
            },
        )
    }

    pub(crate) fn render_fields(&self, ctx: &NodeContext) -> Element {
        let nodes_rendered = self.nodes.iter().map(|m| m.render(ctx));
        rsx! {
            { nodes_rendered.into_iter() }
        }
    }

    pub(crate) fn render_errors(&self, ctx: &NodeContext) -> Element {
        if self.errors.is_empty() {
            rsx! {}
        } else {
            rsx! {
                ul {
                    class: ctx.classes.fixed_class(FixedKey::FormErrors, false),
                    for e in self.errors.clone() {
                        li {
                            class: ctx.classes.fixed_class(FixedKey::FormError, false),
                            "{e.0}"
                        }
                    }
                }
            }
        }
    }

    pub fn collect_errors(&self) -> FormErrors {
        self.errors_from(|path| self.field_errors.get(path))
    }

    /// The errors `read` gives for each path, as a [`FormErrors`].
    ///
    /// Walking the tree, not the map, gives render order and leaves a refused
    /// draft out. An empty entry is "checked and clean", which a caller reading
    /// "did it pass?" off `fields` must not see.
    fn errors_from<'a>(
        &self,
        read: impl Fn(&NodePath) -> Option<&'a Vec<ValidationMessage>>,
    ) -> FormErrors {
        let fields = self
            .paths(Drafts::Exclude)
            .into_iter()
            .filter_map(|path| {
                let messages = read(&path).filter(|m| !m.is_empty())?.clone();
                Some((path, messages))
            })
            .collect();
        FormErrors {
            form: self.errors.clone(),
            fields,
        }
    }

    fn paths(&self, drafts: Drafts) -> Vec<NodePath> {
        let mut out: Vec<NodePath> = Vec::new();
        for n in &self.nodes {
            n.paths(&NodePath::root(), drafts, &mut out);
        }
        out
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
/// enum in `T` starts [`VariantChoice::Unchosen`](crate::nodes::VariantChoice) until
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

    // The walk applies the spec as it builds, so every node is right the first
    // time; see `Walk`.
    let nodes = nodes_for(
        T::SHAPE,
        value.map(Peek::new),
        Walk {
            mode,
            specs: &spec.fields,
        },
        &NodePath::root(),
        /* optional */ false,
    );
    FormState {
        spec,
        nodes,
        errors: Vec::new(),
        field_errors: ErrorsByPath::default(),
        _type: PhantomData,
    }
}

/// `path` with each index resolved to its item's key, or as written when one
/// names no item: then the error is placed by a spelling no node claims, and
/// reaches the user the way every unclaimed path does.
fn keyed_or_as_written<T>(path: &ModelPath<T>, item_keys: &HashMap<NodePath, String>) -> NodePath {
    path.resolve(|list| item_keys.get(list).map(String::as_str))
        .unwrap_or_else(|| NodePath::new(path.as_str()))
}

/// What a live check of one path found: what `FormState::check_live` returns
/// and `Form::check_live` writes to the store. Plain data, so the check itself
/// writes nothing and runs without a Dioxus runtime.
///
/// No `path` field: the caller passed it in, and a copy could only disagree.
#[derive(Debug, PartialEq)]
pub(crate) struct LiveCheck {
    /// The path's own result. `Some` when it was validated, an empty `Vec`
    /// meaning checked and clean (#7). `None` when nothing validates it now,
    /// because it is under an absent option or in a draft, which clears it.
    pub errors_at_path: Option<Vec<ValidationMessage>>,
    /// Other paths whose errors no longer apply, because they are under the
    /// option this check found absent. Each is written `None`. Bare paths, not
    /// pairs with an `Option`, so a live check can only ever CLEAR a sibling,
    /// never add an error to one: a field the user has not reached yet is not
    /// scolded.
    pub paths_to_clear: Vec<NodePath>,
}

#[cfg(test)]
mod tests {
    //! `check_live` on plain state: no runtime, no store. What `Form::check_live`
    //! then writes is tested through the controls, once box 6 calls it.

    use facet::Facet;
    use googletest::prelude::*;

    use super::{FormState, LiveCheck, empty_form};
    use crate::error::ValidationMessage;
    use crate::form::spec::FormSpec;
    use crate::nodes::ValuesByPath;
    use crate::path::NodePath;

    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Phone {
        number: String,
        ext: String,
    }

    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Alt {
        label: String,
        phone: Option<Phone>,
    }

    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Reach {
        email: String,
        phone: Option<Phone>,
        fax: Option<Phone>,
        alt: Option<Alt>,
    }

    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Person {
        name: String,
        reach: Reach,
    }

    fn state() -> FormState<Person> {
        empty_form::<Person>(FormSpec::default())
    }

    /// Every value blank, then `filled` typed in: what the store would hold.
    fn values(state: &FormState<Person>, filled: &[(&str, &str)]) -> ValuesByPath {
        let mut values = state.collect_values();
        for (path, text) in filled {
            values.insert(NodePath::new(*path), (*text).to_string());
        }
        values
    }

    fn required() -> Vec<ValidationMessage> {
        vec![ValidationMessage("This field is required.".to_string())]
    }

    /// **A plain leaf gets its own checks,** and nothing else is cleared: no
    /// option is around it.
    #[gtest]
    fn a_blank_required_field_is_required() {
        let state = state();
        let check = state.check_live(&NodePath::new("name"), &values(&state, &[]));
        expect_that!(
            check,
            eq(&LiveCheck {
                errors_at_path: Some(required()),
                paths_to_clear: Vec::new(),
            })
        );
    }

    /// **Checked and clean is an empty `Vec`, not `None`** (#7).
    #[gtest]
    fn a_filled_field_is_checked_and_clean() {
        let state = state();
        let check = state.check_live(&NodePath::new("name"), &values(&state, &[("name", "Ada")]));
        expect_that!(check.errors_at_path, some(is_empty()));
    }

    /// **Blanking the last filled field of an `Option<struct>` clears its
    /// siblings,** and the option's own path: the option is absent now, so
    /// nothing under it applies. The edited path is `None`, not in the list.
    #[gtest]
    fn blanking_the_last_field_clears_the_option() {
        let state = state();
        let check = state.check_live(&NodePath::new("reach.phone.number"), &values(&state, &[]));
        expect_that!(check.errors_at_path, none());
        expect_that!(
            check.paths_to_clear,
            unordered_elements_are![eq("reach.phone"), eq("reach.phone.ext")]
        );
    }

    /// **While the option stays present, its siblings are left alone,** even
    /// though a submit would now say `ext` is required: a live check never adds
    /// an error to a field the user has not reached.
    #[gtest]
    fn a_present_option_clears_nothing() {
        let state = state();
        let check = state.check_live(
            &NodePath::new("reach.phone.number"),
            &values(&state, &[("reach.phone.number", "555")]),
        );
        expect_that!(
            check,
            eq(&LiveCheck {
                errors_at_path: Some(Vec::new()),
                paths_to_clear: Vec::new(),
            })
        );
    }

    /// **Only the option around `path` is cleared, not another absent one in
    /// the same top-level node.** `reach.fax` is absent too, but nothing about it
    /// changed, and an error there may be a form validator's or a server's,
    /// which nothing brings back before submit.
    #[gtest]
    fn another_absent_option_is_left_alone() {
        let state = state();
        let phone = state.check_live(&NodePath::new("reach.phone.number"), &values(&state, &[]));
        expect_that!(
            phone.paths_to_clear,
            each(not(displays_as(starts_with("reach.fax"))))
        );
        let email = state.check_live(&NodePath::new("reach.email"), &values(&state, &[]));
        expect_that!(email.paths_to_clear, is_empty());
    }

    /// **The outermost absent option wins:** with all of `reach.alt` blank, a
    /// check inside its inner `phone` clears the whole of `alt`, not only the
    /// inner option.
    #[gtest]
    fn the_outermost_absent_option_is_cleared() {
        let state = state();
        let check = state.check_live(
            &NodePath::new("reach.alt.phone.number"),
            &values(&state, &[]),
        );
        expect_that!(
            check.paths_to_clear,
            unordered_elements_are![
                eq("reach.alt"),
                eq("reach.alt.label"),
                eq("reach.alt.phone"),
                eq("reach.alt.phone.ext"),
            ]
        );
    }

    /// **An inner absent option inside a present outer one clears only
    /// itself:** `alt.label` is filled, so `alt` is present, and its error stays.
    #[gtest]
    fn an_inner_absent_option_clears_only_itself() {
        let state = state();
        let check = state.check_live(
            &NodePath::new("reach.alt.phone.number"),
            &values(&state, &[("reach.alt.label", "Work")]),
        );
        expect_that!(
            check.paths_to_clear,
            unordered_elements_are![eq("reach.alt.phone"), eq("reach.alt.phone.ext")]
        );
    }

    /// **A path the form does not have** checks nothing and clears nothing.
    #[gtest]
    fn an_unknown_path_finds_nothing() {
        let state = state();
        let check = state.check_live(&NodePath::new("nope"), &values(&state, &[]));
        expect_that!(
            check,
            eq(&LiveCheck {
                errors_at_path: None,
                paths_to_clear: Vec::new(),
            })
        );
    }
}
