//! A list-typed node. Items are nodes named by a `#`-prefixed KEY, which is
//! what makes `answer_choices.#0.text` fall out of the ordinary `qualify`
//! nesting — and, because a key is an identity rather than a position, what lets
//! an item be inserted mid-list, removed, or reordered without renaming its
//! neighbours. See `item_segment` for why renaming would be a data hazard.

use crate::error::FormAccessError;
use crate::form::ErrorsByPath;
use crate::label_case::LabelCase;
use crate::nodes::{
    Edit, Items, Node, SpecsByPath, ValuesByPath, default_label, ensure_owned, qualify,
};
use crate::nodes::{NodeContext, RawValue};
use crate::widgets::errors::errors_id;
use crate::widgets::{AddItemProps, FieldErrors, ItemAdder, MoveItemButtons, RemoveItemButton};
use dioxus::core::{DynamicNode, VComponent};
use dioxus::prelude::*;
use facet::{Partial, ReflectError};
use formoxus_spec::FixedKey;

#[derive(Clone, Debug)]
pub struct ListNode {
    pub name: String,
    pub label: Option<String>,
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
        // the rows and the widgets that manage them read as one thing, and the
        // legend gives the list's label somewhere to appear.
        //
        // No `aria-labelledby`: a `<legend>` names its own fieldset, and unlike
        // `VariantNode` there is no control inside that needs naming too. No
        // `aria-invalid` either: a bare fieldset is role `group`, where ARIA
        // leaves it undefined (see `RadioGroupView`). So the list's own errors
        // reach assistive technology through `aria-describedby`, and CSS through
        // the fieldset's invalid class.
        //
        // No required marker, unlike `VariantNode`: nothing enforces a minimum row
        // count yet (#14), so marking a list required would be a promise
        // `validate()` doesn't keep.
        let id = ctx.id(&self.name);
        // The list's own path — where an `AddItem`/`RemoveItem` is addressed. NOT
        // `nested.prefix`'s children: the widgets act on the list, not on a row.
        let path = ctx.path(&self.name);
        // `here`, not `within`: a bad cell is its row's error, not the list's.
        let invalid = self.has_errors_here();
        let describedby = invalid.then(|| errors_id(&id));

        let fieldset_class = ctx.classes.fixed_class(FixedKey::List, invalid);
        let legend_class = ctx.classes.fixed_class(FixedKey::ListLegend, false);
        let list_row_class = ctx.classes.fixed_class(FixedKey::ListRow, false);
        let class_for_list_rows = ctx.classes.fixed_class(FixedKey::ListRows, false);
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
                    ol { class: &class_for_list_rows,
                        // Keyed by the row's own name, which is stable across
                        // inserts, removals and reorders. Without a key dioxus diffs
                        // the list by position, so inserting at the top would
                        // re-render every row below it; with one it moves the nodes
                        // it already has. The wrapper exists because a key has to sit
                        // on an element, and it is what pairs a row with its widget.
                        for (index, row) in self.items.iter().enumerate() {
                            // Move buttons on the left, Remove on the right, so Remove
                            // is never next to a move (D in `ORDERED_LISTS_PLAN.md`).
                            li { class: &list_row_class, key: "{row.name()}",
                                MoveItemButtons { path: path.clone(), index, count: self.items.len(), on_edit: ctx.on_edit, classes: ctx.classes.clone() }
                                { row.render(&nested) }
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

    fn raw_value(&self) -> String {
        String::new()
    }

    fn collect_values(&self, prefix: &str, out: &mut Vec<RawValue>) {
        self.items.collect_values(&qualify(prefix, &self.name), out);
    }

    fn distribute_values(&mut self, prefix: &str, specs: &SpecsByPath, values: &ValuesByPath) {
        self.items
            .distribute_values(&qualify(prefix, &self.name), specs, values);
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
        prefix: &str,
        edit: &Edit,
        specs: &SpecsByPath,
    ) -> Result<Option<String>, FormAccessError> {
        let path = edit.path();
        let my_path = qualify(prefix, &self.name);
        ensure_owned(&my_path, path)?;
        if path == my_path {
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
            };
        }

        self.items.edit_within(&my_path, edit, specs)
    }

    fn clear_errors(&mut self) {
        self.items.clear_errors();
    }
}
