//! The `Node` trait every part of a form implements, and the container
//! kinds that make a form a tree rather than a flat list.

use dioxus::html::FileData;
use dioxus::prelude::*;
use facet::{Partial, Peek, ReflectError};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::ops::Deref;
use std::rc::Rc;

mod file_node;
mod items;
mod list_node;
mod option_node;
mod set_node;
mod struct_node;
mod variant_node;

pub use file_node::FileNode;
pub(crate) use file_node::{is_file_upload, stored_shapes};
pub use items::ItemValue;
pub(crate) use items::Items;
pub use list_node::ListNode;
pub use option_node::OptionNode;
pub use set_node::{
    SetEvent, SetFeedback, SetMessage, SetNode, SetOrder, ascii_case_insensitive,
    unicode_case_insensitive,
};
pub use struct_node::StructNode;
pub use variant_node::{VariantChoice, VariantNode};

use crate::classes::Classes;
use crate::error::{FormAccessError, ValidationMessage};
use crate::field_attrs::Validity;
use crate::file::FileMeta;
use crate::form::FieldSpec;
use crate::form::{Contents, ErrorsByPath};
use crate::label_case::{LabelCase, ToCase};
use crate::path::{ITEM_SIGIL, NodePath};
use crate::widgets::Choice;
use crate::widgets::field_handle::OnFieldEvent;
use dioxus::stores::Store;

pub type SpecsByPath = IndexMap<String, FieldSpec>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drafts {
    Include,
    Exclude,
}

/// One part of a form's tree: a leaf ([`FieldNode`](crate::field_node::FieldNode)) or
/// a container ([`StructNode`], [`ListNode`], [`OptionNode`], [`SetNode`],
/// [`VariantNode`]).
///
/// **A node does not know its own path.** Every method that walks takes a
/// `prefix: &NodePath`, which is the PARENT's path, and the node's own is that
/// prefix joined with its segment: its [`name`](Self::name) for a field, `#n`
/// for a list's item, `$Variant` for the chosen variant. So moving an item, or
/// switching a variant, renames nothing stored in the nodes; only the walk's
/// paths change. A `path` argument (as in [`absent_owner`](Self::absent_owner))
/// is a full [`NodePath`], never a [`ModelPath`](crate::path::ModelPath): by
/// the time a node sees a path, the form has resolved any model path to the
/// live node it names.
pub trait Node: Debug {
    /// This node's one segment: a field's name, or an item's key (`#3`).
    /// A `String`, not a [`NodePath`], because it is not a path.
    fn name(&self) -> String;

    fn label(&self, case: LabelCase) -> Option<String>;

    fn render(&self, ctx: &NodeContext) -> Element;

    /// This node's current value as the string an `<input>` would show.
    /// Containers have no scalar value of their own and return `""` — the
    /// widget layer only ever asks leaves for this.
    fn raw_value(&self) -> String;

    /// This node's value as the user reads it, for naming it where it is not
    /// an input: a set's item, its Edit and Remove, its messages. The entries
    /// [`collect_values`](Self::collect_values) sends: a string's text, a
    /// newtype's inner value, each field of a struct. A leaf whose value is
    /// one of its choices gives the choice's label instead, and a variant its
    /// name to be label-cased, which is what each one's `<select>` showed.
    fn shown(&self) -> Shown {
        Shown::of_values(self)
    }

    /// Flatten this node's leaves into `(qualified_path, raw_value)` pairs,
    /// e.g. `("location.street", "123 Main St")`. Paths are qualified because
    /// two structs in one form can each have a `street`.
    fn collect_values(&self, prefix: &NodePath, out: &mut Vec<RawValue>);

    /// The reverse of [`collect_values`](Self::collect_values): each leaf looks
    /// up its own qualified path in `values` and takes the raw string back in.
    /// This is the "shuffle back" from widget state into plain form data.
    fn distribute_values(&mut self, prefix: &NodePath, specs: &SpecsByPath, values: &ValuesByPath);

    /// Take each file's size and content type in from `files`, the table of
    /// files picked or received, keyed by the same qualified paths as `values`.
    /// A container passes the table on to exactly the children its
    /// `distribute_values` reaches, under the same prefixes; a `FileNode` takes
    /// the entry at its own path. Required, not defaulted, so a new container
    /// can't silently skip its files.
    ///
    /// Two rules for the caller, both because a `FileNode` REPLACES what it had
    /// with what the table holds at its path, `None` when nothing:
    /// 1. **Always the whole table, never part of it.** A path missing from
    ///    `files` means "no file picked here", unlike `values`, which may be
    ///    partial. A table holding only the file that just changed would drop
    ///    every other file's size and type, and their checks would pass.
    /// 2. **Values first, then files.** `distribute_values` builds the
    ///    structure (variants, list items) that a file lands in, and a
    ///    `FileNode` forgets its file when the name in the store changes.
    fn distribute_files(&mut self, prefix: &NodePath, files: &FilesByPath);

    /// Validate this node and its children, adding any errors to out, by path
    /// returns true if no errors were added, false if some were
    fn validate(&self, prefix: &NodePath, out: &mut ErrorsByPath) -> bool;

    /// All paths on this node (that can hold an error).
    fn paths(&self, prefix: &NodePath, drafts: Drafts, out: &mut Vec<NodePath>);

    fn clone_box(&self) -> Box<dyn Node>;

    fn write_value_into<'p>(&self, partial: Partial<'p>) -> Result<Partial<'p>, ReflectError>;

    /// The value this node holds, readable by reflection, or `None` if it has
    /// none to give: what a set sorts its items by.
    ///
    /// Only a leaf holding a valid value has one. A container holds nodes, not
    /// a value, and a leaf that is `Empty` or `Invalid` has nothing to
    /// compare, so `Items::sort_by` puts those last. A newtype's leaf gives
    /// its INNER value, since that is what it holds.
    fn peek_value(&self) -> Option<Peek<'_, 'static>> {
        None
    }

    fn write_into<'p>(&self, partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        let mut partial = partial.begin_field(&self.name())?;
        partial = self.write_value_into(partial)?;
        partial.end()
    }

    fn is_present(&self) -> bool;

    /// Apply a structural edit — choose a variant, add or remove an item.
    ///
    /// One method rather than one per edit kind because the containment walk
    /// each container performs is identical regardless of the edit; only the
    /// node that owns the path cares what kind it is. `Edit::path()` is the
    /// only thing a container reads, so containers stay kind-agnostic.
    ///
    /// `Ok(Some(path))` is the full path of the item an `AddItem` created
    /// (`answers.#3`), which is how a caller finds an item whose key it could
    /// not have known in advance. Every other edit, which creates no item, is
    /// `Ok(None)`. `Err` when no node owns the path or when the edit does not
    /// fit the node that does: an out-of-range item, a variant the enum lacks,
    /// an `ItemValue` of the wrong type. A refused edit changes nothing.
    fn edit(
        &mut self,
        prefix: &NodePath,
        edit: &Edit,
        specs: &SpecsByPath,
        errors: &mut ErrorsByPath,
    ) -> Result<Option<NodePath>, FormAccessError>;

    /// The path of the OUTERMOST absent option that owns `path`, or `None` if
    /// no option around it is absent. `prefix` is this node's parent's path, as
    /// in every other method here, and `path` is a full one.
    ///
    /// "Absent" means `!is_present()`, not that a `None` is stored anywhere: an
    /// `Option<struct>` whose fields are all blank is absent. Outermost, because
    /// everything under an absent option has stopped applying, an inner absent
    /// option's paths included; so an `OptionNode` answers for itself BEFORE it
    /// asks its inner node.
    ///
    /// What a live check clears: errors under this option, whoever wrote them,
    /// and nothing outside it (`FormState::check_live`). A node that does not own
    /// `path` answers `None`, so a container can ask each child in turn and take
    /// the first `Some`.
    fn absent_owner(&self, prefix: &NodePath, path: &NodePath) -> Option<NodePath>;

    fn choices(&self, case: LabelCase) -> Option<Vec<Choice>>;
}

impl Clone for Box<dyn Node> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

pub(crate) fn no_such_path(path: &impl Display) -> FormAccessError {
    FormAccessError(format!("no such path: {path}"))
}

/// A node's name humanized for display: `can_shuffle` -> "Can Shuffle".
///
/// The fallback when nothing set a label explicitly, which is every node today
/// — the shape carries a field's name but no prose for it. Reuses the derive
/// path's [`crate::label_case`], so both paths title-case identically.
///
/// `None` for a `ListNode` item key (`#3`) and for an all-digit name. Neither is
/// prose: an item key is an identity, and a tuple-struct field's "0" is a
/// position. The list itself is what carries the label.
pub(crate) fn default_label(name: &str, case: LabelCase) -> Option<String> {
    if name.starts_with(ITEM_SIGIL) || name.chars().all(|c| c.is_ascii_digit()) {
        None
    } else {
        Some(name.to_case(case))
    }
}

/// One entry [`Node::collect_values`] emits, as a qualified path and its raw
/// string.
///
/// A [`Leaf`](Self::Leaf) is what a user types into one input, and lives in the
/// [`ValuesStore`]. A [`Structure`](Self::Structure) decides which nodes exist
/// at all (an enum's variant name, a list's item keys), and lives only in the
/// state, never in the store. Both cross the wire, because a server rebuilds
/// from an empty form and has no other way to learn the structure.
///
/// A [`Draft`](Self::Draft) is a leaf of a set's draft, the blank item its
/// adder renders (`SETS_PLAN.md` box 7). It lives in the store like a
/// leaf, so what the user types is kept, and `reset` clears it, but it never
/// crosses the wire and is never validated: until Add commits it, it is not in
/// the set.
#[derive(Clone, Debug)]
pub enum RawValue {
    Leaf { path: NodePath, data: String },
    Structure { path: NodePath, data: String },
    Draft { path: NodePath, data: String },
}

impl RawValue {
    pub fn into_pair(self) -> (NodePath, String) {
        match self {
            RawValue::Leaf { path: p, data: d }
            | RawValue::Structure { path: p, data: d }
            | RawValue::Draft { path: p, data: d } => (p, d),
        }
    }
}

/// A field's own check (`validator:` on a field in `form!`): its value, typed,
/// in; messages for that field out, with no path, since it is the field's.
///
/// Erased so one spec type holds a check for any field, and an `Arc<dyn Fn>`
/// so a closure can capture app state, as a set's `feedback:` can. Built
/// typed by [`FieldValidator::new`], which reads the node's value as `P`:
/// straight off a leaf, or rebuilt from a container's children the way a
/// model is built from the whole tree.
#[derive(Clone)]
pub struct FieldValidator(
    #[allow(clippy::type_complexity)]
    std::sync::Arc<dyn Fn(&dyn Node) -> Vec<crate::error::ValidationMessage> + Send + Sync>,
);

impl FieldValidator {
    /// A check over values of type `P`, the field's own type (an `Option`'s
    /// inner one, since it runs only on a value that is there).
    pub fn new<P: facet::Facet<'static> + 'static>(
        check: impl Fn(&P) -> Vec<crate::error::ValidationMessage> + Send + Sync + 'static,
    ) -> Self {
        Self(std::sync::Arc::new(move |node| {
            with_value::<P, _>(node, &check).unwrap_or_default()
        }))
    }

    /// The messages for `node`, which its own validation has found nothing
    /// wrong with. None when its value cannot be read as the check's type.
    pub(crate) fn messages_for(&self, node: &dyn Node) -> Vec<crate::error::ValidationMessage> {
        (self.0)(node)
    }
}

/// `node`'s value as a `P`, handed to `f`: a leaf's own, or else the value
/// its tree writes, built as `FormState::validate` builds the model. `None`
/// when it cannot be built as a `P`.
fn with_value<P: facet::Facet<'static> + 'static, R>(
    node: &dyn Node,
    f: impl FnOnce(&P) -> R,
) -> Option<R> {
    if let Some(peek) = node.peek_value()
        && let Ok(value) = peek.get::<P>()
    {
        return Some(f(value));
    }
    let partial = Partial::alloc::<P>().ok()?;
    let value = node
        .write_value_into(partial)
        .ok()?
        .build()
        .ok()?
        .materialize::<P>()
        .ok()?;
    Some(f(&value))
}

impl Debug for FieldValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FieldValidator(..)")
    }
}

/// The same check only when it is the same `Arc`: a closure has no `==`.
impl PartialEq for FieldValidator {
    fn eq(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.0, &other.0)
    }
}

/// A node's value as the user reads it ([`Node::shown`]), in pieces, so the
/// label case it is shown in can wait until it is known: a variant's name is
/// cased as the form's `<select>`s case it, which is resolved when the form
/// renders, not when an edit is made.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shown(Vec<ShownPiece>);

#[derive(Clone, Debug, PartialEq)]
struct ShownPiece {
    text: String,
    /// A variant's name, shown in the form's label case.
    cased: bool,
}

impl Shown {
    /// One piece, shown as it is: a choice's label.
    pub(crate) fn plain(text: String) -> Self {
        Self(vec![ShownPiece { text, cased: false }])
    }

    /// The entries `node.collect_values` sends, each shown as it is.
    pub(crate) fn of_values<N: Node + ?Sized>(node: &N) -> Self {
        let mut values = Vec::new();
        node.collect_values(&NodePath::root(), &mut values);
        Self(
            values
                .into_iter()
                .map(|value| ShownPiece {
                    text: value.into_pair().1,
                    cased: false,
                })
                .collect(),
        )
    }

    /// Mark the first piece a variant's name: a `VariantNode` writes its own
    /// entry before its fields'.
    pub(crate) fn first_is_a_variant(mut self) -> Self {
        if let Some(first) = self.0.first_mut() {
            first.cased = true;
        }
        self
    }

    /// The text, in `case`: the pieces that are not empty, joined.
    pub fn in_case(&self, case: LabelCase) -> String {
        self.0
            .iter()
            .filter(|piece| !piece.text.is_empty())
            .map(|piece| {
                if piece.cased {
                    piece.text.as_str().to_case(case)
                } else {
                    piece.text.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Whether there is nothing to show at all.
    pub fn is_empty(&self) -> bool {
        self.0.iter().all(|piece| piece.text.is_empty())
    }
}

/// The live raw values, keyed by qualified path — what `leaves()` produces
/// and what `apply()` consumes.
pub type ValuesByPath = IndexMap<NodePath, String>;

/// The files picked or received, keyed by the file leaf's qualified path: the
/// size and content type of each, which the store (one string per path) can't
/// hold. What [`Node::distribute_files`] takes; always the whole table.
pub type FilesByPath = IndexMap<NodePath, FileMeta>;

/// A [`std::collections::HashMap`] equivalent to [`ValuesByPath`] held in
/// a store so a write touches one input. The two maps are different,
/// because [`ValuesByPath`] maintains insertion order, but
/// [`struct@dioxus_stores::Store`] doesn't support [`indexmap::IndexMap`], just
/// [`std::collections::HashMap`] and a few others.
pub type ValuesStore = Store<HashMap<NodePath, String>>;

/// The form's file table: each file the user picked, keyed by its file
/// leaf's qualified path. Only [`FieldHandle::pick_file`] writes it, and
/// [`FieldHandle::picked_file`] reads one entry.
///
/// One `Signal`, not a `Store`, so a pick re-renders every file widget in
/// the form (`WIDGETS_PLAN.md` decision KK). A store's `remove` and `clear` wake
/// only readers of the map's shape, not of the removed key, so per-widget
/// re-rendering would need `Option` values set to `None` instead. Crate-
/// private, so that choice can change without breaking anyone.
///
/// [`FieldHandle::pick_file`]: crate::widgets::FieldHandle::pick_file
/// [`FieldHandle::picked_file`]: crate::widgets::FieldHandle::picked_file
pub(crate) type FilesSignal = Signal<IndexMap<NodePath, FileData>>;

/// Client home for field errors. Held in a store so that an error on one
/// field only redraws that field, not the whole page. This is a three-way
/// distinction, for each path:
///
/// - `None`, or a missing key, means that path was never checked or was cleared.
/// - `Some(vec![])` means that path was valid.
/// - `Some(vec![...])` means that path did not validate.
pub type ErrorsStore = Store<HashMap<NodePath, Option<Vec<ValidationMessage>>>>;

/// Reading one path's errors out of an [`ErrorsStore`], for rendering.
///
/// A trait because `ErrorsStore` is an alias for dioxus's `Store`, so the
/// orphan rule allows no inherent methods on it. Crate-private: a widget reads
/// its field's errors through [`WidgetProps`](crate::widgets::WidgetProps),
/// which already knows the path, and never needs the store.
pub(crate) trait ErrorsStoreExt {
    /// The entry at `path` as the store holds it: `None` when it is unchecked
    /// (no key, or a key holding `None`), `Some` of the messages otherwise,
    /// empty when the path is valid.
    ///
    /// **Reads one key, not the map.** `self.read()` would subscribe the caller
    /// to every path, so one field's error would wake every widget. A key that
    /// is not there yet still wakes its reader when `insert` first writes it.
    /// Owned rather than a borrow, because the borrow would be of a read guard
    /// that ends when this returns.
    fn errors_at(&self, path: &NodePath) -> Option<Vec<ValidationMessage>>;

    /// The [`Validity`] of `path`, from [`errors_at`](Self::errors_at), with
    /// the same one-key subscription.
    fn validity_at(&self, path: &NodePath) -> Validity;
}

impl ErrorsStoreExt for ErrorsStore {
    fn errors_at(&self, path: &NodePath) -> Option<Vec<ValidationMessage>> {
        self.get_unchecked(path.clone()).try_read().ok()?.cloned()
    }

    fn validity_at(&self, path: &NodePath) -> Validity {
        Validity::of(self.errors_at(path).as_deref())
    }
}

/// What every node of one render shares: the form's id, where its values,
/// files and errors live, where edits and field events go, and its resolved
/// label case and classes. `Form::root_ctx` builds one per render, since
/// `label_case`, `classes` and whether checks are live are resolved from the
/// cascade each time (`.claude/memory/config_cascade.md`), and
/// `NodeContext::root` puts it behind an `Rc`.
///
/// **Split from [`NodeContext`] by lifetime.** These fields are the same at
/// every node; `NodeContext`'s own fields (`prefix`, `required`,
/// `label_if_unnamed`, `in_draft`) are the ones a descent changes. So
/// [`nested`](NodeContext::nested) and [`optional`](NodeContext::optional)
/// copy one `Rc` for all of these, and a new form-wide field goes HERE, and is
/// set where `Form::root_ctx` builds this, never in `descend_to` or `optional`.
/// Before the split, each form-wide field was written in four places and
/// `form_id` was cloned at every node.
///
/// **No `Clone`,** on purpose: every copy should be an `Rc::clone`, and
/// without `Clone` a deep copy (a `String` and an `Rc` bump) can't be written
/// by accident, as through `NodeContext`'s `Deref`.
#[derive(Debug)]
pub struct FormWide {
    pub form_id: String,
    pub(crate) contents: Contents,
    pub errors: ErrorsStore,
    pub on_edit: Callback<Edit>,
    pub(crate) on_field_event: OnFieldEvent,
    /// Already resolved — the cascade is settled once, where the ctx is built,
    /// rather than re-decided at every node. See
    /// `.claude/memory/config_cascade.md`.
    pub label_case: LabelCase,
    /// The form's classes, resolved once by `Form::classes()` (the app's,
    /// then the form's `classes:` and `classes_plus:`), shared by every node
    /// and widget of one render.
    ///
    /// **An `Rc` of its own, inside the `Rc<FormWide>`,** because a node hands
    /// it on to [`WidgetCommon`](crate::widgets::WidgetCommon), which holds
    /// `classes` and must not hold the whole `FormWide`: that would give every
    /// widget the form's callbacks.
    ///
    /// **`Rc` only in render-side types:** this one, the `Rc<FormWide>` in
    /// [`NodeContext`], and `WidgetCommon`'s. Dioxus renders on one thread, so
    /// `Arc` would only add an atomic count. But an `Rc` must never go into
    /// `FormSpec` or `FormState`: both reach the server, where `Submission`
    /// rebuilds a `FormState` and a handler's future must be `Send`. They hold
    /// `Classes` by value. See `.claude/memory/config_cascade.md`.
    pub classes: Rc<Classes>,
}

/// What a node needs in order to render: what the whole form shares, in
/// [`form`](Self::form), and what is particular to this node: where it sits in
/// the path tree, whether the browser should treat its leaves as required, and
/// two per-node settings that a descent drops.
///
/// It derefs to [`FormWide`], so `ctx.classes` or `ctx.on_edit` reads the
/// form-wide field directly. See there for why the two are split.
///
/// **`required` is a presentation hint, not the authority.** HTML5 `required`
/// is per-input, so it cannot express the all-or-nothing rule an optional
/// struct follows (absent means EVERY leaf empty; a partly filled one is an
/// error). Marking those leaves required would block a deliberately blank
/// address; leaving them unmarked lets the browser accept a half-filled one.
/// The second is the lesser evil, and `validate()` remains where the real rule
/// lives — see the optional-container tests.
#[derive(Clone, Debug)]
pub struct NodeContext {
    /// Shared by every node of this render, and copied as one `Rc` on each
    /// descent. Read through `Deref` (`ctx.classes`) rather than `ctx.form`.
    pub form: Rc<FormWide>,
    pub prefix: NodePath,
    pub required: bool,
    /// The label for a leaf that has none of its own, given only to a set's
    /// draft ("Add to Tags"): a set's items go unlabelled, but the draft is a
    /// box the user types into, so it needs a name. A leaf's own label, from
    /// `tags[] => { label: … }`, still wins. [`nested`](Self::nested) and
    /// [`optional`](Self::optional) drop it, so it reaches only the node it
    /// was given to.
    pub label_if_unnamed: Option<String>,
    /// Rendering a set's draft (`SETS_PLAN.md` box 14): a leaf leaves its
    /// field's constraint attributes (`pattern`, `min`, `minlength`, …) off
    /// its input, keeping only `maxlength`, which typing cannot break. The
    /// draft's input is inside the `<form>`, so the browser would otherwise
    /// check it at submit and block one formoxus would allow, since text left
    /// in the draft is not in the set. The constraints still apply: Add
    /// validates the draft. Dropped by [`nested`](Self::nested) and
    /// [`optional`](Self::optional), like `label_if_unnamed`.
    pub in_draft: bool,
}

impl NodeContext {
    /// The context a whole form starts from: at the root, and required until
    /// some `OptionNode` says otherwise.
    pub(crate) fn root(form: FormWide) -> Self {
        Self {
            form: Rc::new(form),
            prefix: NodePath::root(),
            required: true,
            label_if_unnamed: None,
            in_draft: false,
        }
    }

    /// Descend into a named child, as `NodePath::join` does.
    pub fn nested(&self, name: &str) -> Self {
        self.descend_to(self.prefix.join(name))
    }

    /// Descend into the chosen variant of an enum, as `NodePath::join_variant`
    /// does.
    pub fn nested_variant(&self, name: &str) -> Self {
        self.descend_to(self.prefix.join_variant(name))
    }

    /// What `nested` and `nested_variant` share: everything inherited but the
    /// path, and the per-node settings (`label_if_unnamed`, `in_draft`) reset.
    fn descend_to(&self, prefix: NodePath) -> Self {
        Self {
            form: self.form.clone(),
            prefix,
            required: self.required,
            label_if_unnamed: None,
            in_draft: false,
        }
    }

    /// Everything from here down is optional. `OptionNode` changes the
    /// context this way for its payload, and `SetNode` for its draft, which
    /// an empty box must not block a submit for.
    pub fn optional(&self) -> Self {
        Self {
            form: self.form.clone(),
            prefix: self.prefix.clone(),
            required: false,
            label_if_unnamed: None,
            in_draft: false,
        }
    }

    /// This node's own qualified path, for a leaf's `name` attribute.
    pub fn path(&self, name: &str) -> NodePath {
        self.prefix.join(name)
    }

    pub fn id(&self, name: &str) -> String {
        format!("{}-{}", self.form_id, self.path(name))
    }

    /// The form title's `id`, which the `<form>`'s `aria-labelledby` points
    /// at. `--`, not `-`: a top-level field named `title` already has
    /// `{form id}-title`, and no field's id, nor its `-errors` or `-legend`,
    /// can contain `--`, since a path is identifiers joined by `.`.
    pub fn title_id(&self) -> String {
        format!("{}--title", self.form_id)
    }
}

impl Deref for NodeContext {
    type Target = FormWide;

    fn deref(&self) -> &FormWide {
        &self.form
    }
}

/// A change to the form's *shape*, as opposed to its values — the counterpart
/// to `apply`. Crosses the widget boundary in a `Callback<Edit>`, which is why
/// it owns its strings rather than borrowing.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    ChooseVariant {
        path: NodePath,
        variant: Option<String>,
    },
    AddItem {
        path: NodePath,
        /// What the new item holds. `None` is a blank item, the one the Add
        /// button makes: every leaf empty, an enum item with no variant chosen.
        /// `Some` builds the item from the value the way `form_for` builds a
        /// form from a model, so its leaves show the value and an enum item
        /// starts on the value's variant.
        ///
        /// The value must be of the list's item type (see [`ItemValue::new`]),
        /// or the edit is refused and nothing changes. Through
        /// [`Form::edit`](crate::form::Form::edit) or a button, a filled item's
        /// values also reach the store, which `validate` and `to_wire` read.
        /// [`FormState::edit`](crate::form::FormState::edit) alone puts them
        /// only in the tree.
        value: Option<ItemValue>,
        /// Insert the new item *before* the item currently at this position, or
        /// append when `None`. A position rather than a key because that is what
        /// the widget knows: an "insert here" button sits between two rendered
        /// items and knows only where it is. Keys identify an item across edits;
        /// positions locate a gap at one instant, which is all an insert needs.
        before: Option<usize>,
    },
    RemoveItem {
        path: NodePath,
        index: usize,
    },
    MoveItem {
        path: NodePath,
        /// The position of the item to move. Positions, not keys, for the same
        /// reason as `AddItem::before`: a ↑/↓ button knows where its item is.
        from: usize,
        /// The item's position AFTER the move, so the items between shift to close
        /// the gap: in `a,b,c`, `from: 0, to: 2` gives `b,c,a`, not the swap
        /// `c,b,a`. Moving down one place from `i` is `to: i + 1`.
        to: usize,
    },
    /// Start or stop editing an item of a set (`SETS_PLAN.md` box 8).
    /// `Some(index)` marks the item at that position and puts its value into
    /// the set's draft, where an `AddItem { value: None }` then replaces it
    /// with what the draft holds. `None` stops editing, leaving the item as it
    /// was, and empties the draft. A set's only: a list's items are edited in
    /// place.
    EditItem {
        path: NodePath,
        index: Option<usize>,
    },
}

impl Edit {
    /// Every edit names a target, and the containment walk reads only this —
    /// never the kind.
    pub fn path(&self) -> &NodePath {
        match self {
            Edit::ChooseVariant { path, .. }
            | Edit::AddItem { path, .. }
            | Edit::RemoveItem { path, .. }
            | Edit::MoveItem { path, .. }
            | Edit::EditItem { path, .. } => path,
        }
    }

    pub fn new_choose_variant(path: &NodePath, variant: Option<&str>) -> Edit {
        Edit::ChooseVariant {
            path: path.clone(),
            variant: variant.map(str::to_string),
        }
    }
}
