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
}

struct CustomOrder<T> {
    cmp_value: fn(&T, &T) -> Ordering,
    name: String,
}

impl<T: Facet<'static>> CustomOrder<T> {
    fn value_of(node: &dyn Node) -> Option<T> {
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
            nodes.drain(..).map(|n| (Self::value_of(&*n), n)).collect();
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
            nodes.drain(..).map(|n| (Self::value_of(&*n), n)).collect();
        retain_first(
            &mut keyed,
            |(a, _), (b, _)| matches!((a, b), (Some(a), Some(b)) if a == b),
        );
        nodes.extend(keyed.into_iter().map(|(_, n)| n));
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
            return match edit {
                Edit::AddItem { value, .. } => match value {
                    None => Err(FormAccessError(format!(
                        "{my_path} is a set; you cannot add an empty item to a set"
                    ))),
                    Some(v) => {
                        let result = self
                            .items
                            .add_item(&my_path, None, specs, Some(v))
                            .map(Some);
                        if result.is_ok() {
                            self.items.sort_by(&self.comparator);
                        }
                        result
                    }
                },
                Edit::RemoveItem { index, .. } => {
                    self.items.remove_item(&my_path, *index).map(|()| None)
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
