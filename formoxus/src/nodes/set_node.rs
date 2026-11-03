//! A set-typed node. Items are nodes named by a `#`-prefixed KEY, which is
//! what makes `answer_choices.#0.text` fall out of the ordinary `qualify`
//! nesting — and, because a key is an identity rather than a position, what lets
//! an item be inserted anywhere.
//!
//! Items are kept sorted by a [`SetOrder`]. A set of scalars (`String`, the
//! integers, `bool`, the floats) has a default order; any other item type
//! needs one given, by `sort_by:` in `form!` (for an `Ord` type,
//! `sort_by: Tag::cmp`), and [`SetOrder::default_for`] panics without it.

use std::{
    cmp::Ordering,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use dioxus::{
    core::{DynamicNode, VComponent},
    prelude::*,
};
use facet::{Def, Facet, Partial, Peek, ReflectError, ScalarType, Shape};
use formoxus_spec::FixedKey;
use unicase::{Ascii, UniCase};

use crate::{
    build::retain_first,
    error::FormAccessError,
    form::ErrorsByPath,
    nodes::{
        Edit, Items, Node, NodeContext, SpecsByPath, ValuesByPath, default_label, ensure_owned,
        qualify,
    },
    widgets::{AddItemProps, FieldErrors, ItemAdder, RemoveItemButton, errors::errors_id},
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
    /// `sort_by` was given. Only scalar items have one.
    pub fn default_for(shape: &'static Shape, path: &str) -> Self {
        if let Some(scalar_type) = shape.scalar_type() {
            let default_order = DefaultOrder::for_type(scalar_type);
            SetOrder {
                item_order: Arc::new(default_order),
            }
        } else {
            panic!(
                "{path}: a set of `{shape}` has no default order; give it a sort_by \
                 (only a set of scalars sorts without one)"
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

    /// Where an item equal to `value` sits in `nodes`, by the model's own
    /// `PartialEq` (decision H), for refusing a duplicate `AddItem` (box 6).
    /// An item with no value equals nothing; so does a `value` of another
    /// type, which `Items::add_item` then refuses for its type.
    fn position(&self, nodes: &[Box<dyn Node>], value: Peek<'_, 'static>) -> Option<usize>;
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

    fn position(&self, nodes: &[Box<dyn Node>], value: Peek<'_, 'static>) -> Option<usize> {
        nodes
            .iter()
            .position(|node| node.peek_value().is_some_and(|item| item == value))
    }
}

struct CustomOrder<T> {
    cmp_value: fn(&T, &T) -> Ordering,
    name: String,
}

/// Rebuild an item's `T` from its node, or `None` for an item that has no
/// value to build (an `Empty` or `Invalid` leaf). For what needs a typed item
/// and has only the node: a custom order, and a dev's `feedback`.
fn item_value<T: Facet<'static>>(node: &dyn Node) -> Option<T> {
    let mut probe = node.clone_box();
    probe.validate();
    if probe.has_errors_within() {
        return None;
    }
    let partial = Partial::alloc::<T>().expect("alloc should never fail for a concrete T");
    probe
        .write_value_into(partial)
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

    fn position(&self, nodes: &[Box<dyn Node>], value: Peek<'_, 'static>) -> Option<usize> {
        let value = value.get::<T>().ok()?;
        nodes
            .iter()
            .position(|node| item_value::<T>(&**node).is_some_and(|item| &item == value))
    }
}

#[derive(Clone, Debug)]
pub struct SetNode {
    pub name: String,
    pub label: Option<String>,
    pub optional: bool,
    pub adder: ItemAdder,
    pub items: Items,
    pub comparator: SetOrder,
    /// What the set's last edit did, or why it was refused, for its live
    /// region. Every edit to the set replaces it.
    pub message: Option<SetMessage>,
    /// The dev's words for what an edit did (`feedback:`), or `None` for
    /// formoxus's own.
    pub feedback: Option<SetFeedback>,
}

/// What an edit to a set did, for a dev's `feedback:` to put into words
/// (decision I′). Each carries the item: the one added, the one removed, or
/// the one already there that an Add would have duplicated, which equals the
/// value the user tried by `==`.
///
/// `#[non_exhaustive]`, so editing an item (box 8) can join without breaking
/// a dev's `match`: give it a `_ =>` arm.
#[derive(Debug)]
#[non_exhaustive]
pub enum SetEvent<'a, T> {
    Added(&'a T),
    Removed(&'a T),
    Duplicate(&'a T),
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

/// Which [`SetEvent`] happened, before the item is rebuilt as a `T`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetEventKind {
    Added,
    Removed,
    Duplicate,
}

/// A dev's `feedback:`, with its item type erased: what the spec holds.
///
/// A closure, not a `fn`, so it can capture app state, such as the handle of
/// a toaster to send the message to instead (returning `None`, so the set's
/// own region stays quiet). `Send + Sync`, which a captured Dioxus `Signal`
/// is, so a spec can live in a server function.
#[derive(Clone)]
pub struct SetFeedback(Arc<DescribeFn>);

/// A dev's `feedback:` once its `T` is erased: the event, and the item's node
/// to rebuild a `T` from.
type DescribeFn = dyn Fn(SetEventKind, &dyn Node) -> Option<String> + Send + Sync;

impl SetFeedback {
    /// Words for each [`SetEvent`] over items of type `T`: the line for the
    /// set's live region, or `None` for nothing there.
    pub fn new<T: Facet<'static> + 'static>(
        describe: impl Fn(SetEvent<'_, T>) -> Option<String> + Send + Sync + 'static,
    ) -> Self {
        Self(Arc::new(move |kind, item| {
            // An item that cannot be built (until box 8, one typed into in
            // place) has no `T` to describe, so no message.
            let value = item_value::<T>(item)?;
            describe(match kind {
                SetEventKind::Added => SetEvent::Added(&value),
                SetEventKind::Removed => SetEvent::Removed(&value),
                SetEventKind::Duplicate => SetEvent::Duplicate(&value),
            })
        }))
    }

    fn describe(&self, kind: SetEventKind, item: &dyn Node) -> Option<String> {
        (self.0)(kind, item)
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
    pub text: String,
    /// A refusal (a duplicate), which renders with the slot's invalid class.
    pub refused: bool,
}

impl SetNode {
    /// The line for the live region after `kind` happened to `item`: the
    /// dev's `feedback:` if there is one, else formoxus's own words. A
    /// duplicate is a refusal, so it renders with the slot's invalid class.
    fn message_for(&self, kind: SetEventKind, item: &dyn Node) -> Option<SetMessage> {
        let text = match &self.feedback {
            Some(feedback) => feedback.describe(kind, item),
            None => default_message(kind, &item_text(item)),
        }?;
        Some(SetMessage {
            text,
            refused: kind == SetEventKind::Duplicate,
        })
    }
}

/// formoxus's own words for what an edit did, Django's way (I′), or `None` for
/// an item with nothing to name.
fn default_message(kind: SetEventKind, text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    Some(match kind {
        SetEventKind::Added => format!("“{text}” was added successfully."),
        SetEventKind::Removed => format!("“{text}” was removed successfully."),
        SetEventKind::Duplicate => format!("“{text}” is already there."),
    })
}

/// An item as the user sees it, for a message about it: its values joined,
/// so a string's text, a newtype's inner value, a unit enum's variant name
/// and each field of a struct, never a `Display` the type may not have.
fn item_text(item: &dyn Node) -> String {
    let mut values = Vec::new();
    item.collect_values("", &mut values);
    values
        .into_iter()
        .map(|value| value.into_pair().1)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

impl Node for SetNode {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self, case: crate::label_case::LabelCase) -> Option<String> {
        self.label
            .clone()
            .or_else(|| default_label(&self.name, case))
    }

    fn raw_value(&self) -> String {
        String::new()
    }

    fn collect_values(&self, prefix: &str, out: &mut Vec<super::RawValue>) {
        self.items.collect_values(&qualify(prefix, &self.name), out);
    }

    fn distribute_values(&mut self, prefix: &str, specs: &SpecsByPath, values: &ValuesByPath) {
        self.items
            .distribute_values(&qualify(prefix, &self.name), specs, values);
        self.items.sort_by(&self.comparator);
        self.items.dedup(&self.comparator);
    }

    fn collect_errors(&self, prefix: &str, out: &mut ErrorsByPath) {
        self.items.collect_errors(&qualify(prefix, &self.name), out);
    }

    fn distribute_errors(&mut self, prefix: &str, errors: &mut ErrorsByPath) {
        self.items
            .distribute_errors(&qualify(prefix, &self.name), errors);
    }

    fn validate(&mut self) {
        self.items.validate();
    }

    fn has_errors_within(&self) -> bool {
        self.items.has_errors_within()
    }

    fn has_errors_here(&self) -> bool {
        self.items.has_errors_here()
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

    fn clear_errors(&mut self) {
        self.items.clear_errors();
    }

    fn edit(
        &mut self,
        prefix: &str,
        edit: &Edit,
        specs: &SpecsByPath,
    ) -> Result<Option<String>, FormAccessError> {
        let path = edit.path();
        let my_path = qualify(prefix, &self.name);
        ensure_owned(&my_path, path)?;
        if path == my_path {
            // A message answers the last edit only: whatever the set does next,
            // it is stale.
            self.message = None;
            return match edit {
                Edit::AddItem { value, .. } => match value {
                    None => Err(FormAccessError(format!(
                        "{my_path} is a set; you cannot add an empty item to a set"
                    ))),
                    Some(v) => {
                        // Refused for the user, not the caller (box 6): the set is
                        // unchanged and its live region says why, so `Ok`, not an
                        // error for `use_form` to log. Named by the item already
                        // there, which the user can see.
                        if let Some(at) = self.items.position_of(&self.comparator, v) {
                            self.message = self.items.iter().nth(at).and_then(|item| {
                                self.message_for(SetEventKind::Duplicate, &**item)
                            });
                            return Ok(None);
                        }
                        let added = self.items.add_item(&my_path, None, specs, Some(v))?;
                        self.items.sort_by(&self.comparator);
                        self.message = self
                            .items
                            .iter()
                            .find(|item| qualify(&my_path, &item.name()) == added)
                            .and_then(|item| self.message_for(SetEventKind::Added, &**item));
                        Ok(Some(added))
                    }
                },
                Edit::RemoveItem { index, .. } => {
                    // Described before it goes, while there is an item to name.
                    let message = self
                        .items
                        .iter()
                        .nth(*index)
                        .and_then(|item| self.message_for(SetEventKind::Removed, &**item));
                    self.items.remove_item(&my_path, *index)?;
                    self.message = message;
                    Ok(None)
                }
                Edit::ChooseVariant { .. } => {
                    Err(FormAccessError(format!("{my_path} is a set, not an enum")))
                }
                Edit::MoveItem { .. } => Err(FormAccessError(format!(
                    "{my_path} is a set; you cannot reorder its items"
                ))),
            };
        }

        self.items.edit_within(&my_path, edit, specs)
    }

    fn render(&self, ctx: &NodeContext) -> Element {
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
        // `nested.prefix`'s children: the widgets act on the list, not on a row.
        let path = ctx.path(&self.name);
        // `here`, not `within`: a bad cell is its item's error, not the set's.
        let invalid = self.has_errors_here();
        let describedby = invalid.then(|| errors_id(&id));

        let fieldset_class = ctx.classes.fixed_class(FixedKey::Set, invalid);
        let legend_class = ctx.classes.fixed_class(FixedKey::SetLegend, false);
        let set_item_class = ctx.classes.fixed_class(FixedKey::SetItem, false);
        let class_for_set_items = ctx.classes.fixed_class(FixedKey::SetItems, false);
        let refused = self.message.as_ref().is_some_and(|m| m.refused);
        let message_class = ctx.classes.fixed_class(FixedKey::SetMessage, refused);
        let nested = ctx.nested(&self.name);
        rsx! {
            fieldset {
                class: fieldset_class,
                id: "{id}",
                aria_describedby: describedby,
                if let Some(text) = self.label(ctx.label_case) {
                    legend { class: legend_class, "{text}" }
                }
                // Right after the legend, so the list's own messages are also
                // read in order straight after its name: support for
                // `aria-describedby` on a group varies between screen readers.
                FieldErrors {
                    field_id: id.clone(),
                    errors: self.items.errors().to_vec(),
                    classes: ctx.classes.clone(),
                }
                if !self.items.is_empty() {
                    ul { class: &class_for_set_items,
                        // Keyed by the items's own name, which is stable across
                        // inserts and removals. Without a key dioxus diffs
                        // the list by position, so inserting at the top would
                        // re-render every row below it; with one it moves the nodes
                        // it already has. The wrapper exists because a key has to sit
                        // on an element, and it is what pairs a item with its widget.
                        for (index, item) in self.items.iter().enumerate() {
                            li { class: &set_item_class, key: "{item.name()}",
                                { item.render(&nested) }
                                RemoveItemButton { path: path.clone(), index, on_edit: ctx.on_edit, classes: ctx.classes.clone() }
                            }
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
                            rows: self.items.len(),
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
                        "{message.text}"
                    }
                }
            }
        }
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
