//! A set-typed node. Items are nodes named by a `#`-prefixed KEY, which is
//! what makes `answer_choices.#0.text` fall out of the ordinary `join`
//! nesting — and, because a key is an identity rather than a position, what lets
//! an item be inserted anywhere.
//!
//! Items are kept sorted by a [`SetOrder`]. A set of scalars (`String`, the
//! integers, `bool`, the floats) has a default order, and so does a set of a
//! unit enum: its variants' declaration order (`WIDGETS_PLAN.md` decision O).
//! Any other item type needs one given, by `sort_by:` in `form!` (for an `Ord`
//! type, `sort_by: Tag::cmp`), and [`SetOrder::default_for`] panics without it.

use std::{
    cmp::Ordering,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use dioxus::{
    core::{DynamicNode, VComponent},
    prelude::*,
};
use facet::{Def, Facet, Partial, Peek, ReflectError, ScalarType, Shape, Type, UserType};
use formoxus_spec::FixedKey;
use unicase::{Ascii, UniCase};

use crate::{
    build::retain_first,
    error::FormAccessError,
    field_attrs::Validity,
    form::ErrorsByPath,
    label_case::LabelCase,
    nodes::{
        Drafts, Edit, ErrorsStoreExt, Items, Node, NodeContext, RawValue, Shown, SpecsByPath,
        ValuesByPath, default_label,
    },
    path::NodePath,
    widgets::{
        AddItemProps, Choice, DraftAdder, EditItemButton, FieldErrors, ItemAdder, ItemView,
        RemoveItemButton, SetItemProps, SetWidgetProps, WidgetCommon, described_by, field_help,
        field_row::required_marker, widget::AnySetWidget,
    },
};

pub fn ascii_case_insensitive<T: AsRef<str>>(a: &T, b: &T) -> Ordering {
    Ascii::new(a.as_ref())
        .cmp(&Ascii::new(b.as_ref()))
        .then_with(|| a.as_ref().cmp(b.as_ref()))
}

pub fn unicode_case_insensitive<S: AsRef<str>>(a: &S, b: &S) -> Ordering {
    UniCase::new(a.as_ref())
        .cmp(&UniCase::new(b.as_ref()))
        .then_with(|| a.as_ref().cmp(b.as_ref()))
}

#[derive(Clone)]
pub struct SetOrder {
    pub item_order: Arc<dyn ItemHandler>,
}

impl SetOrder {
    /// The order of a set at `path` whose items are `shape`, when no
    /// `sort_by` was given. Scalar items and unit enum items have one.
    pub fn default_for(shape: &'static Shape, path: &NodePath) -> Self {
        if let Some(scalar_type) = shape.scalar_type() {
            SetOrder {
                item_order: Arc::new(DefaultOrder::for_type(scalar_type)),
            }
        } else if let Some(declared_order) = DeclaredOrder::for_shape(shape) {
            SetOrder {
                item_order: Arc::new(declared_order),
            }
        } else {
            panic!(
                "{path}: a set of `{shape}` has no default order; give it a sort_by \
                 (only a set of scalars or of a unit enum sorts without one)"
            );
        }
    }

    pub fn custom<T: Facet<'static> + 'static + PartialEq>(
        cmp: fn(&T, &T) -> Ordering,
        name: &str,
    ) -> Self {
        Self {
            item_order: Arc::new(CustomOrder {
                cmp_value: cmp,
                name: name.to_string(),
            }),
        }
    }
}

impl Debug for SetOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetOrder")
            .field("item_order", &self.item_order)
            .finish()
    }
}

impl PartialEq for SetOrder {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.item_order, &other.item_order)
    }
}

pub trait ItemHandler: Debug + Send + Sync {
    fn name(&self) -> String;

    /// Sort a set's values as `set_node` found them, BEFORE their keys are
    /// assigned: a `HashSet` iterates in an order that differs between
    /// processes, so keys assigned in iteration order would differ between
    /// the server's render and the client's hydration.
    fn sort_peeks(&self, peeks: &mut Vec<Peek<'_, 'static>>);

    /// Sort built items in place, after an `AddItem` and after
    /// `distribute_values`. An item with nothing to compare (an `Empty` or
    /// `Invalid` leaf, say, after a failed submit) goes after every item that
    /// has a value, in the order it was already in (`sort_by` is stable).
    fn sort_nodes(&self, nodes: &mut Vec<Box<dyn Node>>);

    fn dedup_nodes(&self, nodes: &mut Vec<Box<dyn Node>>);

    /// Where an item equal to `candidate` sits in `nodes`, by the model's own
    /// `PartialEq` (decision H), for refusing a duplicate (box 6): the node an
    /// `AddItem`'s value would become, or a set's draft (box 7). A node with
    /// no value equals nothing.
    fn position(&self, nodes: &[Box<dyn Node>], candidate: &dyn Node) -> Option<usize>;
}

struct DefaultOrder {
    scalar_type: ScalarType,
    peek_cmp: fn(&Peek<'_, '_>, &Peek<'_, '_>) -> Ordering,
}

impl DefaultOrder {
    fn for_type(scalar_type: ScalarType) -> Self {
        let peek_cmp: fn(&Peek<'_, '_>, &Peek<'_, '_>) -> Ordering = match scalar_type {
            ScalarType::F32 => |a, b| {
                a.get::<f32>()
                    .expect("type checked")
                    .total_cmp(b.get::<f32>().expect("type checked"))
            },
            ScalarType::F64 => |a, b| {
                a.get::<f64>()
                    .expect("type checked")
                    .total_cmp(b.get::<f64>().expect("type checked"))
            },
            _ => |a, b| {
                a.partial_cmp(b)
                    .unwrap_or_else(|e| panic!("cannot compare {}: {e}", a.shape()))
                    .unwrap_or_else(|| panic!("{} has no ordering", a.shape()))
            },
        };
        Self {
            scalar_type,
            peek_cmp,
        }
    }
}

impl Debug for DefaultOrder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultOrder({:?})", self.scalar_type)
    }
}

impl ItemHandler for DefaultOrder {
    fn name(&self) -> String {
        format!("{self:?}")
    }

    fn sort_peeks(&self, peeks: &mut Vec<Peek<'_, 'static>>) {
        peeks.sort_by(self.peek_cmp);
    }

    fn sort_nodes(&self, nodes: &mut Vec<Box<dyn Node>>) {
        let cmp = self.peek_cmp;
        nodes.sort_by(|a, b| match (a.peek_value(), b.peek_value()) {
            (Some(a), Some(b)) => cmp(&a, &b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        });
    }

    fn dedup_nodes(&self, nodes: &mut Vec<Box<dyn Node>>) {
        retain_first(nodes, |a, b| match (a.peek_value(), b.peek_value()) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        });
    }

    fn position(&self, nodes: &[Box<dyn Node>], candidate: &dyn Node) -> Option<usize> {
        let value = candidate.peek_value()?;
        nodes
            .iter()
            .position(|node| node.peek_value().is_some_and(|item| item == value))
    }
}

/// A unit enum's default order: its variants' declaration order, which is
/// also the order its `choices` list them in, so a set widget's ticked items
/// line up with its boxes (`WIDGETS_PLAN.md` decision O).
///
/// A peek knows its variant's index. A node does not: a `VariantNode`'s
/// `peek_value` is `None`, as it holds a choice, not a value. So a node is
/// ranked by its entry, the variant name it sends at its own path, looked up
/// in `variants`. An item with no variant chosen, or an unknown one from a bad
/// submit, has no rank and goes last, as an `Empty` leaf does in
/// `DefaultOrder`.
struct DeclaredOrder {
    shape: &'static Shape,
    variants: Vec<&'static str>,
}

impl DeclaredOrder {
    /// `Some` for an enum whose every variant is unit. Matched on `shape.ty`
    /// directly, not through `field_kind::is_unit_enum`, which also looks
    /// through a transparent newtype: a newtype's peek and node are the
    /// newtype's, not the enum's, so the lookups here would not hold. An
    /// `Option` is a `UserType::Enum` too, but its `Some` carries a field.
    fn for_shape(shape: &'static Shape) -> Option<Self> {
        let Type::User(UserType::Enum(enum_type)) = &shape.ty else {
            return None;
        };
        if !enum_type.variants.iter().all(|v| v.data.fields.is_empty()) {
            return None;
        }
        Some(Self {
            shape,
            variants: enum_type.variants.iter().map(|v| v.name).collect(),
        })
    }

    fn node_rank(&self, node: &dyn Node) -> Option<usize> {
        let entry = own_entry(node)?;
        self.variants.iter().position(|v| *v == entry)
    }
}

impl Debug for DeclaredOrder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeclaredOrder({})", self.shape)
    }
}

impl ItemHandler for DeclaredOrder {
    fn name(&self) -> String {
        format!("{self:?}")
    }

    fn sort_peeks(&self, peeks: &mut Vec<Peek<'_, 'static>>) {
        let shape = self.shape;
        peeks.sort_by_key(|p| {
            p.into_enum()
                .unwrap_or_else(|e| panic!("a `{shape}` item should peek as an enum: {e}"))
                .variant_index()
                .unwrap_or_else(|e| panic!("a `{shape}` item has no active variant: {e}"))
        });
    }

    fn sort_nodes(&self, nodes: &mut Vec<Box<dyn Node>>) {
        // `None` last: a plain `Option` sorts it first.
        nodes.sort_by_key(|n| self.node_rank(&**n).map_or((1, 0), |rank| (0, rank)));
    }

    fn dedup_nodes(&self, nodes: &mut Vec<Box<dyn Node>>) {
        retain_first(
            nodes,
            |a, b| matches!((self.node_rank(&**a), self.node_rank(&**b)), (Some(a), Some(b)) if a == b),
        );
    }

    fn position(&self, nodes: &[Box<dyn Node>], candidate: &dyn Node) -> Option<usize> {
        let rank = self.node_rank(candidate)?;
        nodes
            .iter()
            .position(|node| self.node_rank(&**node) == Some(rank))
    }
}

/// What an item sends at its own path: a leaf's raw value, or a unit enum's
/// variant name, which goes out as structure. `None` for an item with nothing
/// there, such as a struct, whose values are all at paths below its own.
///
/// Asked at the root, so the item's own path is just its name. Read from the
/// tree, never the store; see `SetNode::chosen`.
fn own_entry(item: &dyn Node) -> Option<String> {
    let own_path = NodePath::root().join(&item.name());
    let mut values = Vec::new();
    item.collect_values(&NodePath::root(), &mut values);
    values
        .into_iter()
        .map(RawValue::into_pair)
        .find(|(path, _)| *path == own_path)
        .map(|(_, data)| data)
}

struct CustomOrder<T> {
    cmp_value: fn(&T, &T) -> Ordering,
    name: String,
}

/// Rebuild an item's `T` from its node, or `None` for an item that has no
/// value to build (an `Empty` or `Invalid` leaf). For what needs a typed item
/// and has only the node: a custom order, and a dev's `feedback`.
fn item_value<T: Facet<'static>>(node: &dyn Node) -> Option<T> {
    let is_valid = node.validate(&NodePath::root(), &mut ErrorsByPath::default());
    if !is_valid {
        return None;
    }
    let partial = Partial::alloc::<T>().expect("alloc should never fail for a concrete T");
    node.write_value_into(partial)
        .expect("write_value_into should succeed once validate() found no errors")
        .build()
        .expect("build should succeed once the item was written")
        .materialize::<T>()
        .expect("materialized shape should match T")
        .into()
}

impl<T> Debug for CustomOrder<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomOrder")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl<T: Facet<'static> + 'static + PartialEq> ItemHandler for CustomOrder<T> {
    fn name(&self) -> String {
        format!("CustomOrder({})", self.name)
    }

    fn sort_peeks(&self, peeks: &mut Vec<Peek<'_, 'static>>) {
        peeks.sort_by(|a, b| {
            (self.cmp_value)(
                a.get::<T>().expect("should have been typechecked"),
                b.get::<T>().expect("should have been typechecked"),
            )
        });
    }

    fn sort_nodes(&self, nodes: &mut Vec<Box<dyn Node>>) {
        let mut keyed: Vec<(Option<T>, Box<dyn Node>)> =
            nodes.drain(..).map(|n| (item_value::<T>(&*n), n)).collect();
        keyed.sort_by(|(a, _), (b, _)| match (a, b) {
            (Some(a), Some(b)) => (self.cmp_value)(a, b),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        });
        nodes.extend(keyed.into_iter().map(|(_, n)| n));
    }

    fn dedup_nodes(&self, nodes: &mut Vec<Box<dyn Node>>) {
        let mut keyed: Vec<(Option<T>, Box<dyn Node>)> =
            nodes.drain(..).map(|n| (item_value::<T>(&*n), n)).collect();
        retain_first(
            &mut keyed,
            |(a, _), (b, _)| matches!((a, b), (Some(a), Some(b)) if a == b),
        );
        nodes.extend(keyed.into_iter().map(|(_, n)| n));
    }

    fn position(&self, nodes: &[Box<dyn Node>], candidate: &dyn Node) -> Option<usize> {
        let value = item_value::<T>(candidate)?;
        nodes
            .iter()
            .position(|node| item_value::<T>(&**node).is_some_and(|item| item == value))
    }
}

#[derive(Clone, Debug)]
pub struct SetNode {
    pub name: String,
    pub widget: Option<AnySetWidget>,
    pub label: Option<String>,
    /// Its hint (`help_text:`), under the legend.
    pub help_text: Option<String>,
    /// Its own check (`validator:`), over the whole collection once every
    /// item is valid; its messages go under the legend with the list's own.
    pub validator: Option<crate::nodes::FieldValidator>,
    pub optional: bool,
    pub adder: ItemAdder,
    pub items: Items,
    /// The blank item the adder renders, which Add makes an item
    /// (`SETS_PLAN.md` box 7). Never one of `items`: under a key of its own,
    /// off the wire, and never validated with the form.
    pub draft: Box<dyn Node>,
    /// The name (`#3`) of the item being edited, whose value the draft holds
    /// (`SETS_PLAN.md` box 8). A name, not a position: Add sorts the set, and
    /// a name stays with its item. The item stays in the set until Add
    /// replaces it, so a submit while editing loses nothing.
    pub editing: Option<String>,
    /// What shows each item's text (`item:`, box 9).
    pub item_view: ItemView,
    pub comparator: SetOrder,
    /// What the set's last edit did, or why it was refused, for its live
    /// region. Every edit to the set replaces it.
    pub message: Option<SetMessage>,
    /// The dev's words for what an edit did (`feedback:`), or `None` for
    /// formoxus's own.
    pub feedback: Option<SetFeedback>,
}

/// What an edit to a set did, for a dev's `feedback:` to put into words
/// (decision I′). Each carries the item: the one added, the one removed, the
/// one already there that an Add would have duplicated, which equals the
/// value the user tried by `==`, or an edited item before and after (box 8).
///
/// `#[non_exhaustive]`, so a later event can join without breaking a dev's
/// `match`: give it a `_ =>` arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum SetEvent<'a, T> {
    Added(&'a T),
    Removed(&'a T),
    Duplicate(&'a T),
    /// An edited item, replaced by what the draft held.
    Changed {
        from: &'a T,
        to: &'a T,
    },
}

// `Copy` by hand, not derived, since a derive would demand `T: Copy`; every
// variant is only a reference. `Copy` also spares a dev's named
// `fn(SetEvent<'_, Tag>)` clippy's `needless_pass_by_value`.
impl<T> Clone for SetEvent<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SetEvent<'_, T> {}

/// A [`SetEvent`] before its items are rebuilt as `T`s: their nodes.
#[derive(Clone, Copy)]
pub(crate) enum NodeEvent<'a> {
    Added(&'a dyn Node),
    Removed(&'a dyn Node),
    Duplicate(&'a dyn Node),
    Changed {
        from: &'a dyn Node,
        to: &'a dyn Node,
    },
}

/// A dev's `feedback:`, with its item type erased: what the spec holds.
///
/// A closure, not a `fn`, so it can capture app state, such as the handle of
/// a toaster to send the message to instead (returning `None`, so the set's
/// own region stays quiet). `Send + Sync`, which a captured Dioxus `Signal`
/// is, so a spec can live in a server function.
#[derive(Clone)]
pub struct SetFeedback(Arc<DescribeFn>);

/// A dev's `feedback:` once its `T` is erased: the event, with the nodes to
/// rebuild each `T` from.
type DescribeFn = dyn Fn(NodeEvent<'_>) -> Option<String> + Send + Sync;

impl SetFeedback {
    /// Words for each [`SetEvent`] over items of type `T`: the line for the
    /// set's live region, or `None` for nothing there.
    pub fn new<T: Facet<'static> + 'static>(
        describe: impl Fn(SetEvent<'_, T>) -> Option<String> + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(move |event| {
            // An item that cannot be built (an invalid one from the wire) has
            // no `T` to describe, so no message.
            match event {
                NodeEvent::Added(item) => describe(SetEvent::Added(&item_value::<T>(item)?)),
                NodeEvent::Removed(item) => describe(SetEvent::Removed(&item_value::<T>(item)?)),
                NodeEvent::Duplicate(item) => {
                    describe(SetEvent::Duplicate(&item_value::<T>(item)?))
                }
                NodeEvent::Changed { from, to } => describe(SetEvent::Changed {
                    from: &item_value::<T>(from)?,
                    to: &item_value::<T>(to)?,
                }),
            }
        }))
    }

    fn describe(&self, event: NodeEvent<'_>) -> Option<String> {
        (self.0)(event)
    }
}

impl Debug for SetFeedback {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("SetFeedback(..)")
    }
}

/// The same closure, as `SetOrder` compares its comparator: a closure has no
/// `==` of its own.
impl PartialEq for SetFeedback {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// A line for a set's live region (box 6): what its last edit did, or why it
/// was refused.
#[derive(Clone, Debug, PartialEq)]
pub struct SetMessage {
    words: Words,
    /// A refusal (a duplicate), which renders with the slot's invalid class.
    pub refused: bool,
}

/// A message's words: a dev's, said already, or formoxus's own, with the
/// items still to be named, since an item's text can wait on the label case
/// ([`Shown`]), which is known when the set renders, not when it is edited.
#[derive(Clone, Debug, PartialEq)]
enum Words {
    Said(String),
    Added(Shown),
    Removed(Shown),
    Duplicate(Shown),
    Changed { from: Shown, to: Shown },
}

impl SetMessage {
    /// The line, with its items named in `case`.
    pub fn text(&self, case: LabelCase) -> String {
        match &self.words {
            Words::Said(text) => text.clone(),
            Words::Added(item) => format!("“{}” was added successfully.", item.in_case(case)),
            Words::Removed(item) => format!("“{}” was removed successfully.", item.in_case(case)),
            Words::Duplicate(item) => format!("“{}” is already there.", item.in_case(case)),
            Words::Changed { from, to } => format!(
                "“{}” was changed to “{}” successfully.",
                from.in_case(case),
                to.in_case(case)
            ),
        }
    }
}

impl SetNode {
    /// A set widget's `chosen`: what each item now in the set sends at its own
    /// path (`colors.#0` → `"Red"`), its [`own_entry`], in item order, so
    /// position `i` is the index a `RemoveItem` for that item takes. `path` is
    /// the set's own, for the panic.
    ///
    /// Each item is asked on its own, not the whole set through
    /// `collect_values`, which would bring the draft along too.
    ///
    /// Read from the tree, never from the value store: `render` runs in the
    /// form's scope, so a store read here would make every keystroke anywhere
    /// re-render the whole form. The tree is current for a set widget's items,
    /// because they change only by `AddItem`/`RemoveItem`, which write both.
    fn chosen(&self, path: &NodePath) -> Vec<String> {
        self.items
            .iter()
            .map(|item| {
                own_entry(&**item).unwrap_or_else(|| {
                    panic!(
                        "{}: an item of a set with a widget sends no value at its own \
                         path; the set's item type should have been refused for a set \
                         widget",
                        path.join(&item.name())
                    )
                })
            })
            .collect()
    }

    /// One item's `<li>`, keyed by its name. Shown, not edited in place
    /// (decision E): its text through the item view, then its own errors (a
    /// dev's validator or a server can still name one), then Edit, where the
    /// adder shows the draft Edit fills, and Remove.
    fn render_item(
        &self,
        ctx: &NodeContext,
        item: &dyn Node,
        index: usize,
        editable: bool,
    ) -> Element {
        // The set's own path: the item buttons' edits are addressed to it.
        let path = ctx.path(&self.name);
        let name = item.name();
        let text = item_text(item, ctx.label_case);
        let editing = self.editing.as_deref() == Some(name.as_str());
        let mut paths = Vec::new();
        item.paths(&path, Drafts::Exclude, &mut paths);
        rsx! {
            li { class: ctx.classes.fixed_class(FixedKey::SetItem, false), key: "{name}",
                {
                    DynamicNode::Component(VComponent::new(
                        self.item_view.0,
                        SetItemProps {
                            text: text.clone(),
                            path: path.join(&name),
                            index,
                            editing,
                            values: ctx.values,
                            classes: ctx.classes.clone(),
                        },
                        "ItemView",
                    ))
                }
                FieldErrors {
                    field_id: ctx.nested(&self.name).id(&name),
                    paths,
                    errors: ctx.errors,
                    classes: ctx.classes.clone(),
                }
                if editable {
                    EditItemButton {
                        path: path.clone(),
                        index,
                        text: text.clone(),
                        editing,
                        on_edit: ctx.on_edit,
                        classes: ctx.classes.clone(),
                    }
                }
                RemoveItemButton {
                    path,
                    index,
                    text,
                    disabled: self.items.at_min(),
                    on_edit: ctx.on_edit,
                    classes: ctx.classes.clone(),
                }
            }
        }
    }

    /// Set the duplicate message and say `true` if `candidate` equals an
    /// item; `false`, with nothing changed, if it does not.
    fn refuse_duplicate(&mut self, candidate: &dyn Node) -> bool {
        let Some(at) = self.items.position_of(&self.comparator, candidate) else {
            return false;
        };
        self.message = self
            .items
            .iter()
            .nth(at)
            .and_then(|item| self.message_for(NodeEvent::Duplicate(&**item)));
        true
    }

    /// Start editing the item at `index`, or with `None` stop (box 8), and
    /// return the draft's path, whose values `apply_edit` then copies into
    /// the store. Either way the draft is rebuilt under the key it holds:
    /// blank to stop, and to start, filled with the item's values, moved from
    /// the item's path to the draft's. Whatever was typed into it goes.
    fn edit_item(
        &mut self,
        my_path: &NodePath,
        index: Option<usize>,
        specs: &SpecsByPath,
        errors: &mut ErrorsByPath,
    ) -> Result<NodePath, FormAccessError> {
        self.forget_draft_errors(my_path, errors);
        let draft_name = self.draft.name();
        let draft_path = my_path.join(&draft_name);
        let mut draft = self.items.new_named(my_path, &draft_name, specs, None);
        self.editing = match index {
            None => None,
            Some(index) => {
                let item = self.items.iter().nth(index).ok_or_else(|| {
                    FormAccessError(format!(
                        "cannot edit item at index {index} of {my_path}: it has {} items",
                        self.items.len()
                    ))
                })?;
                let item_path = my_path.join(&item.name());
                let mut values = Vec::new();
                item.collect_values(my_path, &mut values);
                let moved: ValuesByPath = values
                    .into_iter()
                    .map(|value| {
                        let (path, data) = value.into_pair();
                        let moved_path = path
                            .replace_prefix(&item_path, &draft_path)
                            .expect("ever path under the item should start with its own path");
                        (moved_path, data)
                    })
                    .collect();
                draft.distribute_values(my_path, specs, &moved);
                Some(item.name())
            }
        };
        self.draft = draft;
        Ok(draft_path)
    }

    fn forget_draft_errors(&self, my_path: &NodePath, out: &mut ErrorsByPath) {
        let draft_path = my_path.join(&self.draft.name());
        out.retain(|path, _| !path.starts_with(&draft_path));
    }

    /// Make the draft an item, if it can be one (`SETS_PLAN.md` 7c), and
    /// return the item's path. Its values were distributed from the store
    /// just before, by `apply_edit`.
    ///
    /// 1. Empty: nothing happens, and nothing is said, so Enter in an empty
    ///    box does not scold.
    /// 2. Invalid: refused, its error shown by the draft's own widget, the
    ///    text left to fix.
    /// 3. A duplicate: refused, with the duplicate message, the text left.
    ///    Equal to the item being edited is not a duplicate: it is that item
    ///    saved unchanged.
    /// 4. Otherwise it becomes an item under the key it has, so its leaves in
    ///    the store are already at the item's path, and a new blank draft
    ///    takes the next key. If an item was being edited (box 8), it goes in
    ///    the same edit, so the set never holds both.
    fn commit_draft(
        &mut self,
        my_path: &NodePath,
        specs: &SpecsByPath,
        errors: &mut ErrorsByPath,
    ) -> Option<NodePath> {
        self.forget_draft_errors(my_path, errors);
        if !self.draft.is_present() || !self.draft.validate(my_path, errors) {
            return None;
        }

        let candidate = self.draft.clone_box();
        let equal = self
            .items
            .position_of(&self.comparator, &*candidate)
            .and_then(|at| self.items.iter().nth(at))
            .map(|item| item.name());
        let unchanged = equal.is_some() && equal == self.editing;
        if equal.is_some() && !unchanged {
            self.refuse_duplicate(&*candidate);
            return None;
        }
        let fresh = self.items.new_draft(my_path, specs);
        let item = std::mem::replace(&mut self.draft, fresh);
        let added = my_path.join(&item.name());
        let replaced = self
            .editing
            .take()
            .and_then(|name| self.items.remove_named(&name));
        self.message = match &replaced {
            // Saved as it was: nothing to say.
            Some(_) if unchanged => None,
            Some(from) => self.message_for(NodeEvent::Changed {
                from: &**from,
                to: &*item,
            }),
            None => self.message_for(NodeEvent::Added(&*item)),
        };
        self.items.insert_node(item);
        self.items.sort_by(&self.comparator);
        Some(added)
    }

    /// The line for the live region after `event`: the dev's `feedback:` if
    /// there is one, else formoxus's own words. A duplicate is a refusal, so
    /// it renders with the slot's invalid class.
    fn message_for(&self, event: NodeEvent<'_>) -> Option<SetMessage> {
        let words = match &self.feedback {
            Some(feedback) => Words::Said(feedback.describe(event)?),
            None => default_words(event)?,
        };
        Some(SetMessage {
            words,
            refused: matches!(event, NodeEvent::Duplicate(_)),
        })
    }
}

/// formoxus's own words for what an edit did, Django's way (I′), or `None` for
/// an item with nothing to name. Worded by [`SetMessage::text`].
fn default_words(event: NodeEvent<'_>) -> Option<Words> {
    let named = |item: &dyn Node| Some(item.shown()).filter(|shown| !shown.is_empty());
    Some(match event {
        NodeEvent::Added(item) => Words::Added(named(item)?),
        NodeEvent::Removed(item) => Words::Removed(named(item)?),
        NodeEvent::Duplicate(item) => Words::Duplicate(named(item)?),
        NodeEvent::Changed { from, to } => Words::Changed {
            from: named(from)?,
            to: named(to)?,
        },
    })
}

/// An item as the user sees it, shown and named in its buttons:
/// [`Node::shown`] in the form's label case, so a choice's label where the
/// item has choices and a variant cased as its `<select>` shows it, never a
/// `Display` the type may not have.
fn item_text(item: &dyn Node, case: LabelCase) -> String {
    item.shown().in_case(case)
}

impl Node for SetNode {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self, case: LabelCase) -> Option<String> {
        self.label
            .clone()
            .or_else(|| default_label(&self.name, case))
    }

    fn raw_value(&self) -> String {
        String::new()
    }

    fn collect_values(&self, prefix: &NodePath, out: &mut Vec<RawValue>) {
        let my_path = prefix.join(&self.name);
        self.items.collect_values(&my_path, out);
        // Its leaves as drafts, so the store holds them but the wire does not.
        // Its structure (a unit enum draft's chosen variant) not at all: like
        // all structure it lives in the state, and only the state needs it.
        // In the store, `apply_edit`'s sync would read it back as `""` and
        // unchoose what the user picked.
        let mut draft = Vec::new();
        self.draft.collect_values(&my_path, &mut draft);
        out.extend(draft.into_iter().filter_map(|value| match value {
            RawValue::Leaf { path, data } => Some(RawValue::Draft { path, data }),
            RawValue::Structure { .. } | RawValue::Draft { .. } => None,
        }));
    }

    fn distribute_values(&mut self, prefix: &NodePath, specs: &SpecsByPath, values: &ValuesByPath) {
        let my_path = prefix.join(&self.name);
        self.items.distribute_values(&my_path, specs, values);
        // A server's draft was built before the wire's keys arrived, so one
        // of them can be its key. Nothing was typed into a server's draft, so
        // a fresh blank one loses nothing.
        if self.items.has_item_named(&self.draft.name()) {
            self.draft = self.items.new_draft(&my_path, specs);
        }
        self.draft.distribute_values(&my_path, specs, values);
        self.items.sort_by(&self.comparator);
        self.items.dedup(&self.comparator);
    }

    fn validate(&self, prefix: &NodePath, out: &mut ErrorsByPath) -> bool {
        let my_path = prefix.join(&self.name);
        let mut is_valid = self.items.validate(&my_path, out);
        if is_valid {
            let errors = self
                .validator
                .as_ref()
                .map(|validator| validator.messages_for(self))
                .unwrap_or_default();
            is_valid = errors.is_empty();
            out.insert(my_path, errors);
        }
        is_valid
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }

    fn write_value_into<'p>(&self, partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        let partial = match partial.shape().def {
            Def::Set(_) => {
                let mut partial = partial.init_set()?;
                for item in &self.items {
                    partial = partial.begin_set_item()?;
                    partial = item.write_value_into(partial)?;
                    partial = partial.end()?;
                }
                partial
            }
            Def::List(_) => {
                let mut partial = partial.init_list()?;
                for item in &self.items {
                    partial = partial.begin_list_item()?;
                    partial = item.write_value_into(partial)?;
                    partial = partial.end()?;
                }
                partial
            }
            _ => panic!(
                "{}: a set writes into a set or a list, not `{}`",
                self.name,
                partial.shape()
            ),
        };
        Ok(partial)
    }

    fn is_present(&self) -> bool {
        self.items.is_present()
    }

    fn edit(
        &mut self,
        prefix: &NodePath,
        edit: &Edit,
        specs: &SpecsByPath,
        errors: &mut ErrorsByPath,
    ) -> Result<Option<NodePath>, FormAccessError> {
        let path = edit.path();
        let my_path = prefix.join(&self.name);
        path.ensure_starts_with(&my_path)?;
        if *path == my_path {
            // A message answers the last edit only: whatever the set does next,
            // it is stale.
            self.message = None;
            return match edit {
                Edit::AddItem { value, .. } => match value {
                    // "Add what the adder holds": for a set, the draft.
                    None => Ok(self.commit_draft(&my_path, specs, errors)),
                    Some(v) => {
                        // Refused for the user, not the caller (box 6): the set is
                        // unchanged and its live region says why, so `Ok`, not an
                        // error for `use_form` to log. Named by the item already
                        // there, which the user can see.
                        let probe = self.items.probe(&my_path, specs, v);
                        if let Some(probe) = probe
                            && self.refuse_duplicate(&*probe)
                        {
                            return Ok(None);
                        }
                        let added = self.items.add_item(&my_path, None, specs, Some(v))?;
                        self.items.sort_by(&self.comparator);
                        self.message = self
                            .items
                            .iter()
                            .find(|item| my_path.join(&item.name()) == added)
                            .and_then(|item| self.message_for(NodeEvent::Added(&**item)));
                        Ok(Some(added))
                    }
                },
                Edit::RemoveItem { index, .. } => {
                    // Described before it goes, while there is an item to name.
                    let item = self.items.iter().nth(*index);
                    let message =
                        item.and_then(|item| self.message_for(NodeEvent::Removed(&**item)));
                    let name = item.map(|item| item.name());
                    self.items.remove_item(&my_path, *index)?;
                    // Removing the item being edited ends the edit. The draft
                    // keeps its text, which Add would now add back.
                    if name.is_some() && name == self.editing {
                        self.editing = None;
                    }
                    self.message = message;
                    Ok(None)
                }
                Edit::EditItem { index, .. } => {
                    self.edit_item(&my_path, *index, specs, errors).map(Some)
                }
                Edit::ChooseVariant { .. } => {
                    Err(FormAccessError(format!("{my_path} is a set, not an enum")))
                }
                Edit::MoveItem { .. } => Err(FormAccessError(format!(
                    "{my_path} is a set; you cannot reorder its items"
                ))),
            };
        }

        // An edit inside the draft, such as a unit enum draft's choice.
        if path.starts_with(&my_path.join(&self.draft.name())) {
            return self.draft.edit(&my_path, edit, specs, errors);
        }
        self.items.edit_within(&my_path, edit, specs, errors)
    }

    fn render(&self, ctx: &NodeContext) -> Element {
        if let Some(widget) = &self.widget {
            let path = ctx.path(&self.name);
            let chosen = self.chosen(&path);
            return widget.render(SetWidgetProps {
                common: WidgetCommon {
                    id: ctx.id(&self.name),
                    path,
                    label: self.label(ctx.label_case),
                    help_text: self.help_text.clone(),
                    errors: ctx.errors,
                    classes: ctx.classes.clone(),
                },
                choices: self
                    .choices(ctx.label_case)
                    .expect("a SetWidget should have had choices"),
                chosen,
                required: self.items.min_items().is_some_and(|min| min > 0),
                max_items: self.items.max_items(),
                on_edit: ctx.on_edit,
            });
        }

        // A `fieldset` for the same reason `StructNode` and `VariantNode` use one:
        // the items and the widgets that manage them read as one thing, and the
        // legend gives the list's label somewhere to appear.
        //
        // No `aria-labelledby`: a `<legend>` names its own fieldset, and unlike
        // `VariantNode` there is no control inside that needs naming too. No
        // `aria-invalid` either: a bare fieldset is role `group`, where ARIA
        // leaves it undefined (see `RadioGroupView`). So the list's own errors
        // reach assistive technology through `aria-describedby`, and CSS through
        // the fieldset's invalid class.
        //
        // No required marker, unlike `VariantNode`: nothing enforces a minimum item
        // count yet (#14), so marking a set required would be a promise
        // `validate()` doesn't keep.
        let id = ctx.id(&self.name);
        // The set's own path — where an `AddItem`/`RemoveItem` is addressed. NOT
        // `nested.prefix`'s children: the widgets act on the list, not on an item.
        let path = ctx.path(&self.name);
        let invalid = ctx.errors.validity_at(&path) == Validity::Invalid;
        let describedby = described_by(&id, self.help_text.is_some(), invalid);

        let fieldset_class = ctx.classes.fixed_class(FixedKey::Set, invalid);
        let legend_class = ctx.classes.fixed_class(FixedKey::SetLegend, false);
        let class_for_set_items = ctx.classes.fixed_class(FixedKey::SetItems, false);
        let refused = self.message.as_ref().is_some_and(|m| m.refused);
        let message_class = ctx.classes.fixed_class(FixedKey::SetMessage, refused);
        let nested = ctx.nested(&self.name);
        // Not required, whatever the set is: an empty draft is a set with
        // nothing being added, and a `required` box would block the submit.
        // Named for the set, since items have no labels of their own.
        let mut draft_ctx = nested.optional();
        draft_ctx.in_draft = true;
        draft_ctx.label_if_unnamed = Some(match self.label(ctx.label_case) {
            Some(label) => format!("Add to {label}"),
            None => "Add".to_string(),
        });
        let draft = self.draft.render(&draft_ctx);
        // Edit only beside the adder that renders the draft it fills: a
        // dev's adder may leave the draft out (a search-to-add), and Edit
        // would then fill a box nobody can see.
        let editable = self.adder == ItemAdder(DraftAdder);
        rsx! {
            fieldset {
                class: fieldset_class,
                id: "{id}",
                aria_describedby: describedby,
                if let Some(text) = self.label(ctx.label_case) {
                    legend { class: legend_class,
                        "{text}"
                        if self.items.min_items().is_some_and(|min| min > 0) {
                            {required_marker(&ctx.classes)}
                        }
                    }
                }
                {field_help(&id, self.help_text.as_deref(), &ctx.classes)}
                // Right after the legend, so the list's own messages are also
                // read in order straight after its name: support for
                // `aria-describedby` on a group varies between screen readers.
                FieldErrors {
                    field_id: id.clone(),
                    paths: vec![path.clone()],
                    errors: ctx.errors,
                    classes: ctx.classes.clone(),
                }
                if !self.items.is_empty() {
                    ul { class: &class_for_set_items,
                        // Keyed by the items's own name, which is stable across
                        // inserts and removals. Without a key dioxus diffs
                        // the list by position, so inserting at the top would
                        // re-render every item below it; with one it moves the nodes
                        // it already has. The wrapper exists because a key has to sit
                        // on an element, and it is what pairs a item with its widget.
                        for (index, item) in self.items.iter().enumerate() {
                            { self.render_item(ctx, &**item, index, editable) }
                        }
                    },
                }
                {
                    DynamicNode::Component(VComponent::new(
                        self.adder.0,
                        AddItemProps {
                            path: path.clone(),
                            id: id.clone(),
                            values: ctx.values,
                            on_edit: ctx.on_edit,
                            classes: ctx.classes.clone(),
                            item_count: self.items.len(),
                            max_items: self.items.max_items(),
                            draft: Some(draft),
                            editing: self.editing.as_ref().and_then(|name| {
                                self.items
                                    .iter()
                                    .find(|item| &item.name() == name)
                                    .map(|item| item_text(&**item, ctx.label_case))
                            }),
                        },
                        "ItemAdder",
                    ))
                }
                // Always present, empty until an edit: a `status` region is
                // announced reliably only if it is already in the page when its
                // text changes. After the adder, whatever adder the dev gave, so
                // it is read where the user just acted.
                div {
                    class: message_class,
                    role: "status",
                    if let Some(message) = &self.message {
                        "{message.text(ctx.label_case)}"
                    }
                }
            }
        }
    }

    fn paths(&self, prefix: &NodePath, drafts: Drafts, out: &mut Vec<NodePath>) {
        let my_path = prefix.join(&self.name);
        out.push(my_path.clone());
        self.items.paths(&my_path, drafts, out);
        if drafts == Drafts::Include {
            self.draft.paths(&my_path, drafts, out);
        }
    }

    fn absent_owner(&self, prefix: &NodePath, path: &NodePath) -> Option<NodePath> {
        let nested = prefix.join(&self.name);
        if path.starts_with(&nested) {
            self.items
                .iter()
                .find_map(|n| n.absent_owner(&nested, path))
        } else {
            None
        }
    }

    fn choices(&self, case: LabelCase) -> Option<Vec<Choice>> {
        self.draft.choices(case)
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use googletest::prelude::*;

    use super::{ascii_case_insensitive, unicode_case_insensitive};

    /// Plain `str::cmp` puts every capital before every lowercase letter,
    /// so "Banana" before "apple"; both of these ignore case for that.
    #[gtest]
    fn case_does_not_decide_the_order() {
        for cmp in [
            ascii_case_insensitive::<&str>,
            unicode_case_insensitive::<&str>,
        ] {
            let mut words = vec!["cherry", "Banana", "apple"];
            words.sort_by(cmp);
            expect_that!(
                words,
                elements_are![eq(&"apple"), eq(&"Banana"), eq(&"cherry")]
            );
        }
    }

    /// A set may hold both "Rust" and "rust", and `Equal` would leave them in
    /// arrival order, which for a `HashSet` differs between server and client.
    /// So only the same string is `Equal`; otherwise plain `str::cmp` decides.
    #[gtest]
    fn strings_differing_only_in_case_are_never_equal() {
        for cmp in [
            ascii_case_insensitive::<&str>,
            unicode_case_insensitive::<&str>,
        ] {
            expect_that!(cmp(&"Rust", &"rust"), eq(Ordering::Less));
            expect_that!(cmp(&"rust", &"Rust"), eq(Ordering::Greater));
            expect_that!(cmp(&"rust", &"rust"), eq(Ordering::Equal));
        }
    }

    /// The difference between the two: only `unicode_` folds "É" to "é", so
    /// it compares "élan" and "Été" at their second letter. `ascii_` leaves
    /// "é" and "É" as two different letters, and "É" comes first.
    #[gtest]
    fn only_unicode_folds_case_beyond_ascii() {
        expect_that!(
            unicode_case_insensitive(&"élan", &"Été"),
            eq(Ordering::Less)
        );
        expect_that!(
            ascii_case_insensitive(&"élan", &"Été"),
            eq(Ordering::Greater)
        );
    }

    /// Full case folding, not lowercasing: "ß" folds to "ss", so "Straße" and
    /// "STRASSE" are the same word, and only the tie-break orders them ("t"
    /// after "T"). Lowercasing would compare "straße" with "strasse" and put
    /// "ß" after "s"; "STRASSF" shows the fold, not the tie-break, decided.
    #[gtest]
    fn unicode_folds_eszett_to_ss() {
        expect_that!(
            unicode_case_insensitive(&"Straße", &"STRASSE"),
            eq(Ordering::Greater)
        );
        expect_that!(
            unicode_case_insensitive(&"Straße", &"STRASSF"),
            eq(Ordering::Less)
        );
    }

    /// Generic over `AsRef<str>`, so `form!`'s `sort_by:` (4c) can hand
    /// either one to a set of any string-like type, its `T` inferred from
    /// the field.
    #[gtest]
    fn either_fits_a_set_of_any_string_like_type() {
        let _: fn(&String, &String) -> Ordering = unicode_case_insensitive;
        let _: fn(&Box<str>, &Box<str>) -> Ordering = ascii_case_insensitive;
        expect_that!(
            unicode_case_insensitive(&"b".to_string(), &"A".to_string()),
            eq(Ordering::Greater)
        );
    }
}
