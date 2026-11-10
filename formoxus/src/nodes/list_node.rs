//! A list-typed node. Items are nodes named by a `#`-prefixed KEY, which is
//! what makes `answer_choices.#0.text` fall out of the ordinary `join`
//! nesting — and, because a key is an identity rather than a position, what lets
//! an item be inserted mid-list, removed, or reordered without renaming its
//! neighbours. See `item_segment` for why renaming would be a data hazard.

use crate::error::FormAccessError;
use crate::field_attrs::Validity;
use crate::form::ErrorsByPath;
use crate::label_case::LabelCase;
use crate::nodes::{
    Drafts, Edit, ErrorsStoreExt, Items, Node, SpecsByPath, ValuesByPath, default_label,
};
use crate::nodes::{NodeContext, RawValue};
use crate::path::NodePath;
use crate::widgets::field_row::required_marker;
use crate::widgets::{AddItemProps, FieldErrors, ItemAdder, MoveItemButtons, RemoveItemButton};
use crate::widgets::{described_by, field_help};
use dioxus::core::{DynamicNode, VComponent};
use dioxus::prelude::*;
use facet::{Partial, ReflectError};
use formoxus_spec::FixedKey;

#[derive(Clone, Debug)]
pub struct ListNode {
    pub name: String,
    pub label: Option<String>,
    /// Its hint (`help_text:`), under the legend.
    pub help_text: Option<String>,
    /// Its own check (`validator:`), over the whole collection once every
    /// item is valid; its messages go under the legend with the list's own.
    pub validator: Option<crate::nodes::FieldValidator>,
    pub optional: bool,
    pub adder: ItemAdder,
    pub items: Items,
}

impl Node for ListNode {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self, case: LabelCase) -> Option<String> {
        self.label
            .clone()
            .or_else(|| default_label(&self.name, case))
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
        // A required marker only with a `min_items` of at least 1 (#14):
        // that is the promise `validate()` keeps. Without one, an empty list
        // is a complete answer, so marking it required would be a promise
        // nothing keeps.
        let id = ctx.id(&self.name);
        // The list's own path — where an `AddItem`/`RemoveItem` is addressed. NOT
        // `nested.prefix`'s children: the widgets act on the list, not on an item.
        let path = ctx.path(&self.name);
        let invalid = ctx.errors.validity_at(&path) == Validity::Invalid;

        let describedby = described_by(&id, self.help_text.is_some(), invalid);

        let fieldset_class = ctx.classes.fixed_class(FixedKey::List, invalid);
        let legend_class = ctx.classes.fixed_class(FixedKey::ListLegend, false);
        let list_item_class = ctx.classes.fixed_class(FixedKey::ListItem, false);
        let class_for_list_items = ctx.classes.fixed_class(FixedKey::ListItems, false);
        let nested = ctx.nested(&self.name);
        rsx! {
            fieldset {
                class: fieldset_class,
                id: "{id}",
                aria_describedby: describedby,
                if let Some(text) = self.label(ctx.label_case) {
                    legend { class: legend_class,
                        "{text}"
                        // Required once at least one item is: a promise
                        // `validate` keeps through `min_items` (#14).
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
                    ol { class: &class_for_list_items,
                        // Keyed by the item's own name, which is stable across
                        // inserts, removals and reorders. Without a key dioxus diffs
                        // the list by position, so inserting at the top would
                        // re-render every item below it; with one it moves the nodes
                        // it already has. The wrapper exists because a key has to sit
                        // on an element, and it is what pairs an item with its widget.
                        for (index, item) in self.items.iter().enumerate() {
                            // Move buttons on the left, Remove on the right, so Remove
                            // is never next to a move (D in `ORDERED_LISTS_PLAN.md`).
                            li { class: &list_item_class, key: "{item.name()}",
                                MoveItemButtons { path: path.clone(), index, count: self.items.len(), on_edit: ctx.on_edit, classes: ctx.classes.clone() }
                                { item.render(&nested) }
                                RemoveItemButton { path: path.clone(), index, disabled: self.items.at_min(), on_edit: ctx.on_edit, classes: ctx.classes.clone() }
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
                            item_count: self.items.len(),
                            max_items: self.items.max_items(),
                            draft: None,
                            editing: None,
                        },
                        "ItemAdder",
                    ))
                }
            }
        }
    }

    fn raw_value(&self) -> String {
        String::new()
    }

    fn collect_values(&self, prefix: &NodePath, out: &mut Vec<RawValue>) {
        self.items.collect_values(&prefix.join(&self.name), out);
    }

    fn distribute_values(&mut self, prefix: &NodePath, specs: &SpecsByPath, values: &ValuesByPath) {
        self.items
            .distribute_values(&prefix.join(&self.name), specs, values);
    }

    fn paths(&self, prefix: &NodePath, drafts: Drafts, out: &mut Vec<NodePath>) {
        let path = prefix.join(&self.name);
        out.push(path.clone());
        self.items.paths(&path, drafts, out);
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
        let mut partial = partial.init_list()?;
        for r in &self.items {
            partial = partial.begin_list_item()?;
            partial = r.write_value_into(partial)?;
            partial = partial.end()?;
        }
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
            return match edit {
                Edit::AddItem { before, value, .. } => self
                    .items
                    .add_item(&my_path, *before, specs, value.as_ref())
                    .map(Some),
                Edit::RemoveItem { index, .. } => {
                    self.items.remove_item(&my_path, *index).map(|()| None)
                }
                Edit::ChooseVariant { .. } => {
                    Err(FormAccessError(format!("{my_path} is a list, not an enum")))
                }
                Edit::MoveItem { from, to, .. } => {
                    self.items.move_item(&my_path, *from, *to).map(|()| None)
                }
                Edit::EditItem { .. } => Err(FormAccessError(format!(
                    "{my_path} is a list; its items are edited in place"
                ))),
            };
        }

        self.items.edit_within(&my_path, edit, specs, errors)
    }

    fn absent_owner(&self, prefix: &NodePath, path: &NodePath) -> Option<NodePath> {
        let nested = prefix.join(&self.name);
        if path.starts_with(&nested) {
            self.items
                .iter()
                .find_map(|i| i.absent_owner(&nested, path))
        } else {
            None
        }
    }
}
