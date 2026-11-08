//! A struct-typed node: its fields, nested under its own name.

use crate::error::FormAccessError;
use crate::form::ErrorsByPath;
use crate::label_case::LabelCase;
use crate::nodes::{
    Drafts, Edit, Node, SpecsByPath, ValuesByPath, default_label, ensure_owned, no_such_path, owns,
    qualify,
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
        let path = ctx.path(&self.name);

        let own_errors = ctx.errors_at(&path);
        let invalid = !own_errors.is_empty();

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
                    paths: vec![path],
                    errors: ctx.errors,
                    classes: ctx.classes.clone(),
                }
                { nodes_rendered.into_iter() }
            }
        }
    }

    fn validate(&self, prefix: &str, out: &mut ErrorsByPath) -> bool {
        let my_path = qualify(prefix, &self.name);
        let mut is_valid = true;
        for m in &self.nodes {
            is_valid = m.validate(&my_path, out) && is_valid;
        }
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

    fn distribute_values(&mut self, prefix: &str, specs: &SpecsByPath, values: &ValuesByPath) {
        let nested = qualify(prefix, &self.name);
        for m in &mut self.nodes {
            m.distribute_values(&nested, specs, values);
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
        errors: &mut ErrorsByPath,
    ) -> Result<Option<String>, FormAccessError> {
        let nested = qualify(prefix, &self.name);
        let path = edit.path();
        ensure_owned(&nested, path)?;

        // Paths are unique, so at most one child can own this one. Dispatching
        // by containment rather than trying each in turn is what lets a child's
        // real error ("no such variant") reach the caller intact.
        for m in &mut self.nodes {
            if owns(&qualify(&nested, &m.name()), path) {
                return m.edit(&nested, edit, specs, errors);
            }
        }
        Err(no_such_path(path))
    }

    fn paths(&self, prefix: &str, drafts: Drafts, out: &mut Vec<String>) {
        let nested = qualify(prefix, &self.name);
        out.push(nested.clone());
        for n in &self.nodes {
            n.paths(&nested, drafts, out);
        }
    }

    fn absent_owner(&self, prefix: &str, path: &str) -> Option<String> {
        let nested = qualify(prefix, &self.name);
        if owns(&nested, path) {
            self.nodes
                .iter()
                .find_map(|n| n.absent_owner(&nested, path))
        } else {
            None
        }
    }
}
