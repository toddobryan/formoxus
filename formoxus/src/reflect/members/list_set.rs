//! A list-typed member. Rows are members named by their index, which is what
//! makes `answer_choices.0.text` fall out of the ordinary `qualify` nesting.

use facet::{Partial, ReflectError};
use std::collections::HashMap;
use crate::error::{FormAccessError, FormError};
use crate::reflect::members::{FormMember, owns, qualify};

#[derive(Clone, Debug)]
pub struct ListSet {
    pub name: String,
    pub label: Option<String>,
    pub rows: Vec<Box<dyn FormMember>>,
    pub errors: Vec<FormError>,
}

impl FormMember for ListSet {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self) -> Option<String> {
        self.label.clone()
    }

    fn render(&self) -> String {
        self.rows
            .iter()
            .map(|r| r.render())
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn raw_value(&self) -> String {
        String::new()
    }

    fn collect_leaves(&self, prefix: &str, out: &mut Vec<(String, String)>) {
        let nested = qualify(prefix, &self.name);
        for r in self.rows.iter() {
            r.collect_leaves(&nested, out);
        }
    }

    fn apply_leaves(&mut self, prefix: &str, values: &HashMap<String, String>) {
        let nested = qualify(prefix, &self.name);
        for r in self.rows.iter_mut() {
            r.apply_leaves(&nested, values);
        }
    }

    fn validate(&mut self) {
        self.errors.clear();
        for r in self.rows.iter_mut() {
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
        for r in self.rows.iter() {
            partial = partial.begin_list_item()?;
            partial = r.write_value_into(partial)?;
            partial = partial.end()?;
        }
        Ok(partial)
    }
    
    fn is_present(&self) -> bool {
        !self.rows.is_empty() && self.rows.iter().any(|fm| fm.is_present())
    }
    
    fn choose_variant(&mut self, prefix: &str, path: &str, variant: &str) -> Result<(), FormAccessError> {
        let nested = qualify(prefix, &self.name);
        if !owns(&nested, path) {
            return Err(FormAccessError(format!("no such path: {path}")));
        }
        // Paths are unique, so at most one child can own this one. Dispatching
        // by containment rather than trying each in turn is what lets a child's
        // real error ("no such variant") reach the caller intact.
        for m in self.rows.iter_mut() {
            if owns(&qualify(&nested, &m.name()), path) {
                return m.choose_variant(&nested, path, variant);
            }
        }
        Err(FormAccessError(format!("no such path: {path}")))
    }

    fn clear_errors(&mut self) {
        self.errors.clear();
        for r in self.rows.iter_mut() {
            r.clear_errors();
        }
    }
}
