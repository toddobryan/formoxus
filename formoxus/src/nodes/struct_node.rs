//! A struct-typed node: its fields, nested under its own name.

use crate::error::{FormAccessError, ValidationMessage};
use crate::form::ErrorsByPath;
use crate::label_case::LabelCase;
use crate::nodes::{
    Edit, Node, SpecsByPath, ValuesByPath, default_label, ensure_owned, no_such_path, owns, qualify,
};
use crate::nodes::{NodeContext, RawValue};
use crate::widgets::{FieldErrors, described_by, field_help};
use dioxus::prelude::*;
use facet::{Partial, ReflectError};
use formoxus_spec::FixedKey;

#[derive(Clone, Debug)]
pub struct StructNode {
    pub name: String,
    pub optional: bool,
    pub label: Option<String>,
    /// Its hint (`help_text:`), under the legend.
    pub help_text: Option<String>,
    /// Its own check (`validator:`), over the whole struct once every field
    /// is valid.
    pub validator: Option<crate::nodes::FieldValidator>,
    pub nodes: Vec<Box<dyn Node>>,
    pub errors: Vec<ValidationMessage>,
}

impl Node for StructNode {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self, case: LabelCase) -> Option<String> {
        self.label
            .clone()
            .or_else(|| default_label(&self.name, case))
    }

    fn render(&self, ctx: &NodeContext) -> Element {
        let nested = ctx.nested(&self.name);
        let nodes_rendered = self.nodes.iter().map(|m| m.render(&nested));
        // `here`, not `within`: a bad field is its own error, not the struct's.
        let invalid = self.has_errors_here();
        let fieldset_class = ctx.classes.fixed_class(FixedKey::Struct, invalid);
        let legend_class = ctx.classes.fixed_class(FixedKey::StructLegend, false);
        let id = ctx.id(&self.name);
        let describedby = described_by(&id, self.help_text.is_some(), invalid);
        rsx! {
            fieldset { class: fieldset_class,
                aria_describedby: describedby,
                if let Some(text) = self.label(ctx.label_case) {
                    legend { class: legend_class, "{text}" }
                }
                {field_help(&id, self.help_text.as_deref(), &ctx.classes)}
                // Its own errors under its name, as a list's are, before its
                // fields: `aria-describedby` on a group varies between screen
                // readers, so reading order has to carry them too.
                FieldErrors {
                    field_id: id.clone(),
                    errors: self.errors.clone(),
                    classes: ctx.classes.clone(),
                }
                { nodes_rendered.into_iter() }
            }
        }
    }

    fn validate(&mut self) {
        self.errors.clear();
        for m in &mut self.nodes {
            m.validate();
        }
        if !self.has_errors_within() {
            let messages = self
                .validator
                .as_ref()
                .map(|validator| validator.messages_for(self))
                .unwrap_or_default();
            self.errors.extend(messages);
        }
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }

    fn has_errors_within(&self) -> bool {
        self.has_errors_here() || self.nodes.iter().any(|m| m.has_errors_within())
    }

    fn has_errors_here(&self) -> bool {
        !self.errors.is_empty()
    }

    fn raw_value(&self) -> String {
        // A struct isn't a scalar — it has no single input of its own.
        String::new()
    }

    fn collect_values(&self, prefix: &str, out: &mut Vec<RawValue>) {
        let nested = qualify(prefix, &self.name);
        for m in &self.nodes {
            m.collect_values(&nested, out);
        }
    }

    fn collect_errors(&self, prefix: &str, out: &mut ErrorsByPath) {
        let nested = qualify(prefix, &self.name);
        // Its own, at its own path, as a list's are: a validator's, a
        // server's, or a form validator's `path!(Model.address)`.
        if !self.errors.is_empty() {
            out.insert(nested.clone(), self.errors.clone());
        }
        for m in &self.nodes {
            m.collect_errors(&nested, out);
        }
    }

    fn distribute_values(&mut self, prefix: &str, specs: &SpecsByPath, values: &ValuesByPath) {
        let nested = qualify(prefix, &self.name);
        for m in &mut self.nodes {
            m.distribute_values(&nested, specs, values);
        }
    }

    fn distribute_errors(&mut self, prefix: &str, errors: &mut ErrorsByPath) {
        let nested = qualify(prefix, &self.name);
        if let Some(found) = errors.shift_remove(&nested) {
            self.errors.extend(found);
        }
        for m in &mut self.nodes {
            m.distribute_errors(&nested, errors);
        }
    }

    fn write_value_into<'p>(&self, mut partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        for m in &self.nodes {
            partial = m.write_into(partial)?;
        }
        Ok(partial)
    }

    fn is_present(&self) -> bool {
        self.nodes.iter().any(|fm| fm.is_present())
    }

    fn edit(
        &mut self,
        prefix: &str,
        edit: &Edit,
        specs: &SpecsByPath,
    ) -> Result<Option<String>, FormAccessError> {
        let nested = qualify(prefix, &self.name);
        let path = edit.path();
        ensure_owned(&nested, path)?;

        // Paths are unique, so at most one child can own this one. Dispatching
        // by containment rather than trying each in turn is what lets a child's
        // real error ("no such variant") reach the caller intact.
        for m in &mut self.nodes {
            if owns(&qualify(&nested, &m.name()), path) {
                return m.edit(&nested, edit, specs);
            }
        }
        Err(no_such_path(path))
    }

    fn clear_errors(&mut self) {
        self.errors.clear();
        for m in &mut self.nodes {
            m.clear_errors();
        }
    }
}
