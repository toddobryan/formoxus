//! A list-typed node. Rows are nodes named by a `#`-prefixed KEY, which is
//! what makes `answer_choices.#0.text` fall out of the ordinary `qualify`
//! nesting — and, because a key is an identity rather than a position, what lets
//! a row be inserted mid-list, removed, or reordered without renaming its
//! neighbours. See `row_segment` for why renaming would be a data hazard.

use crate::build::{FormMode, Walk, node_for_shape};
use crate::error::{FormAccessError, ValidationMessage};
use crate::form::ErrorsByPath;
use crate::label_case::LabelCase;
use crate::nodes::{
    Edit, Node, ROW_SIGIL, SpecsByPath, ValuesByPath, default_label, ensure_owned, no_such_path,
    owns, qualify, row_segment,
};
use crate::nodes::{NodeContext, RawValue};
use crate::widgets::errors::errors_id;
use crate::widgets::{AddRowButton, FieldErrors, RemoveRowButton};
use dioxus::prelude::*;
use facet::{Partial, ReflectError, Shape};
use formoxus_spec::FixedKey;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct ListSet {
    pub name: String,
    pub label: Option<String>,
    pub shape: &'static Shape,
    pub optional: bool,
    pub rows: Vec<Box<dyn Node>>,
    pub errors: Vec<ValidationMessage>,
    pub next_key: usize,
    /// A row-key entry that arrived malformed, kept so `validate` can re-derive
    /// the error, which a pushed error would not survive. The list's version of
    /// `VariantChoice::Unknown`.
    pub rejected_keys: Option<String>,
    /// Whether the user may move rows: the list's `allow_reordering` flag in
    /// `form!`. Such a list renders its rows as an `<ol>` of `<li>`s, so a
    /// screen reader announces each row's position, which here means something.
    pub allow_reordering: bool,
}

impl ListSet {
    /// Build one blank row under the next key and put it *before* position
    /// `at`, or on the end. The row's prefix has to match what `list_node`
    /// produces at construction, or the row would render fine and write to
    /// store keys nobody reads; `blank_row` keeps the two the same.
    fn add_row(
        &mut self,
        my_path: &str,
        before: Option<usize>,
        optional: bool,
        specs: &SpecsByPath,
    ) -> Result<(), FormAccessError> {
        let at = before.unwrap_or(self.rows.len());
        if at > self.rows.len() {
            return Err(FormAccessError(format!(
                "cannot insert at {at} in {my_path}: it has {} rows",
                self.rows.len()
            )));
        }
        let row = self.blank_row(my_path, self.next_key, optional, specs);
        self.next_key += 1;
        self.rows.insert(at, row);
        Ok(())
    }

    /// One blank row under `key`, built the way construction builds one.
    ///
    /// `None` peek and `FormMode::Blank`: a new row has no value to read, the
    /// same pair `VariantSet::set_variant` passes when it rebuilds a variant's
    /// nodes. Shared by `add_row`, which takes the next key, and `set_rows`,
    /// which takes the key the wire named.
    fn blank_row(
        &self,
        my_path: &str,
        key: usize,
        optional: bool,
        specs: &SpecsByPath,
    ) -> Box<dyn Node> {
        let name = row_segment(key);
        node_for_shape(
            self.shape,
            &name,
            None,
            Walk {
                mode: FormMode::Blank,
                specs,
            },
            &qualify(my_path, &name),
            optional,
        )
    }

    /// Make `rows` exactly `keys`, in that order: a row already here keeps its
    /// node (and so its values and errors), and a key not here gets a blank row.
    ///
    /// This is how a server, which starts from a list with no rows, gets the
    /// client's rows back. `next_key` only ever grows, so a key the wire named
    /// is never handed out again by a later `add_row`.
    fn set_rows(&mut self, my_path: &str, keys: &[usize], specs: &SpecsByPath) {
        let mut existing: HashMap<String, Box<dyn Node>> =
            self.rows.drain(..).map(|r| (r.name(), r)).collect();
        let rows: Vec<Box<dyn Node>> = keys
            .iter()
            .map(|&key| {
                existing
                    .remove(&row_segment(key))
                    .unwrap_or_else(|| self.blank_row(my_path, key, self.optional, specs))
            })
            .collect();
        self.rows = rows;
        if let Some(&largest) = keys.iter().max() {
            self.next_key = self.next_key.max(largest + 1);
        }
    }

    /// Drop the row at `index`.
    ///
    /// No renumbering, and that is the point of keys: the surviving rows keep
    /// their names, so every leaf beneath them keeps its path and the value
    /// store needs no shuffle. Bounds-checked rather than left to `Vec::remove`,
    /// which panics — and on wasm a panic aborts instead of reaching an
    /// `ErrorBoundary`.
    fn remove_row(&mut self, my_path: &str, index: usize) -> Result<(), FormAccessError> {
        if index >= self.rows.len() {
            return Err(FormAccessError(format!(
                "cannot remove row {index} from {my_path}: it has {} rows",
                self.rows.len()
            )));
        }
        self.rows.remove(index);
        Ok(())
    }

    /// Move the row according to the given indexes. If indexes are out of range,
    /// return `FormAccessError`.
    fn move_row(&mut self, my_path: &str, from: usize, to: usize) -> Result<(), FormAccessError> {
        let len = self.rows.len();
        if from >= len || to >= len {
            return Err(FormAccessError(format!(
                "cannot move row {from} to {to} in {my_path}; it has {len} rows"
            )));
        }
        let row = self.rows.remove(from);
        self.rows.insert(to, row);
        Ok(())
    }

    fn rows(&self, ctx: &NodeContext) -> Element {
        let list_row_class = ctx.classes.fixed_class(FixedKey::ListRow, false);
        let nested = ctx.nested(&self.name);
        let path = ctx.path(&self.name);
        rsx! {
            // Keyed by the row's own name, which is stable across
            // inserts, removals and reorders. Without a key dioxus diffs
            // the list by position, so inserting at the top would
            // re-render every row below it; with one it moves the nodes
            // it already has. The wrapper exists because a key has to sit
            // on an element, and it is what pairs a row with its widget.
            for (index, row) in self.rows.iter().enumerate() {
                if self.allow_reordering {
                    li { class: &list_row_class, key: "{row.name()}",
                        { row.render(&nested) }
                        RemoveRowButton { path: path.clone(), index, on_edit: ctx.on_edit, classes: ctx.classes.clone() }
                    }
                } else {
                    div { class: &list_row_class, key: "{row.name()}",
                        { row.render(&nested) }
                        RemoveRowButton { path: path.clone(), index, on_edit: ctx.on_edit, classes: ctx.classes.clone() }
                    }
                }
            }
        }
    }
}

impl Node for ListSet {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self, case: LabelCase) -> Option<String> {
        self.label
            .clone()
            .or_else(|| default_label(&self.name, case))
    }

    fn render(&self, ctx: &NodeContext) -> Element {
        // A `fieldset` for the same reason `FieldSet` and `VariantSet` use one:
        // the rows and the widgets that manage them read as one thing, and the
        // legend gives the list's label somewhere to appear.
        //
        // No `aria-labelledby`: a `<legend>` names its own fieldset, and unlike
        // `VariantSet` there is no control inside that needs naming too. No
        // `aria-invalid` either: a bare fieldset is role `group`, where ARIA
        // leaves it undefined (see `RadioGroupView`). So the list's own errors
        // reach assistive technology through `aria-describedby`, and CSS through
        // the fieldset's invalid class.
        //
        // No required marker, unlike `VariantSet`: nothing enforces a minimum row
        // count yet (#14), so marking a list required would be a promise
        // `validate()` doesn't keep.
        let id = ctx.id(&self.name);
        // The list's own path — where an `AddRow`/`RemoveRow` is addressed. NOT
        // `nested.prefix`'s children: the widgets act on the list, not on a row.
        let path = ctx.path(&self.name);
        // `here`, not `within`: a bad cell is its row's error, not the list's.
        let invalid = self.has_errors_here();
        let describedby = invalid.then(|| errors_id(&id));

        let fieldset_class = ctx.classes.fixed_class(FixedKey::ListSet, invalid);
        let legend_class = ctx.classes.fixed_class(FixedKey::ListSetLegend, false);
        let list_rows_class = ctx.classes.fixed_class(FixedKey::ListRows, false);
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
                    errors: self.errors.clone(),
                    classes: ctx.classes.clone(),
                }
                if self.allow_reordering {
                    ol { class: &list_rows_class,
                        { self.rows(ctx) }
                    }
                } else {
                    { self.rows(ctx) }
                }
                AddRowButton { path: path.clone(), on_edit: ctx.on_edit, classes: ctx.classes.clone() }
            }
        }
    }

    fn raw_value(&self) -> String {
        String::new()
    }

    fn collect_values(&self, prefix: &str, out: &mut Vec<RawValue>) {
        let my_path = qualify(prefix, &self.name);
        let keys: Vec<String> = self.rows.iter().map(|r| r.name()).collect();
        out.push(RawValue::Structure {
            path: my_path.clone(),
            data: keys.join(","),
        });
        for r in &self.rows {
            r.collect_values(&my_path, out);
        }
    }

    fn distribute_values(&mut self, prefix: &str, specs: &SpecsByPath, values: &ValuesByPath) {
        let my_path = qualify(prefix, &self.name);

        // The rows first, if the entry was sent, the way `VariantSet` applies
        // its choice first: on a server the list starts with no rows, and this
        // is what builds the ones the leaves below are about to fill. Absent
        // means "leave the rows alone", for a leaves-only map.
        if let Some(raw) = values.get(&my_path) {
            match parse_row_keys(raw) {
                Some(keys) => {
                    self.rejected_keys = None;
                    let current: Vec<String> = self.rows.iter().map(|r| r.name()).collect();
                    let wanted: Vec<String> = keys.iter().map(|&k| row_segment(k)).collect();
                    if current != wanted {
                        self.set_rows(&my_path, &keys, specs);
                    }
                }
                // Kept, not reported: `validate` turns it into the error. The
                // rows are left as they are.
                None => self.rejected_keys = Some(raw.clone()),
            }
        }

        for r in &mut self.rows {
            r.distribute_values(&my_path, specs, values);
        }
    }

    fn collect_errors(&self, prefix: &str, out: &mut ErrorsByPath) {
        // The list's own errors at its own path, before its rows', as
        // `VariantSet` does. Before #13 nothing ever put one there, so this
        // only recursed, and a rejected row-key entry came back as an `Err`
        // with no message anywhere.
        let nested = qualify(prefix, &self.name);
        if !self.errors.is_empty() {
            out.insert(nested.clone(), self.errors.clone());
        }
        for r in &self.rows {
            r.collect_errors(&nested, out);
        }
    }

    fn distribute_errors(&mut self, prefix: &str, errors: &mut ErrorsByPath) {
        let nested = qualify(prefix, &self.name);
        if let Some(found) = errors.shift_remove(&nested) {
            self.errors.extend(found);
        }
        for r in &mut self.rows {
            r.distribute_errors(&nested, errors);
        }
    }

    fn validate(&mut self) {
        self.errors.clear();
        if let Some(raw) = &self.rejected_keys {
            self.errors.push(ValidationMessage(format!(
                "{raw:?} is not a list of row keys (expected e.g. \"#0,#1\")"
            )));
        }
        for r in &mut self.rows {
            r.validate();
        }
    }

    fn has_errors_within(&self) -> bool {
        self.has_errors_here() || self.rows.iter().any(|r| r.has_errors_within())
    }

    fn has_errors_here(&self) -> bool {
        !self.errors.is_empty()
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }

    fn write_value_into<'p>(&self, partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        let mut partial = partial.init_list()?;
        for r in &self.rows {
            partial = partial.begin_list_item()?;
            partial = r.write_value_into(partial)?;
            partial = partial.end()?;
        }
        Ok(partial)
    }

    fn is_present(&self) -> bool {
        !self.rows.is_empty() && self.rows.iter().any(|fm| fm.is_present())
    }

    fn edit(
        &mut self,
        prefix: &str,
        edit: &Edit,
        specs: &SpecsByPath,
    ) -> Result<(), FormAccessError> {
        let path = edit.path();
        let my_path = qualify(prefix, &self.name);
        ensure_owned(&my_path, path)?;
        if path == my_path {
            return match edit {
                Edit::AddRow { before, .. } => {
                    self.add_row(&my_path, *before, self.optional, specs)
                }
                Edit::RemoveRow { index, .. } => self.remove_row(&my_path, *index),
                Edit::ChooseVariant { .. } => {
                    Err(FormAccessError(format!("{my_path} is a list, not an enum")))
                }
                Edit::MoveRow { from, to, .. } => self.move_row(&my_path, *from, *to),
            };
        }

        // Paths are unique, so at most one row can own this one. Dispatching by
        // containment rather than trying each in turn is what lets a row's real
        // error reach the caller intact.
        // `my_path`, NOT the row's own path: `edit`'s prefix excludes the
        // node's own name, which the node qualifies on itself. Passing
        // the row's full path double-qualifies it into `shapes.#1.#1` — the
        // same trap `FieldSet::choose_variant` fell into.
        for m in &mut self.rows {
            if owns(&qualify(&my_path, &m.name()), path) {
                return m.edit(&my_path, edit, specs);
            }
        }

        Err(no_such_path(path))
    }

    fn clear_errors(&mut self) {
        self.errors.clear();
        for r in &mut self.rows {
            r.clear_errors();
        }
    }
}

/// The row keys a list's wire entry names, in order: `"#0,#2"` is `[0, 2]`, and
/// `""` is no rows.
///
/// `None` for anything `collect_values` would not have written: a segment that
/// is not `#` and a number, a number not written canonically (`#01`, `#+1`,
/// which would otherwise name the same row as `#1`), or a repeated key. A
/// `HashSet`, not `Vec::contains`, so a forged entry with many keys costs time
/// linear in its length rather than quadratic.
fn parse_row_keys(raw: &str) -> Option<Vec<usize>> {
    if raw.is_empty() {
        return Some(Vec::new());
    }
    let mut keys = Vec::new();
    let mut seen = HashSet::new();
    for segment in raw.split(',') {
        let key: usize = segment.strip_prefix(ROW_SIGIL)?.parse().ok()?;
        if row_segment(key) != segment || !seen.insert(key) {
            return None;
        }
        keys.push(key);
    }
    Some(keys)
}

#[cfg(test)]
mod tests {
    use super::parse_row_keys;
    use googletest::prelude::*;

    #[gtest]
    fn keys_parse_in_order() {
        expect_that!(parse_row_keys("#2,#0,#1"), some(eq(&vec![2, 0, 1])));
    }

    #[gtest]
    fn empty_is_no_rows() {
        expect_that!(parse_row_keys(""), some(eq(&Vec::<usize>::new())));
    }

    #[gtest]
    fn anything_collect_values_would_not_write_is_rejected() {
        for bad in [
            "0", "#x", "#-1", "#01", "#+1", "#0,#0", "#0,", ",#0", "#0, #1",
        ] {
            expect_that!(parse_row_keys(bad), none(), "{bad:?}");
        }
    }
}
