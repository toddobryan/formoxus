//! A list-typed member. Rows are members named by a `#`-prefixed KEY, which is
//! what makes `answer_choices.#0.text` fall out of the ordinary `qualify`
//! nesting — and, because a key is an identity rather than a position, what lets
//! a row be inserted mid-list, removed, or reordered without renaming its
//! neighbours. See `row_segment` for why renaming would be a data hazard.

use crate::RenderCtx;
use crate::build::{FormMode, member_for_shape};
use crate::error::{FieldError, FormAccessError, FormError};
use crate::form::FieldErrors;
use crate::label_case::LabelCase;
use crate::members::{
    Edit, FieldSpecs, FormMember, default_label, ensure_owned, no_such_path, owns, qualify,
    row_segment,
};
use crate::widgets::{AddRowButton, RemoveRowButton};
use dioxus::prelude::*;
use facet::{Partial, ReflectError, Shape};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct ListSet {
    pub name: String,
    pub label: Option<String>,
    pub shape: &'static Shape,
    pub optional: bool,
    pub rows: Vec<Box<dyn FormMember>>,
    pub errors: Vec<FormError>,
    pub next_key: usize,
}

impl ListSet {
    /// Build one blank row and put it *before* position `at`, or on the end.
    ///
    /// `None` peek and `FormMode::Blank`: a new row has no value to read, the
    /// same pair `VariantSet::set_variant` passes when it rebuilds a variant's
    /// members. The row's prefix has to match what `list_member` produces at
    /// construction, or the row would render fine and write to store keys
    /// nobody reads.
    fn add_row(
        &mut self,
        my_path: &str,
        before: Option<usize>,
        optional: bool,
    ) -> Result<(), FormAccessError> {
        let at = before.unwrap_or(self.rows.len());
        if at > self.rows.len() {
            return Err(FormAccessError(format!(
                "cannot insert at {at} in {my_path}: it has {} rows",
                self.rows.len()
            )));
        }
        let name = row_segment(self.next_key);
        self.next_key += 1;
        let row = member_for_shape(
            self.shape,
            &name,
            None,
            FormMode::Blank,
            &qualify(my_path, &name),
            optional,
        );
        self.rows.insert(at, row);
        Ok(())
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
}

impl FormMember for ListSet {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self, case: LabelCase) -> Option<String> {
        self.label
            .clone()
            .or_else(|| default_label(&self.name, case))
    }

    fn render(&self, ctx: &RenderCtx) -> Element {
        let nested = ctx.nested(&self.name);
        // The list's own path — where an `AddRow`/`RemoveRow` is addressed. NOT
        // `nested.prefix`'s children: the widgets act on the list, not on a row.
        let path = ctx.path(&self.name);
        // A `fieldset` for the same reason `VariantSet` uses one: the rows and
        // the widgets that manage them are one thing. It also gives the list's
        // label somewhere to appear — until now no container but `FieldSet`
        // rendered its own label, so a `Vec<String>` called `answers` showed up
        // on the page as a bare stack of inputs.
        //
        // No required star, unlike `VariantSet`: nothing enforces a minimum row
        // count, so marking a list required would be a promise `validate()`
        // doesn't keep.
        rsx! {
            fieldset {
                if let Some(text) = self.label(ctx.label_case) {
                    legend { "{text}" }
                }
                for (index, row) in self.rows.iter().enumerate() {
                    // Keyed by the row's own name, which is stable across
                    // inserts, removals and reorders. Without a key dioxus diffs
                    // the list by position, so inserting at the top would
                    // re-render every row below it; with one it moves the nodes
                    // it already has. The wrapper exists because a key has to sit
                    // on an element, and it is what pairs a row with its widget.
                    div { class: "form-row", key: "{row.name()}",
                        { row.render(&nested) }
                        RemoveRowButton { path: path.clone(), index, on_edit: ctx.on_edit }
                    }
                }
                AddRowButton { path: path.clone(), on_edit: ctx.on_edit }
            }
        }
    }

    fn raw_value(&self) -> String {
        String::new()
    }

    fn collect_leaves(&self, prefix: &str, out: &mut Vec<(String, String)>) {
        let nested = qualify(prefix, &self.name);
        for r in &self.rows {
            r.collect_leaves(&nested, out);
        }
    }

    fn collect_errors(&self, prefix: &str, out: &mut FieldErrors) {
        let nested = qualify(prefix, &self.name);
        for r in &self.rows {
            r.collect_errors(&nested, out);
        }
    }

    fn apply_leaves(&mut self, prefix: &str, values: &HashMap<String, String>) {
        let nested = qualify(prefix, &self.name);
        for r in &mut self.rows {
            r.apply_leaves(&nested, values);
        }
    }

    fn validate(&mut self) {
        self.errors.clear();
        for r in &mut self.rows {
            r.validate();
        }
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.rows.iter().any(|r| r.has_errors())
    }

    fn clone_box(&self) -> Box<dyn FormMember> {
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

    fn edit(&mut self, prefix: &str, edit: &Edit) -> Result<(), FormAccessError> {
        let path = edit.path();
        let my_path = qualify(prefix, &self.name);
        ensure_owned(&my_path, path)?;
        if path == my_path {
            return match edit {
                Edit::AddRow { before, .. } => self.add_row(&my_path, *before, self.optional),
                Edit::RemoveRow { index, .. } => self.remove_row(&my_path, *index),
                Edit::ChooseVariant { .. } => {
                    Err(FormAccessError(format!("{my_path} is a list, not an enum")))
                }
            };
        }

        // Paths are unique, so at most one row can own this one. Dispatching by
        // containment rather than trying each in turn is what lets a row's real
        // error reach the caller intact.
        // `my_path`, NOT the row's own path: `edit`'s prefix excludes the
        // member's own name, which the member qualifies on itself. Passing
        // the row's full path double-qualifies it into `shapes.#1.#1` — the
        // same trap `FieldSet::choose_variant` fell into.
        for m in &mut self.rows {
            if owns(&qualify(&my_path, &m.name()), path) {
                return m.edit(&my_path, edit);
            }
        }

        Err(no_such_path(path))
    }

    fn push_field_error(
        &mut self,
        prefix: &str,
        path: &str,
        error: FieldError,
    ) -> Result<(), FormAccessError> {
        // No `path == my_path` case, unlike `edit`: a list has no field of its
        // own to attach a `FieldError` to (`self.errors` holds `FormError`s, a
        // different type) — only a row can be the field a server complained
        // about. `my_path`, not a row's own path, for the same reason `edit`
        // qualifies against it: a row qualifies its own name onto whatever
        // prefix it's handed.
        let my_path = qualify(prefix, &self.name);
        ensure_owned(&my_path, path)?;
        for m in &mut self.rows {
            if owns(&qualify(&my_path, &m.name()), path) {
                return m.push_field_error(&my_path, path, error);
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

    fn apply_specs(&mut self, prefix: &str, fields: &FieldSpecs) {
        let my_path = qualify(prefix, &self.name);
        if let Some(spec) = fields.get(&my_path) {
            // Checked before anything is written — see `FieldSet::apply_specs`.
            assert!(
                spec.custom_widget.is_none(),
                "{my_path} is a list, which has no single widget to override — \
                 write `{}[]` to give every ROW a widget, or did you mean `label`?",
                self.name
            );
            self.label = spec.label.clone().or(self.label.take());
        }

        // `venues[]` and `venues[].city` name every row and every row's field.
        // They are resolved HERE, by substituting each row's actual segment, so
        // the rows themselves are then visited by the ordinary traversal and need
        // to know nothing about `[]`.
        //
        // Substitution rather than a second lookup path because it composes:
        // `rows[].cells[]` rewrites one bracket per level as the recursion
        // descends, and a nested `ListSet` resolves its own without special
        // casing. Unrelated keys pass through untouched.
        let marker = format!("{my_path}[]");
        for m in &mut self.rows {
            // `my_path`, NOT the row's own path: a row qualifies its own name
            // onto whatever prefix it is handed, so passing the full path
            // double-qualifies it into `shapes.#1.#1` — the trap `edit`
            // documents above.
            let row_path = qualify(&my_path, &m.name());
            let row_fields: FieldSpecs = fields
                .iter()
                .map(|(k, v)| (k.replace(&marker, &row_path), v.clone()))
                .collect();
            m.apply_specs(&my_path, &row_fields);
        }
    }
}
