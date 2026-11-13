use dioxus::prelude::*;
use facet::{Facet, Partial, ReflectError, Shape};

use crate::{
    error::{FormAccessError, ValidationMessage}, field_attrs::FieldAttrs, file::{FileType, FileUpload}, form::ErrorsByPath, label_case::LabelCase, nodes::{
        Drafts, Edit, FieldValidator, Node, NodeContext, RawValue, SpecsByPath, ValuesByPath,
        default_label,
    }, path::NodePath, widgets::{AnyWidget, Choice},
};

#[derive(Clone, Debug)]
pub struct FileNode {
    pub name: String,
    pub label: Option<String>,
    pub help_text: Option<String>,
    pub validator: Option<FieldValidator>,
    pub optional: bool,
    pub widget: AnyWidget,
    pub attrs: FieldAttrs,
    pub file_name: String,
    pub max_size: Option<u64>,
    pub accept: Vec<FileType>,
    pub meta: Option<FileMeta>,
}

#[derive(Clone, Debug)]
pub struct FileMeta {
    pub content_type: Option<String>,
    pub size: u64,
}

pub(crate) fn is_file_upload(shape: &Shape) -> bool {
    shape.decl_id == <FileUpload as Facet>::SHAPE.decl_id
}

impl Node for FileNode {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn label(&self, case: LabelCase) -> Option<String> {
        self.label
            .clone()
            .or_else(|| default_label(&self.name, case))
    }

    fn render(&self, _ctx: &NodeContext) -> Element {
        VNode::empty()
    }

    fn raw_value(&self) -> String {
        self.file_name.clone()
    }

    fn collect_values(&self, prefix: &NodePath, out: &mut Vec<RawValue>) {
        out.push(RawValue::Leaf {
            path: prefix.join(&self.name()),
            data: self.raw_value(),
        });
    }

    fn distribute_values(
        &mut self,
        prefix: &NodePath,
        _specs: &SpecsByPath,
        values: &ValuesByPath,
    ) {
        let Some(raw) = values.get(&prefix.join(&self.name)) else {
            return;
        };
        if raw != &self.file_name {
            self.meta = None;
        }
        self.file_name.clone_from(raw);
    }

    fn validate(&self, prefix: &NodePath, out: &mut ErrorsByPath) -> bool {
        let mut errors: Vec<ValidationMessage> = Vec::new();

        if self.file_name.is_empty() {
            errors.push(ValidationMessage("This field is required.".to_string()));
        }

        if let Some(_meta) = &self.meta {
            // TODO: check max_size and accept
        }

        if errors.is_empty() {
            errors.extend(
                self.validator
                    .as_ref()
                    .map(|v| v.messages_for(self))
                    .unwrap_or_default(),
            );
        }
        let is_valid = errors.is_empty();
        out.insert(prefix.join(&self.name), errors);
        is_valid
    }

    fn paths(&self, prefix: &crate::path::NodePath, _drafts: Drafts, out: &mut Vec<NodePath>) {
        out.push(prefix.join(&self.name));
    }

    fn clone_box(&self) -> Box<dyn Node> {
        Box::new(self.clone())
    }

    fn write_value_into<'p>(&self, partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        // TODO: handle chosen and Received when we handle meta
        partial
            .select_variant_named("Existing")?
            .set_field("name", self.file_name.clone())
    }

    fn is_present(&self) -> bool {
        !self.file_name.is_empty()
    }

    fn edit(
        &mut self,
        prefix: &NodePath,
        edit: &Edit,
        _specs: &SpecsByPath,
        _errors: &mut ErrorsByPath,
    ) -> Result<Option<NodePath>, FormAccessError> {
        // A leaf can only ever be the wrong answer, but which wrong answer is
        // worth saying: hitting a real field means the caller's path was right
        // and its *expectation* was wrong.
        let path = edit.path();
        Err(if *path == prefix.join(&self.name) {
            // The article has to travel with the noun, so this carries both.
            let wanted = match edit {
                Edit::ChooseVariant { .. } => "an enum",
                Edit::AddItem { .. } | Edit::RemoveItem { .. } | Edit::MoveItem { .. } => "a list",
                Edit::EditItem { .. } => "a set",
            };
            FormAccessError(format!("{path} is a field, not {wanted}"))
        } else {
            FormAccessError(format!("{path} is a field, so edits cannot be applied"))
        })
    }

    fn absent_owner(&self, _prefix: &NodePath, _path: &NodePath) -> Option<NodePath> {
        None
    }

    fn choices(&self, _case: LabelCase) -> Option<Vec<Choice>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use facet::Facet;
    use googletest::prelude::*;

    use super::is_file_upload;
    use crate::file::{FileUpload, TemporaryUploadedFile};

    /// The same name and variants as formoxus's own, declared somewhere else.
    mod lookalike {
        #[derive(facet::Facet)]
        #[repr(u8)]
        #[allow(dead_code)]
        pub(super) enum FileUpload {
            Existing { name: String },
        }
    }

    #[gtest]
    fn every_s_is_a_file_upload() {
        expect_that!(
            is_file_upload(<FileUpload<TemporaryUploadedFile>>::SHAPE),
            eq(true)
        );
        expect_that!(is_file_upload(<FileUpload<String>>::SHAPE), eq(true));
        expect_that!(
            is_file_upload(<FileUpload<std::path::PathBuf>>::SHAPE),
            eq(true)
        );
    }

    #[gtest]
    fn an_option_around_one_is_not_peeled_here() {
        // The walk peels an `Option` before it asks, so this need not.
        expect_that!(is_file_upload(<Option<FileUpload>>::SHAPE), eq(false));
    }

    #[gtest]
    fn a_dev_type_of_the_same_name_is_not_one() {
        // `decl_id` comes from where a type is declared, not what it is called.
        expect_that!(is_file_upload(lookalike::FileUpload::SHAPE), eq(false));
        expect_that!(is_file_upload(String::SHAPE), eq(false));
    }
}
