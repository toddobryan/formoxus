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
    nodes::{
        Edit, Node, RawValue, SetOrder, SpecsByPath, ValuesByPath, no_such_path, owns, qualify,
    },
};

#[derive(Clone, Debug)]
pub struct Items {
    shape: &'static Shape,
    collection: Vec<Box<dyn Node>>,
    next_key: usize,
    rejected_keys: Option<String>,
    errors: Vec<ValidationMessage>,
}

impl Items {
    pub(crate) fn new(shape: &'static Shape, collection: Vec<Box<dyn Node>>) -> Self {
        let next_key = collection.len();
        Self {
            shape,
            collection,
            next_key,
            rejected_keys: None,
            errors: Vec::new(),
        }
    }

    pub(crate) fn errors(&self) -> &[ValidationMessage] {
        &self.errors
    }

    pub(crate) fn collect_values(&self, my_path: &str, out: &mut Vec<RawValue>) {
        let keys: Vec<String> = self.iter().map(|item| item.name()).collect();
        out.push(RawValue::Structure {
            path: my_path.to_string(),
            data: keys.join(","),
        });
        for item in self {
            item.collect_values(my_path, out);
        }
    }

    pub(crate) fn distribute_values(
        &mut self,
        my_path: &str,
        specs: &SpecsByPath,
        values: &ValuesByPath,
    ) {
        if let Some(raw) = values.get(my_path) {
            match parse_item_keys(raw) {
                Some(keys) => {
                    self.rejected_keys = None;
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

    pub(crate) fn collect_errors(&self, my_path: &str, out: &mut ErrorsByPath) {
        // The list's own errors at its own path, before its rows', as
        // `VariantNode` does. Before #13 nothing ever put one there, so this
        // only recursed, and a rejected row-key entry came back as an `Err`
        // with no message anywhere.
        if !self.errors.is_empty() {
            out.insert(my_path.to_string(), self.errors.clone());
        }
        for r in &self.collection {
            r.collect_errors(my_path, out);
        }
    }

    pub(crate) fn distribute_errors(&mut self, my_path: &str, errors: &mut ErrorsByPath) {
        if let Some(found) = errors.shift_remove(my_path) {
            self.errors.extend(found);
        }
        for r in &mut self.collection {
            r.distribute_errors(my_path, errors);
        }
    }

    pub(crate) fn validate(&mut self) {
        self.errors.clear();
        if let Some(raw) = &self.rejected_keys {
            self.errors.push(ValidationMessage(format!(
                "{raw:?} is not a list of item keys (expected e.g. \"#0,#1\")"
            )));
        }
        for r in &mut self.collection {
            r.validate();
        }
    }

    pub(crate) fn has_errors_within(&self) -> bool {
        self.has_errors_here() || self.iter().any(|r| r.has_errors_within())
    }

    pub(crate) fn has_errors_here(&self) -> bool {
        !self.errors.is_empty()
    }

    pub(crate) fn is_present(&self) -> bool {
        !self.is_empty() && self.iter().any(|fm| fm.is_present())
    }

    pub(crate) fn clear_errors(&mut self) {
        self.errors.clear();
        for r in &mut self.collection {
            r.clear_errors();
        }
    }

    /// Build one item under the next key, from `value` if there is one and
    /// blank if not, and put it *before* position `at`, or at the end. A value
    /// whose type is not the item type is refused, and nothing is added. The
    /// item's prefix has to match what its container produces at construction, or
    /// the item would render fine and write to store keys nobody reads;
    /// `new_item` keeps the two the same.
    pub(crate) fn add_item(
        &mut self,
        my_path: &str,
        before: Option<usize>,
        specs: &SpecsByPath,
        value: Option<&ItemValue>,
    ) -> Result<String, FormAccessError> {
        if let Some(value) = value
            && value.shape() != self.shape
        {
            return Err(FormAccessError(format!(
                "cannot add a `{}` item to {my_path}: its items are `{}`",
                value.shape(),
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
        let item_path = qualify(my_path, &item.name());
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
        my_path: &str,
        key: usize,
        specs: &SpecsByPath,
        value: Option<&ItemValue>,
    ) -> Box<dyn Node> {
        let name = item_segment(key);
        // A value makes this one item a `form_for` of its own: read from it, and
        // in `Populated` mode, so a `None` inside it means "absent" (answered),
        // not "nothing to read" as it does in a blank item.
        let (peek, mode) = match value {
            Some(value) => (Some(value.peek()), FormMode::Populated),
            None => (None, FormMode::Blank),
        };
        node_for_shape(
            self.shape,
            &name,
            peek,
            Walk { mode, specs },
            &qualify(my_path, &name),
            // Never the container's own `optional`: that says the container is an
            // `Option`'s payload, which its items are not, exactly as
            // `list_node` builds them. A row that can be absent says so in its
            // own shape (`Vec<Option<T>>`), which the walk peels.
            /* optional */
            false,
        )
    }

    /// Make `items` exactly `keys`, in that order: an item already here keeps its
    /// node (and so its values and errors), and a key not here gets a blank row.
    ///
    /// This is how a server, which starts from a collection with no items, gets the
    /// client's items back. `next_key` only ever grows, so a key the wire named
    /// is never handed out again by a later `add_item`.
    pub(crate) fn set_items(&mut self, my_path: &str, keys: &[usize], specs: &SpecsByPath) {
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
        my_path: &str,
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
        my_path: &str,
        from: usize,
        to: usize,
    ) -> Result<(), FormAccessError> {
        let len = self.collection.len();
        if from >= len || to >= len {
            return Err(FormAccessError(format!(
                "cannot move item at index {from} to index {to} in {my_path}; it has {len} items"
            )));
        }
        let row = self.collection.remove(from);
        self.collection.insert(to, row);
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
        my_path: &str,
        specs: &SpecsByPath,
        value: &ItemValue,
    ) -> Option<Box<dyn Node>> {
        (value.shape() == self.shape)
            .then(|| self.new_item(my_path, self.next_key, specs, Some(value)))
    }

    /// A blank item under a key of its own, which no item will ever be given:
    /// a set's draft (`SETS_PLAN.md` 7a). Not one of the items until
    /// [`insert_node`](Self::insert_node) makes it one.
    pub(crate) fn new_draft(&mut self, my_path: &str, specs: &SpecsByPath) -> Box<dyn Node> {
        let draft = self.new_item(my_path, self.next_key, specs, None);
        self.next_key += 1;
        draft
    }

    /// Whether an item already has `name`, which a key from the wire can take
    /// from a draft built before the keys arrived (a server's).
    pub(crate) fn has_item_named(&self, name: &str) -> bool {
        self.iter().any(|item| item.name() == name)
    }

    /// Make `node`, built by [`new_draft`](Self::new_draft), the last item,
    /// under the key it already has, so its leaves in the store are already at
    /// its path.
    pub(crate) fn insert_node(&mut self, node: Box<dyn Node>) {
        self.collection.push(node);
    }

    // Paths are unique, so at most one item can own this one. Dispatching by
    // containment rather than trying each in turn is what lets a row's real
    // error reach the caller intact.
    // `my_path`, NOT the items's own path: `edit`'s prefix excludes the
    // node's own name, which the node qualifies on itself. Passing
    // the item's full path double-qualifies it into `shapes.#1.#1` — the
    // same trap `StructNode::choose_variant` fell into.
    pub(crate) fn edit_within(
        &mut self,
        my_path: &str,
        edit: &Edit,
        specs: &SpecsByPath,
    ) -> Result<Option<String>, FormAccessError> {
        let edit_path = edit.path();
        for m in &mut self.collection {
            if owns(&qualify(my_path, &m.name()), edit_path) {
                return m.edit(my_path, edit, specs);
            }
        }

        Err(no_such_path(edit_path))
    }

    /// Sort the items by `order`. Only their order on screen changes: every
    /// key stays, since a key is an identity, not a position. How items are
    /// compared, and where an item with nothing to compare goes, is
    /// [`ItemOrder::sort_nodes`](crate::nodes::set_node::ItemOrder::sort_nodes)'s
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

/// The value a new item arrives filled in with: [`Edit::AddItem`][crate::nodes::Edit::AddItem]'s `value`.
///
/// **Type-erased** because `Edit` is not generic and cannot be. It crosses
/// the widget boundary as a `Callback<Edit>`, and one `Edit` type serves
/// every form, whatever its model. A list's item type is known only from its
/// shape, at runtime, so the value is a `T` behind `dyn Any`.
///
/// **The shape is kept** because nothing checks the type at compile time.
/// An edit names its list by a string path, so an `ItemValue` of any type can
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
/// equal, not even to itself.
#[derive(Clone)]
pub struct ItemValue {
    value: Rc<dyn Any>,
    shape: &'static Shape,
    peek: for<'a> fn(&'a dyn Any) -> Peek<'a, 'static>,
}

impl ItemValue {
    /// Wrap an item's value. `T` is the collection's item type: a `String` for a
    /// `Vec<String>` or an `Option<Vec<String>>`, an `Option<String>` for a
    /// `Vec<Option<String>>`.
    pub fn new<T: Facet<'static> + 'static>(value: T) -> Self {
        Self {
            value: Rc::new(value),
            shape: T::SHAPE,
            peek: |any| {
                Peek::new(
                    any.downcast_ref::<T>()
                        .expect("an ItemValue holds the T it was made from"),
                )
            },
        }
    }

    /// The shape of the `T` this was made from, which `add_item` checks
    /// against the collection's rows.
    pub fn shape(&self) -> &'static Shape {
        self.shape
    }

    /// The value, readable by reflection: what the item is built from.
    pub(crate) fn peek(&self) -> Peek<'_, 'static> {
        (self.peek)(self.value.as_ref())
    }
}

impl Debug for ItemValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ItemValue").field(&self.peek()).finish()
    }
}

impl PartialEq for ItemValue {
    fn eq(&self, other: &Self) -> bool {
        self.shape == other.shape && self.peek() == other.peek()
    }
}

/// Item keys are `#`-prefixed for the same reason variants are `$`-prefixed:
/// neither character can begin a Rust identifier, so a key can never be mistaken
/// for a field name.
pub(crate) const ITEM_SIGIL: char = '#';

/// An item key segment: `3` -> `#3`.
///
/// An item's name is its *identity*, not its position — which is what lets an item
/// be inserted in the middle, removed, or reordered without renaming its
/// neighbours. Renaming them would move every leaf beneath them to a different
/// key in the value store, and the values would have to be shuffled to match:
/// the same silent-corruption hazard [`variant_segment`] exists to prevent,
/// except recurring on every insert and remove rather than only on a variant
/// switch.
///
/// The cost is that an item's *position* is no longer recoverable from its path.
/// Order lives in `ListNode::items` and nowhere else — see [`model_path`].
pub(crate) fn item_segment(key: usize) -> String {
    format!("{ITEM_SIGIL}{key}")
}

/// The row keys a list's wire entry names, in order: `"#0,#2"` is `[0, 2]`, and
/// `""` is no rows.
///
/// `None` for anything `collect_values` would not have written: a segment that
/// is not `#` and a number, a number not written canonically (`#01`, `#+1`,
/// which would otherwise name the same row as `#1`), or a repeated key. A
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
    fn empty_is_no_rows() {
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
