use std::{
    any::Any,
    collections::{HashMap, HashSet},
    fmt::Debug,
    rc::Rc,
    slice::Iter,
};

use facet::{Facet, Peek, Shape};

use crate::{
    build::{FormMode, Walk, node_for_shape},
    error::{FormAccessError, ValidationMessage},
    form::ErrorsByPath,
    nodes::{Drafts, Edit, Node, RawValue, SetOrder, SpecsByPath, ValuesByPath, no_such_path},
    path::{ITEM_SIGIL, NodePath, item_segment},
};

#[derive(Clone, Debug)]
pub struct Items {
    shape: &'static Shape,
    collection: Vec<Box<dyn Node>>,
    next_key: usize,
    rejected_keys: Option<String>,
    /// `min_items:` and `max_items:` (#14), the counts `validate` holds the
    /// collection to.
    min: Option<usize>,
    max: Option<usize>,
    /// How many item keys the wire listed when that was more than
    /// `max_items`: refused before any were built, so a forged request
    /// cannot make the server build thousands of nodes to then reject.
    over_max: Option<usize>,
}

impl Items {
    pub(crate) fn new(shape: &'static Shape, collection: Vec<Box<dyn Node>>) -> Self {
        let next_key = collection.len();
        Self {
            shape,
            collection,
            next_key,
            rejected_keys: None,
            min: None,
            max: None,
            over_max: None,
        }
    }

    /// This collection, held to `min` and `max` items.
    pub(crate) fn with_limits(mut self, min: Option<usize>, max: Option<usize>) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    pub(crate) fn min_items(&self) -> Option<usize> {
        self.min
    }

    pub(crate) fn max_items(&self) -> Option<usize> {
        self.max
    }

    /// Whether one fewer item would fall below the minimum: Remove is
    /// disabled.
    pub(crate) fn at_min(&self) -> bool {
        self.min.is_some_and(|min| self.len() <= min)
    }

    pub(crate) fn collect_values(&self, my_path: &NodePath, out: &mut Vec<RawValue>) {
        let keys: Vec<String> = self.iter().map(|item| item.name()).collect();
        out.push(RawValue::Structure {
            path: my_path.clone(),
            data: keys.join(","),
        });
        for item in self {
            item.collect_values(my_path, out);
        }
    }

    pub(crate) fn distribute_values(
        &mut self,
        my_path: &NodePath,
        specs: &SpecsByPath,
        values: &ValuesByPath,
    ) {
        if let Some(raw) = values.get(my_path) {
            match parse_item_keys(raw) {
                Some(keys) if self.max.is_some_and(|max| keys.len() > max) => {
                    // Refused unbuilt: validation fails on the count anyway.
                    self.rejected_keys = None;
                    self.over_max = Some(keys.len());
                }
                Some(keys) => {
                    self.rejected_keys = None;
                    self.over_max = None;
                    let current: Vec<String> = self.iter().map(|item| item.name()).collect();
                    let wanted: Vec<String> = keys.iter().map(|&key| item_segment(key)).collect();
                    if current != wanted {
                        self.set_items(my_path, &keys, specs);
                    }
                }
                None => self.rejected_keys = Some(raw.clone()),
            }
        }
        for item in &mut *self {
            item.distribute_values(my_path, specs, values);
        }
    }

    pub(crate) fn paths(&self, prefix: &NodePath, drafts: Drafts, out: &mut Vec<NodePath>) {
        for r in &self.collection {
            r.paths(prefix, drafts, out);
        }
    }

    pub(crate) fn validate(&self, my_path: &NodePath, out: &mut ErrorsByPath) -> bool {
        let mut errors: Vec<ValidationMessage> = Vec::new();

        if let Some(raw) = &self.rejected_keys {
            errors.push(ValidationMessage(format!(
                "{raw:?} is not a list of item keys (expected e.g. \"#0,#1\")"
            )));
        }
        // The count, the collection's own error whatever its items say.
        let count = self.over_max.unwrap_or(self.len());
        if let Some(min) = self.min
            && count < min
        {
            errors.push(ValidationMessage(format!(
                "Add at least {}.",
                items_word(min)
            )));
        }
        if let Some(max) = self.max
            && count > max
        {
            errors.push(ValidationMessage(format!(
                "No more than {}.",
                items_word(max)
            )));
        }
        let mut is_valid = errors.is_empty();
        out.insert(my_path.clone(), errors);
        for r in &self.collection {
            // `validate` first, otherwise &&'s short circuit stops on error
            is_valid = r.validate(my_path, out) && is_valid;
        }
        is_valid
    }

    /// Whether the collection is there at all, which is what an `OptionNode`
    /// asks before choosing between validating it and reading it as `None`.
    ///
    /// Present when any item is, and also while `rejected_keys` or `over_max`
    /// is set. Both leave the collection empty, since neither builds items, and
    /// an empty collection reads as absent. Absent, `OptionNode::validate`
    /// would skip `validate`, the only place either becomes an error, and the
    /// bad entry would pass as `None` (#20). `VariantChoice::Unknown` counts
    /// as present for the same reason.
    pub(crate) fn is_present(&self) -> bool {
        self.rejected_keys.is_some()
            || self.over_max.is_some()
            || (!self.is_empty() && self.iter().any(|n| n.is_present()))
    }

    /// Build one item under the next key, from `value` if there is one and
    /// blank if not, and put it *before* position `at`, or at the end. A value
    /// whose type is not the item type is refused, and nothing is added. The
    /// item's prefix has to match what its container produces at construction, or
    /// the item would render fine and write to store keys nobody reads;
    /// `new_item` keeps the two the same.
    pub(crate) fn add_item(
        &mut self,
        my_path: &NodePath,
        before: Option<usize>,
        specs: &SpecsByPath,
        value: Option<&ItemValue>,
    ) -> Result<NodePath, FormAccessError> {
        if let Some(shape) = value.and_then(ItemValue::shape)
            && shape != self.shape
        {
            return Err(FormAccessError(format!(
                "cannot add a `{shape}` item to {my_path}: its items are `{}`",
                self.shape,
            )));
        }
        let len = self.collection.len();
        let at = before.unwrap_or(len);
        if at > len {
            return Err(FormAccessError(format!(
                "cannot insert at index {at} in {my_path}: it has {len} items"
            )));
        }

        let item = self.new_item(my_path, self.next_key, specs, value);
        // Taken before the item moves into `items`: its full path, which is what
        // `FormState::edit` hands back so the caller can find the item.
        let item_path = my_path.join(&item.name());
        self.next_key += 1;
        self.collection.insert(at, item);
        Ok(item_path)
    }

    /// One item under `key`, built the way construction builds one: from
    /// `value` in `Populated` mode when there is one, as `form_for` would,
    /// else blank (`None` peek, `FormMode::Blank`, the pair
    /// `VariantNode::set_variant` passes). Shared by `add_item`, which takes the
    /// next key, and `set_items`, which takes the key the wire named.
    pub(crate) fn new_item(
        &self,
        my_path: &NodePath,
        key: usize,
        specs: &SpecsByPath,
        value: Option<&ItemValue>,
    ) -> Box<dyn Node> {
        self.new_named(my_path, &item_segment(key), specs, value)
    }

    /// [`new_item`](Self::new_item) under a name already made, so a set can
    /// rebuild its draft blank under the key the draft already holds.
    pub(crate) fn new_named(
        &self,
        my_path: &NodePath,
        name: &str,
        specs: &SpecsByPath,
        value: Option<&ItemValue>,
    ) -> Box<dyn Node> {
        // A value makes this one item a `form_for` of its own: read from it, and
        // in `Populated` mode, so a `None` inside it means "absent" (answered),
        // not "nothing to read" as it does in a blank item.
        let (peek, mode) = match value {
            Some(value) if value.peek().is_some() => (value.peek(), FormMode::Populated),
            _ => (None, FormMode::Blank),
        };
        let mut node = node_for_shape(
            self.shape,
            name,
            peek,
            Walk { mode, specs },
            &my_path.join(name),
            // Never the container's own `optional`: that says the container is an
            // `Option`'s payload, which its items are not, exactly as
            // `list_node` builds them. An item that can be absent says so in its
            // own shape (`Vec<Option<T>>`), which the walk peels.
            /* optional */
            false,
        );
        // An entry is the string the wire would carry at the item's own path, so the
        // item takes it the way a rebuild from the wire does: a leaf parses it, a
        // unit-enum item chooses that variant or remembers an unknown name.
        if let Some(entry) = value.and_then(ItemValue::entry) {
            let values = [(my_path.join(name), entry.to_string())]
                .into_iter()
                .collect();
            node.distribute_values(my_path, specs, &values);
        }
        node
    }

    /// Make `items` exactly `keys`, in that order: an item already here keeps its
    /// node (and so its values), and a key not here gets a blank item.
    ///
    /// This is how a server, which starts from a collection with no items, gets the
    /// client's items back. `next_key` only ever grows, so a key the wire named
    /// is never handed out again by a later `add_item`.
    pub(crate) fn set_items(&mut self, my_path: &NodePath, keys: &[usize], specs: &SpecsByPath) {
        let mut existing: HashMap<String, Box<dyn Node>> =
            self.collection.drain(..).map(|r| (r.name(), r)).collect();
        let items: Vec<Box<dyn Node>> = keys
            .iter()
            .map(|&key| {
                existing
                    .remove(&item_segment(key))
                    .unwrap_or_else(|| self.new_item(my_path, key, specs, None))
            })
            .collect();
        self.collection = items;
        if let Some(&largest) = keys.iter().max() {
            self.next_key = self.next_key.max(largest + 1);
        }
    }

    /// Drop the item at `index`.
    ///
    /// No renumbering, and that is the point of keys: the surviving items keep
    /// their names, so every leaf beneath them keeps its path and the value
    /// store needs no shuffle. Bounds-checked rather than left to `Vec::remove`,
    /// which panics — and on wasm a panic aborts instead of reaching an
    /// `ErrorBoundary`.
    pub(crate) fn remove_item(
        &mut self,
        my_path: &NodePath,
        index: usize,
    ) -> Result<(), FormAccessError> {
        if index >= self.collection.len() {
            return Err(FormAccessError(format!(
                "cannot remove item at index {index} from {my_path}: it has {} items",
                self.collection.len()
            )));
        }
        self.collection.remove(index);
        Ok(())
    }

    /// Move the item according to the given indexes. If indexes are out of range,
    /// return `FormAccessError`.
    pub(crate) fn move_item(
        &mut self,
        my_path: &NodePath,
        from: usize,
        to: usize,
    ) -> Result<(), FormAccessError> {
        let len = self.collection.len();
        if from >= len || to >= len {
            return Err(FormAccessError(format!(
                "cannot move item at index {from} to index {to} in {my_path}; it has {len} items"
            )));
        }
        let item = self.collection.remove(from);
        self.collection.insert(to, item);
        Ok(())
    }

    pub(crate) fn len(&self) -> usize {
        self.collection.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.collection.is_empty()
    }

    pub(crate) fn iter(&self) -> Iter<'_, Box<dyn Node>> {
        self.collection.iter()
    }

    /// Where an item equal to `candidate` sits, as `order` judges it: only the
    /// set's item handler knows the item type. A node, not a value, so a
    /// set's draft and an `AddItem`'s value are asked the same way.
    pub(crate) fn position_of(&self, order: &SetOrder, candidate: &dyn Node) -> Option<usize> {
        order.item_order.position(&self.collection, candidate)
    }

    /// The node `value` would become, for asking whether it is already an
    /// item before it is added; `None` for a value of another type, which
    /// [`add_item`](Self::add_item) then refuses for its type. Built under the
    /// next key, which it never takes.
    pub(crate) fn probe(
        &self,
        my_path: &NodePath,
        specs: &SpecsByPath,
        value: &ItemValue,
    ) -> Option<Box<dyn Node>> {
        value
            .shape()
            .is_none_or(|shape| shape == self.shape)
            .then(|| self.new_item(my_path, self.next_key, specs, Some(value)))
    }

    /// A blank item under a key of its own, which no item will ever be given:
    /// a set's draft (`SETS_PLAN.md` 7a). Not one of the items until
    /// [`insert_node`](Self::insert_node) makes it one.
    pub(crate) fn new_draft(&mut self, my_path: &NodePath, specs: &SpecsByPath) -> Box<dyn Node> {
        let draft = self.new_item(my_path, self.next_key, specs, None);
        self.next_key += 1;
        draft
    }

    /// Whether an item already has `name`, which a key from the wire can take
    /// from a draft built before the keys arrived (a server's).
    pub(crate) fn has_item_named(&self, name: &str) -> bool {
        self.iter().any(|item| item.name() == name)
    }

    /// Take out the item named `name`, if there is one: a set's edited item,
    /// as its replacement goes in.
    pub(crate) fn remove_named(&mut self, name: &str) -> Option<Box<dyn Node>> {
        let at = self
            .collection
            .iter()
            .position(|item| item.name() == name)?;
        Some(self.collection.remove(at))
    }

    /// Make `node`, built by [`new_draft`](Self::new_draft), the last item,
    /// under the key it already has, so its leaves in the store are already at
    /// its path.
    pub(crate) fn insert_node(&mut self, node: Box<dyn Node>) {
        self.collection.push(node);
    }

    // Paths are unique, so at most one item can own this one. Dispatching by
    // containment rather than trying each in turn is what lets an item's real
    // error reach the caller intact.
    // `my_path`, NOT the items's own path: `edit`'s prefix excludes the
    // node's own name, which the node qualifies on itself. Passing
    // the item's full path double-qualifies it into `shapes.#1.#1` — the
    // same trap `StructNode::choose_variant` fell into.
    pub(crate) fn edit_within(
        &mut self,
        my_path: &NodePath,
        edit: &Edit,
        specs: &SpecsByPath,
        errors: &mut ErrorsByPath,
    ) -> Result<Option<NodePath>, FormAccessError> {
        let edit_path = edit.path();
        for m in &mut self.collection {
            if edit_path.starts_with(&my_path.join(&m.name())) {
                return m.edit(my_path, edit, specs, errors);
            }
        }

        Err(no_such_path(edit_path))
    }

    /// Sort the items by `order`. Only their order on screen changes: every
    /// key stays, since a key is an identity, not a position. How items are
    /// compared, and where an item with nothing to compare goes, is
    /// [`ItemHandler::sort_nodes`](crate::nodes::set_node::ItemHandler::sort_nodes)'s
    /// business.
    pub(crate) fn sort_by(&mut self, order: &SetOrder) {
        order.item_order.sort_nodes(&mut self.collection);
    }

    pub(crate) fn dedup(&mut self, order: &SetOrder) {
        order.item_order.dedup_nodes(&mut self.collection);
    }
}

impl<'a> IntoIterator for &'a Items {
    type Item = &'a Box<dyn Node>;
    type IntoIter = std::slice::Iter<'a, Box<dyn Node>>;
    fn into_iter(self) -> Self::IntoIter {
        self.collection.iter()
    }
}

impl<'a> IntoIterator for &'a mut Items {
    type Item = &'a mut Box<dyn Node>;
    type IntoIter = std::slice::IterMut<'a, Box<dyn Node>>;
    fn into_iter(self) -> Self::IntoIter {
        self.collection.iter_mut()
    }
}

/// The value a new item arrives filled in with: [`Edit::AddItem`]'s `value`.
///
/// **Two kinds.** [`new`](Self::new) wraps a typed value, which the item is
/// built from as `form_for` would build it. [`raw`](Self::raw) wraps an ENTRY,
/// the string the wire carries at the item's own path (`tags.#2` = `rust`, or
/// `days.#2` = `Friday` for a unit-enum item), for a widget that knows its
/// choices only as strings, such as a `checkbox_multiple` (`WIDGETS_PLAN.md`
/// decision F). An entry's item is built blank and given the entry the way a
/// rebuild from the wire gives it one, so it is parsed, not type-checked: an
/// entry that does not parse is an item holding an error, as typed text would be.
/// Everything below is about the typed kind.
///
/// **Type-erased** because `Edit` is not generic and cannot be. It crosses
/// the widget boundary as a `Callback<Edit>`, and one `Edit` type serves
/// every form, whatever its model. A list's item type is known only from its
/// shape, at runtime, so the value is a `T` behind `dyn Any`.
///
/// **The shape is kept** because nothing checks the type at compile time.
/// An edit names its list by a run-time [`NodePath`], so an `ItemValue` of any type can
/// be aimed at any list. `add_item` compares [`shape`](Self::shape) with the
/// list's item shape and refuses a mismatch, rather than building an item from
/// a value it would misread.
///
/// The rest serves `Edit`'s derives. `Rc`, not `Box`, makes `Clone` cheap
/// (the value is only ever read). The `peek` fn pointer, made in `new` where
/// `T` is still known, is how an erased value gets back to a
/// [`Peek`]: that is how the item is built, and how `Debug` and
/// `PartialEq` work without requiring either of `T`. Equality goes through
/// facet, so two values are equal only if they have the same shape and
/// facet can compare them. A `T` whose shape has no `PartialEq` is never
/// equal, not even to itself. Two entries are equal when their strings are,
/// and a typed value never equals an entry.
#[derive(Clone)]
pub struct ItemValue(ValueType);

/// [`ItemValue`]'s two kinds, private so the public type stays one name.
#[derive(Clone)]
enum ValueType {
    Typed {
        value: Rc<dyn Any>,
        shape: &'static Shape,
        peek: for<'a> fn(&'a dyn Any) -> Peek<'a, 'static>,
    },
    Entry(String),
}

impl ItemValue {
    /// Wrap an item's value. `T` is the collection's item type: a `String` for a
    /// `Vec<String>` or an `Option<Vec<String>>`, an `Option<String>` for a
    /// `Vec<Option<String>>`.
    pub fn new<T: Facet<'static> + 'static>(value: T) -> Self {
        Self(ValueType::Typed {
            value: Rc::new(value),
            shape: T::SHAPE,
            peek: |any| {
                Peek::new(
                    any.downcast_ref::<T>()
                        .expect("an ItemValue holds the T it was made from"),
                )
            },
        })
    }

    /// Name an item by its entry: the string the wire carries at the item's own
    /// path. A leaf item's text (`"rust"`), a unit-enum item's variant name
    /// (`"Friday"`). Parsed when the item is built, never type-checked: see the
    /// type's docs.
    pub fn raw(entry: impl Into<String>) -> Self {
        Self(ValueType::Entry(entry.into()))
    }

    /// The shape of the `T` this was made from, which `add_item` checks
    /// against the collection's items. `None` for an entry, which has no
    /// type to check: it is parsed when the item is built.
    pub fn shape(&self) -> Option<&'static Shape> {
        match self.0 {
            ValueType::Entry(_) => None,
            ValueType::Typed { shape, .. } => Some(shape),
        }
    }

    /// The value, readable by reflection: what a typed item is built from.
    /// `None` for an entry, which is parsed instead.
    pub(crate) fn peek(&self) -> Option<Peek<'_, 'static>> {
        match &self.0 {
            ValueType::Typed { value, peek, .. } => Some(peek(value.as_ref())),
            ValueType::Entry(_) => None,
        }
    }

    /// The entry a [`raw`](Self::raw) value names its item by; `None` for a
    /// typed value, which is read through [`peek`](Self::peek) instead.
    pub(crate) fn entry(&self) -> Option<&str> {
        match &self.0 {
            ValueType::Typed { .. } => None,
            ValueType::Entry(entry) => Some(entry),
        }
    }
}

impl Debug for ItemValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            ValueType::Typed { value, peek, .. } => f
                .debug_tuple("ItemValue")
                .field(&peek(value.as_ref()))
                .finish(),
            // Shown as the call that made it, so a log says which kind it was.
            ValueType::Entry(entry) => f.debug_tuple("ItemValue::raw").field(entry).finish(),
        }
    }
}

impl PartialEq for ItemValue {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (ValueType::Entry(a), ValueType::Entry(b)) => a == b,
            (ValueType::Typed { shape: a, .. }, ValueType::Typed { shape: b, .. }) => {
                a == b && self.peek() == other.peek()
            }
            // A typed value and an entry are never equal, even when they would
            // build the same item: comparing them would mean parsing the entry.
            _ => false,
        }
    }
}

/// `1 item`, `2 items`: a count of items, for a message.
fn items_word(count: usize) -> String {
    if count == 1 {
        "1 item".to_string()
    } else {
        format!("{count} items")
    }
}

/// The item keys a list's wire entry names, in order: `"#0,#2"` is `[0, 2]`, and
/// `""` is no items.
///
/// `None` for anything `collect_values` would not have written: a segment that
/// is not `#` and a number, a number not written canonically (`#01`, `#+1`,
/// which would otherwise name the same item as `#1`), or a repeated key. A
/// `HashSet`, not `Vec::contains`, so a forged entry with many keys costs time
/// linear in its length rather than quadratic.
pub(crate) fn parse_item_keys(raw: &str) -> Option<Vec<usize>> {
    if raw.is_empty() {
        return Some(Vec::new());
    }
    let mut keys = Vec::new();
    let mut seen = HashSet::new();
    for segment in raw.split(',') {
        let key: usize = segment.strip_prefix(ITEM_SIGIL)?.parse().ok()?;
        if item_segment(key) != segment || !seen.insert(key) {
            return None;
        }
        keys.push(key);
    }
    Some(keys)
}

#[cfg(test)]
mod tests {
    use super::parse_item_keys;
    use googletest::prelude::*;

    #[gtest]
    fn keys_parse_in_order() {
        expect_that!(parse_item_keys("#2,#0,#1"), some(eq(&vec![2, 0, 1])));
    }

    #[gtest]
    fn empty_is_no_items() {
        expect_that!(parse_item_keys(""), some(eq(&Vec::<usize>::new())));
    }

    #[gtest]
    fn anything_collect_values_would_not_write_is_rejected() {
        for bad in [
            "0", "#x", "#-1", "#01", "#+1", "#0,#0", "#0,", ",#0", "#0, #1",
        ] {
            expect_that!(parse_item_keys(bad), none(), "{bad:?}");
        }
    }
}
