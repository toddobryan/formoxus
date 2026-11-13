//! A `FileUpload` field, through the walk (`WIDGETS_PLAN.md` 11d-11e).
//!
//! `FileUpload` is an enum, so without 11d the walk would build a
//! `VariantNode` for it, a `<select>` of `Chosen`/`Received`/`Existing`. It
//! is one leaf instead, a `FileNode`, holding the file's name in the store.
//!
//! What these pin is the part with no file in it: a model's name in, a model
//! back out. Populated from a model, a node has no file to hand, so whatever
//! variant the model held comes back `Existing` (11e.5), which is Django's
//! "keep the current file". The sizes and content types that make `Chosen` and
//! `Received` arrive with `distribute_files` (11e.2, 11e.4).

use super::across_the_wire;
use facet::Facet;
use formoxus::file::FileUpload;
use formoxus::nodes::FieldValidator;
use formoxus::path::NodePath;
use formoxus::prelude::*;
use googletest::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Assignment {
    title: String,
    essay: FileUpload,
    notes: Option<FileUpload>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Portfolio {
    drafts: Vec<FileUpload>,
}

/// A dev's own `S`, standing in for an S3 key: the walk must know a
/// `FileUpload` whatever it is generic over.
#[derive(Facet, Clone, Debug, PartialEq)]
struct Report {
    file: FileUpload<String>,
}

fn existing(name: &str) -> FileUpload {
    FileUpload::Existing {
        name: name.to_string(),
    }
}

fn assignment(essay: FileUpload, notes: Option<FileUpload>) -> Assignment {
    Assignment {
        title: "Unit 1".to_string(),
        essay,
        notes,
    }
}

fn paths_of<T>(form: &FormState<T>) -> Vec<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    form.collect_values()
        .into_iter()
        .map(|(p, _)| p.to_string())
        .collect()
}

fn value_at<T>(form: &FormState<T>, path: &str) -> Option<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    form.collect_values()
        .into_iter()
        .find(|(p, _)| p == path)
        .map(|(_, v)| v)
}

fn field_errors<T>(form: &FormState<T>, path: &str) -> Vec<String>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    form.collect_errors()
        .fields
        .get(path)
        .map(|found| found.iter().map(|m| m.0.clone()).collect())
        .unwrap_or_default()
}

// ── One leaf, not an enum ────────────────────────────────────────────────

#[gtest]
fn a_file_is_one_leaf_at_the_fields_own_path() {
    let form = empty_form::<Assignment>(FormSpec::default());
    expect_that!(
        paths_of(&form),
        elements_are![eq("title"), eq("essay"), eq("notes")],
        "no `essay.$Chosen`, no `essay.name`: the variant is not the user's to pick"
    );
}

#[gtest]
fn a_file_with_a_devs_own_s_is_one_leaf_too() {
    let form = empty_form::<Report>(FormSpec::default());
    expect_that!(paths_of(&form), elements_are![eq("file")]);
}

#[gtest]
fn a_populated_file_holds_its_name_in_the_store() {
    let form = form_for(
        &assignment(existing("essay.pdf"), None),
        FormSpec::default(),
    );
    expect_that!(value_at(&form, "essay"), some(eq("essay.pdf")));
    expect_that!(value_at(&form, "notes"), some(eq("")));
}

// ── Round trips ──────────────────────────────────────────────────────────

#[gtest]
fn an_existing_file_round_trips() {
    let model = assignment(existing("essay.pdf"), Some(existing("notes.txt")));
    let mut form = form_for(&model, FormSpec::default());
    expect_that!(form.validate(), some(eq(&model)));
    expect_that!(across_the_wire(&model), ok(eq(&model)));
}

#[gtest]
fn an_optional_file_left_empty_is_none() {
    let model = assignment(existing("essay.pdf"), None);
    let mut form = form_for(&model, FormSpec::default());
    expect_that!(form.validate(), some(eq(&model)));
    expect_that!(across_the_wire(&model), ok(eq(&model)));
}

#[gtest]
fn a_list_of_files_round_trips() {
    let model = Portfolio {
        drafts: vec![existing("draft-1.pdf"), existing("draft-2.pdf")],
    };
    let mut form = form_for(&model, FormSpec::default());
    expect_that!(form.validate(), some(eq(&model)));
    expect_that!(across_the_wire(&model), ok(eq(&model)));
}

#[gtest]
fn a_chosen_file_in_the_model_comes_back_existing() {
    // A model has no file to hand, only its name, so the form keeps it.
    let chosen = FileUpload::Chosen {
        name: "essay.pdf".to_string(),
        content_type: Some("application/pdf".to_string()),
        size: 1234,
    };
    let mut form = form_for(&assignment(chosen, None), FormSpec::default());
    expect_that!(
        form.validate(),
        some(eq(&assignment(existing("essay.pdf"), None)))
    );
}

#[gtest]
fn a_received_file_in_the_model_comes_back_existing() {
    // Its stored output (here an S3 key) is the dev's record of where the
    // file is; the form keeps only the name, which never says where.
    let received = Report {
        file: FileUpload::Received {
            name: "report.pdf".to_string(),
            content_type: None,
            size: 99,
            stored: "uploads/7f3a".to_string(),
        },
    };
    let mut form = form_for(&received, FormSpec::default());
    expect_that!(
        form.validate(),
        some(eq(&Report {
            file: FileUpload::Existing {
                name: "report.pdf".to_string()
            }
        }))
    );
}

// ── Submitted values ─────────────────────────────────────────────────────

#[gtest]
fn a_submitted_name_builds_an_existing_file() {
    let mut form = empty_form::<Assignment>(FormSpec::default());
    form.distribute_form_values(&[
        (NodePath::new("title"), "Unit 1".to_string()),
        (NodePath::new("essay"), "essay.pdf".to_string()),
    ]);
    expect_that!(
        form.validate(),
        some(eq(&assignment(existing("essay.pdf"), None)))
    );
}

#[gtest]
fn a_missing_file_is_required_like_any_other_field() {
    let mut form = empty_form::<Assignment>(FormSpec::default());
    form.distribute_form_values(&[(NodePath::new("title"), "Unit 1".to_string())]);
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "essay"),
        elements_are![eq("This field is required.")]
    );
    expect_that!(
        field_errors(&form, "notes"),
        is_empty(),
        "an optional file left empty is not missing"
    );
}

// ── A dev's own check ────────────────────────────────────────────────────

fn pdf_only() -> FormSpec<Assignment> {
    FormSpec::default().with_field_validator(
        &path!(Assignment.essay),
        FieldValidator::new(|essay: &FileUpload| {
            let is_pdf = std::path::Path::new(essay.name())
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));
            if is_pdf {
                Vec::new()
            } else {
                vec![ValidationMessage::from("A PDF, please.")]
            }
        }),
    )
}

#[gtest]
fn a_validator_sees_the_typed_file() {
    let mut form = form_for(&assignment(existing("essay.docx"), None), pdf_only());
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "essay"),
        elements_are![eq("A PDF, please.")]
    );

    let mut form = form_for(&assignment(existing("essay.pdf"), None), pdf_only());
    expect_that!(form.validate(), some(anything()));
}

#[gtest]
fn a_validator_does_not_run_on_a_missing_file() {
    let mut form = empty_form::<Assignment>(pdf_only());
    form.distribute_form_values(&[(NodePath::new("title"), "Unit 1".to_string())]);
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "essay"),
        elements_are![eq("This field is required.")]
    );
}
