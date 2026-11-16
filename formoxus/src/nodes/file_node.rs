use dioxus::prelude::*;
use facet::{Facet, Partial, ReflectError, Shape};
use formoxus_spec::FileType;

use crate::{
    error::{FormAccessError, ValidationMessage},
    field_attrs::FieldAttrs,
    file::{FileMeta, FileUpload, format_size},
    form::ErrorsByPath,
    label_case::LabelCase,
    nodes::{
        Drafts, Edit, FieldValidator, FilesByPath, Node, NodeContext, RawValue, SpecsByPath,
        ValuesByPath, default_label,
    },
    path::NodePath,
    widgets::{AnyWidget, Choice},
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

    fn distribute_files(&mut self, prefix: &NodePath, files: &FilesByPath) {
        self.meta = files.get(&prefix.join(&self.name)).cloned();
    }

    fn validate(&self, prefix: &NodePath, out: &mut ErrorsByPath) -> bool {
        let mut errors: Vec<ValidationMessage> = Vec::new();

        if self.file_name.is_empty() {
            errors.push(ValidationMessage("This field is required.".to_string()));
        }

        if let Some(meta) = &self.meta {
            if let Some(max) = &self.max_size
                && meta.size > *max
            {
                errors.push(ValidationMessage(format!(
                    "This file is too large. The largest allowed is {}.",
                    format_size(*max)
                )));
            }

            if !FileType::accepts(&self.accept, &self.file_name, meta.content_type.as_deref()) {
                let accepted = self
                    .accept
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                errors.push(ValidationMessage(format!(
                    "This file type isn't allowed. Allowed: {}.",
                    accepted.join(", ")
                )));
            }
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
        match &self.meta {
            None => partial
                .select_variant_named("Existing")?
                .set_field("name", self.file_name.clone()),
            Some(meta) => match &meta.stored {
                None => partial
                    .select_variant_named("Chosen")?
                    .set_field("name", self.file_name.clone())?
                    .set_field("content_type", meta.content_type.clone())?
                    .set_field("size", meta.size),
                Some(stored) => partial
                    .select_variant_named("Received")?
                    .set_field("name", self.file_name.clone())?
                    .set_field("content_type", meta.content_type.clone())?
                    .set_field("size", meta.size)?
                    .begin_field("stored")
                    .and_then(|p| stored.write_into(p))?
                    .end(),
            },
        }
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

    // ── `Received`, which only a `StoredFile` makes (11e.4) ─────────────
    //
    // Here and not in the suite: `StoredFile::new` is `pub(crate)`, since only
    // `accept_multipart` (11h) stores files, so a test outside the crate can't
    // make one until 11h does it for real.

    use crate::file::{FileMeta, StoredFile};
    use crate::form::{FormSpec, form_for};
    use crate::nodes::FilesByPath;
    use crate::path::NodePath;

    /// A dev's own `S`, standing in for an S3 key, as in the suite's `Report`.
    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Report {
        file: FileUpload<String>,
    }

    #[derive(Facet, Clone, Debug, PartialEq)]
    struct Essay {
        essay: FileUpload,
    }

    fn received_table(path: &str, stored: StoredFile) -> FilesByPath {
        [(
            NodePath::new(path),
            FileMeta {
                content_type: Some("application/pdf".to_string()),
                size: 99,
                stored: Some(stored),
            },
        )]
        .into_iter()
        .collect()
    }

    fn existing_report() -> Report {
        Report {
            file: FileUpload::Existing {
                name: "report.pdf".to_string(),
            },
        }
    }

    #[gtest]
    fn a_stored_file_is_received_with_the_handlers_output() {
        let mut form = form_for(&existing_report(), FormSpec::default());
        form.distribute_files(&received_table(
            "file",
            StoredFile::new("uploads/7f3a".to_string()),
        ));
        expect_that!(
            form.validate(),
            some(eq(&Report {
                file: FileUpload::Received {
                    name: "report.pdf".to_string(),
                    content_type: Some("application/pdf".to_string()),
                    size: 99,
                    stored: "uploads/7f3a".to_string(),
                },
            }))
        );
    }

    /// With the default `S`: the model gets a clone of the handler's
    /// `TemporaryUploadedFile`, which shares its guard, so it IS the stored
    /// file (`PartialEq` compares by `Arc::ptr_eq`), not a copy of the bytes.
    #[gtest]
    fn a_received_temporary_file_is_the_handlers_own() {
        let path =
            std::env::temp_dir().join(format!("formoxus-received-test-{}", std::process::id()));
        let temporary = TemporaryUploadedFile::new(path);
        let model = Essay {
            essay: FileUpload::Existing {
                name: "essay.pdf".to_string(),
            },
        };
        let mut form = form_for(&model, FormSpec::default());
        form.distribute_files(&received_table("essay", StoredFile::new(temporary.clone())));
        let received = form.validate();
        expect_that!(
            received,
            some(eq(&Essay {
                essay: FileUpload::Received {
                    name: "essay.pdf".to_string(),
                    content_type: Some("application/pdf".to_string()),
                    size: 99,
                    stored: temporary,
                },
            }))
        );
    }

    /// The mismatch decision BB catches first, met here as the assert that
    /// backs it: a dev bug, so a panic naming the handler's type.
    #[gtest]
    #[should_panic(expected = "but the upload handler stores `u64`")]
    fn a_stored_file_of_the_wrong_type_panics_naming_it() {
        let mut form = form_for(&existing_report(), FormSpec::default());
        form.distribute_files(&received_table("file", StoredFile::new(42_u64)));
        let _ = form.validate();
    }

    /// The value is a `dyn Any` and the pointer an address, so the type's name
    /// is what `Debug` shows.
    #[gtest]
    fn a_stored_file_debugs_as_its_type_name() {
        expect_that!(
            format!("{:?}", StoredFile::new("key".to_string())),
            eq(r#"StoredFile { type_name: "alloc::string::String", .. }"#)
        );
    }
}
