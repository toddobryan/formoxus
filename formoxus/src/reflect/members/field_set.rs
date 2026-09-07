//! A struct-typed member: its fields, nested under its own name.

use dioxus::prelude::*;
use facet::{Partial, ReflectError};
use std::collections::HashMap;
use crate::error::{FormAccessError, FormError};
use crate::reflect::RenderCtx;
use crate::reflect::members::{
    Edit, FormMember, default_label, ensure_owned, no_such_path, owns, qualify,
};

#[derive(Clone, Debug)]
pub struct FieldSet {
    pub name: String,
    pub label: Option<String>,
    pub members: Vec<Box<dyn FormMember>>,
    pub errors: Vec<FormError>,
}

impl FormMember for FieldSet {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self) -> Option<String> {
        self.label.clone().or_else(|| default_label(&self.name))
    }

    fn render(&self, ctx: &RenderCtx) -> Element {
        let nested = ctx.nested(&self.name);
        let members_rendered = self.members.iter().map(|m| m.render(&nested));
        rsx! {
            fieldset { class: "field-set",
                if let Some(text) = self.label() {
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
    

    fn clear_errors(&mut self) {
        self.errors.clear();
        for m in self.members.iter_mut() {
            m.clear_errors();
        }
    }
}
