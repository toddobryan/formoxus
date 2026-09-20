//! A struct-typed member: its fields, nested under its own name.

use crate::RenderCtx;
use crate::error::{FieldError, FormAccessError, FormError};
use crate::form::FieldErrors;
use crate::label_case::LabelCase;
use crate::members::{
    Edit, FieldSpecs, FormMember, default_label, ensure_owned, no_such_path, owns, qualify,
};
use dioxus::prelude::*;
use facet::{Partial, ReflectError};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FieldSet {
    pub name: String,
    pub optional: bool,
    pub label: Option<String>,
    pub members: Vec<Box<dyn FormMember>>,
    pub errors: Vec<FormError>,
}

impl FormMember for FieldSet {
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
        let members_rendered = self.members.iter().map(|m| m.render(&nested));
        rsx! {
            fieldset {
                if let Some(text) = self.label(ctx.label_case) {
                    legend { "{text}"}
                }
                { members_rendered.into_iter() }
            }
        }
    }

    fn validate(&mut self) {
        self.errors.clear();
        for m in self.members.iter_mut() {
            m.validate();
        }
    }

    fn clone_box(&self) -> Box<dyn FormMember> {
        Box::new(self.clone())
    }

    fn has_errors(&self) -> bool {
        !self.errors.is_empty() || self.members.iter().any(|m| m.has_errors())
    }

    fn raw_value(&self) -> String {
        // A field set isn't a scalar — it has no single input of its own.
        String::new()
    }

    fn collect_leaves(&self, prefix: &str, out: &mut Vec<(String, String)>) {
        let nested = qualify(prefix, &self.name);
        for m in self.members.iter() {
            m.collect_leaves(&nested, out);
        }
    }

    fn collect_errors(&self, prefix: &str, out: &mut FieldErrors) {
        let nested = qualify(prefix, &self.name);
        for m in self.members.iter() {
            m.collect_errors(&nested, out);
        }
    }

    fn apply_leaves(&mut self, prefix: &str, values: &HashMap<String, String>) {
        let nested = qualify(prefix, &self.name);
        for m in self.members.iter_mut() {
            m.apply_leaves(&nested, values);
        }
    }

    fn write_value_into<'p>(&self, mut partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        for m in self.members.iter() {
            partial = m.write_into(partial)?;
        }
        Ok(partial)
    }

    fn is_present(&self) -> bool {
        self.members.iter().any(|fm| fm.is_present())
    }

    fn edit(&mut self, prefix: &str, edit: &Edit) -> Result<(), FormAccessError> {
        let nested = qualify(prefix, &self.name);
        let path = edit.path();
        ensure_owned(&nested, path)?;

        // Paths are unique, so at most one child can own this one. Dispatching
        // by containment rather than trying each in turn is what lets a child's
        // real error ("no such variant") reach the caller intact.
        for m in self.members.iter_mut() {
            if owns(&qualify(&nested, &m.name()), path) {
                return m.edit(&nested, edit);
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
        let nested = qualify(prefix, &self.name);
        ensure_owned(&nested, path)?;
        for m in self.members.iter_mut() {
            if owns(&qualify(&nested, &m.name()), path) {
                return m.push_field_error(&nested, path, error);
            }
        }
        Err(no_such_path(path))
    }

    fn clear_errors(&mut self) {
        self.errors.clear();
        for m in self.members.iter_mut() {
            m.clear_errors();
        }
    }

    fn apply_specs(&mut self, prefix: &str, fields: &FieldSpecs) {
        let my_path = qualify(prefix, &self.name);
        if let Some(spec) = fields.get(&my_path) {
            // Checked before anything is written, so a rejected spec leaves the
            // member untouched — which matters the day this becomes a `Result`.
            assert!(
                spec.custom_control.is_none(),
                "{my_path} is a field set, which has no single control to \
                 override (a composite control for one isn't supported yet) — \
                 did you mean `label`?"
            );
            self.label = spec.label.clone().or(self.label.take());
        }
        // Unlike `edit`, which finds the one member owning a path and stops, a
        // spec may speak about any number of descendants — so every child is
        // visited, with this member's path as their prefix.
        for m in self.members.iter_mut() {
            m.apply_specs(&my_path, fields);
        }
    }
}
