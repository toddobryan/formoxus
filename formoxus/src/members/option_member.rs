use std::collections::HashMap;

use dioxus::core::Element;

use crate::error::{FieldError, FormAccessError};
use crate::label_case::LabelCase;
use crate::members::{Edit, FieldSpecs};
use crate::{FormMember, RenderCtx};

#[derive(Clone, Debug)]
pub struct OptionMember {
    pub inner: Box<dyn FormMember>,
}

impl FormMember for OptionMember {
    fn name(&self) -> String {
        self.inner.name()
    }

    fn label(&self, case: LabelCase) -> Option<String> {
        self.inner.label(case)
    }

    fn render(&self, ctx: &RenderCtx) -> Element {
        // The prefix passes through unchanged — this is a decorator, and
        // `name()` delegates inward, so qualifying here would double up. What
        // it *does* change is `required`: everything below is optional.
        self.inner.render(&ctx.optional())
    }

    fn raw_value(&self) -> String {
        self.inner.raw_value()
    }

    fn collect_leaves(&self, prefix: &str, out: &mut Vec<(String, String)>) {
        self.inner.collect_leaves(prefix, out);
    }

    fn collect_errors(&self, prefix: &str, out: &mut crate::form::FieldErrors) {
        self.inner.collect_errors(prefix, out);
    }

    fn apply_leaves(&mut self, prefix: &str, values: &HashMap<String, String>) {
        self.inner.apply_leaves(prefix, values);
    }

    fn validate(&mut self) {
        if self.is_present() {
            self.inner.validate()
        } else {
            self.inner.clear_errors()
        }
    }

    fn has_errors(&self) -> bool {
        self.inner.has_errors()
    }

    fn clone_box(&self) -> Box<dyn FormMember> {
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

    fn edit(&mut self, prefix: &str, edit: &Edit) -> Result<(), FormAccessError> {
        // `name()` delegates to the inner, so the inner derives the same
        // qualified path from this same prefix. Passing it through unchanged is
        // what keeps `Option` from contributing a path segment of its own.
        self.inner.edit(prefix, edit)
    }

    fn push_field_error(
        &mut self,
        prefix: &str,
        path: &str,
        error: FieldError,
    ) -> Result<(), FormAccessError> {
        // Same pass-through as `edit`, for the same reason: a decorator, not a
        // path segment.
        self.inner.push_field_error(prefix, path, error)
    }

    fn clear_errors(&mut self) {
        self.inner.clear_errors();
    }

    fn apply_specs(&mut self, prefix: &str, fields: &FieldSpecs) {
        self.inner.apply_specs(prefix, fields);
    }
}
