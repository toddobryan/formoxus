use dioxus::core::Element;

use crate::error::FormAccessError;
use crate::form::ErrorsByPath;
use crate::label_case::LabelCase;
use crate::nodes::{
    Drafts, Edit, Node, NodeContext, RawValue, SpecsByPath, ValuesByPath, owns, qualify,
};

#[derive(Clone, Debug)]
pub struct OptionNode {
    pub inner: Box<dyn Node>,
}

impl Node for OptionNode {
    fn name(&self) -> String {
        self.inner.name()
    }

    fn label(&self, case: LabelCase) -> Option<String> {
        self.inner.label(case)
    }

    fn render(&self, ctx: &NodeContext) -> Element {
        // The prefix passes through unchanged — this is a decorator, and
        // `name()` delegates inward, so qualifying here would double up. What
        // it *does* change is `required`: everything below is optional.
        self.inner.render(&ctx.optional())
    }

    fn raw_value(&self) -> String {
        self.inner.raw_value()
    }

    fn collect_values(&self, prefix: &str, out: &mut Vec<RawValue>) {
        self.inner.collect_values(prefix, out);
    }

    fn distribute_values(&mut self, prefix: &str, specs: &SpecsByPath, values: &ValuesByPath) {
        self.inner.distribute_values(prefix, specs, values);
    }

    fn paths(&self, prefix: &str, drafts: Drafts, out: &mut Vec<String>) {
        self.inner.paths(prefix, drafts, out);
    }

    fn validate(&self, prefix: &str, out: &mut ErrorsByPath) -> bool {
        !self.is_present() || self.inner.validate(prefix, out)
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }

    fn write_value_into<'p>(
        &self,
        partial: facet::Partial<'p>,
    ) -> Result<facet::Partial<'p>, facet::ReflectError> {
        if self.is_present() {
            self.inner.write_value_into(partial.begin_some()?)?.end()
        } else {
            partial.set_default()
        }
    }

    fn is_present(&self) -> bool {
        self.inner.is_present()
    }

    fn edit(
        &mut self,
        prefix: &str,
        edit: &Edit,
        specs: &SpecsByPath,
        errors: &mut ErrorsByPath,
    ) -> Result<Option<String>, FormAccessError> {
        // `name()` delegates to the inner, so the inner derives the same
        // qualified path from this same prefix. Passing it through unchanged is
        // what keeps `Option` from contributing a path segment of its own.
        self.inner.edit(prefix, edit, specs, errors)
    }

    fn absent_owner(&self, prefix: &str, path: &str) -> Option<String> {
        let my_path = qualify(prefix, &self.name());
        if !owns(&my_path, path) {
            None
        } else if !self.is_present() {
            Some(my_path)
        } else {
            self.inner.absent_owner(prefix, path)
        }
    }
}
