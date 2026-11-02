//! A set-typed node. Items are nodes named by a `#`-prefixed KEY, which is
//! what makes `answer_choices.#0.text` fall out of the ordinary `qualify`
//! nesting — and, because a key is an identity rather than a position, what lets
//! an item be inserted anywhere. If the item type implements Ord, then the items
//! are displayed in ascending order. If not, they are displayed in ascending order
//! of their Debug string output. The ordering can be overridden using the `sort_by`
//! attribute in the form! macro.

use std::{cmp::Ordering, fmt::Debug, sync::Arc};

use dioxus::{
    core::{DynamicNode, VComponent},
    prelude::*,
};
use facet::{Partial, Peek, ReflectError};
use formoxus_spec::FixedKey;

use crate::{
    error::FormAccessError,
    form::ErrorsByPath,
    nodes::{
        Edit, Items, Node, NodeContext, SpecsByPath, ValuesByPath, default_label, ensure_owned,
        qualify,
    },
    widgets::{AddItemProps, FieldErrors, ItemAdder, RemoveItemButton, errors::errors_id},
};

#[derive(Clone)]
pub struct SetOrder {
    pub f: Arc<dyn Fn(Peek<'_, '_>, Peek<'_, '_>) -> Ordering + Send + Sync>,
    pub name: String,
}

impl Debug for SetOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SetOrder")
            .field("name", &self.name)
            .finish()
    }
}

impl PartialEq for SetOrder {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.f, &other.f) && self.name == other.name
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
        let mut partial = partial.init_set()?;
        for item in &self.items {
            partial = partial.begin_set_item()?;
            partial = item.write_value_into(partial)?;
            partial = partial.end()?;
        }
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
