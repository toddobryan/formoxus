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
//!
//! `FormState` has no way to read a node's `meta`, so `distribute_files` is
//! seen through what it is for: `max_size:` and `accept:` (11e.3) refusing a
//! file, at the path of whichever leaf the table reached.

use super::across_the_wire;
use facet::Facet;
use formoxus::file::{FileType, FileUpload};
use formoxus::nodes::{FieldValidator, FileMeta, FilesByPath};
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

// ── A file's size and type (11e.2, 11e.3) ───────────────────────────────

const TOO_LARGE_2_MB: &str = "This file is too large. The largest allowed is 2 MB.";

/// One file in the table. The one place a `FileMeta` is spelled out, so 11e.4's
/// `stored` field changes only this.
fn meta(content_type: Option<&str>, size: u64) -> FileMeta {
    FileMeta {
        content_type: content_type.map(str::to_string),
        size,
    }
}

/// The table `Form` keeps (11f) or `accept_multipart` builds (11h).
fn files<const N: usize>(entries: [(&str, FileMeta); N]) -> FilesByPath {
    entries
        .into_iter()
        .map(|(path, meta)| (NodePath::new(path), meta))
        .collect()
}

fn pdf(size: u64) -> FileMeta {
    meta(Some("application/pdf"), size)
}

fn accept(list: &str) -> Vec<FileType> {
    FileType::parse_list(list).expect("the test's `accept` should parse")
}

/// `essay` limited to 2 MB, with a picked file at the limit named in the store.
fn limited_essay() -> FormState<Assignment> {
    let spec = FormSpec::default().with_max_size(&path!(Assignment.essay), 2_000_000);
    form_for(&assignment(existing("essay.pdf"), None), spec)
}

#[gtest]
fn a_file_over_max_size_is_refused() {
    let mut form = limited_essay();
    form.distribute_files(&files([("essay", pdf(2_000_001))]));
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "essay"),
        elements_are![eq(TOO_LARGE_2_MB)]
    );
}

/// The limit is the largest size allowed, not the first refused.
#[gtest]
fn a_file_of_exactly_max_size_passes() {
    let mut form = limited_essay();
    form.distribute_files(&files([("essay", pdf(2_000_000))]));
    expect_that!(form.validate(), some(anything()));
    expect_that!(field_errors(&form, "essay"), is_empty());
}

#[gtest]
fn without_max_size_any_size_passes() {
    let mut form = form_for(
        &assignment(existing("essay.pdf"), None),
        FormSpec::default(),
    );
    form.distribute_files(&files([("essay", pdf(50_000_000_000))]));
    expect_that!(form.validate(), some(anything()));
}

/// The limit as the message shows it: decimal units, at most one decimal place,
/// no trailing `.0`, and ROUNDED DOWN, so it never promises more than it
/// allows (2,050,000 bytes is "2 MB", not "2.1 MB").
#[gtest]
fn the_limit_is_shown_in_decimal_units_rounded_down() {
    for (max_size, shown) in [
        (2_000_000, "2 MB"),
        (1_500_000, "1.5 MB"),
        (2_050_000, "2 MB"),
        (2_099_999, "2 MB"),
        (10_250_000, "10.2 MB"),
        (500_000, "500 kB"),
        (1_999, "1.9 kB"),
        (1_000, "1 kB"),
        (999, "999 bytes"),
        (1, "1 byte"),
        (5_000_000_000, "5 GB"),
    ] {
        let spec = FormSpec::default().with_max_size(&path!(Assignment.essay), max_size);
        let mut form = form_for(&assignment(existing("essay.pdf"), None), spec);
        form.distribute_files(&files([("essay", pdf(max_size + 1))]));
        form.validate();
        expect_that!(
            field_errors(&form, "essay"),
            elements_are![eq(&format!(
                "This file is too large. The largest allowed is {shown}."
            ))],
            "{max_size}"
        );
    }
}

fn pdf_or_image_essay(name: &str) -> FormState<Assignment> {
    let spec = FormSpec::default().with_accept(&path!(Assignment.essay), accept(".pdf, image/*"));
    form_for(&assignment(existing(name), None), spec)
}

/// The list is the dev's entries in the dev's order, as `FileType` displays
/// them: lowercase.
#[gtest]
fn a_file_of_a_type_not_accepted_is_refused() {
    let mut form = pdf_or_image_essay("essay.docx");
    form.distribute_files(&files([(
        "essay",
        meta(
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
            1_000,
        ),
    )]));
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "essay"),
        elements_are![eq("This file type isn't allowed. Allowed: .pdf, image/*.")]
    );
}

/// By its name's extension, or by its content type: either is enough.
#[gtest]
fn a_file_of_an_accepted_type_passes() {
    let mut by_name = pdf_or_image_essay("essay.pdf");
    by_name.distribute_files(&files([("essay", meta(None, 1_000))]));
    expect_that!(by_name.validate(), some(anything()));

    let mut by_type = pdf_or_image_essay("photo");
    by_type.distribute_files(&files([("essay", meta(Some("image/heic"), 1_000))]));
    expect_that!(by_type.validate(), some(anything()));
}

#[gtest]
fn the_type_is_checked_against_the_name_in_the_store() {
    // `accepts` reads the name the node holds, which is the store's: a file
    // picked as `essay.PDF` is a PDF whatever its content type says.
    let mut form = pdf_or_image_essay("essay.PDF");
    form.distribute_files(&files([(
        "essay",
        meta(Some("application/octet-stream"), 1_000),
    )]));
    expect_that!(form.validate(), some(anything()));
}

#[gtest]
fn too_large_and_the_wrong_type_are_both_reported_in_that_order() {
    let spec = FormSpec::default()
        .with_max_size(&path!(Assignment.essay), 2_000_000)
        .with_accept(&path!(Assignment.essay), accept(".pdf"));
    let mut form = form_for(&assignment(existing("essay.docx"), None), spec);
    form.distribute_files(&files([("essay", meta(None, 3_000_000))]));
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "essay"),
        elements_are![
            eq(TOO_LARGE_2_MB),
            eq("This file type isn't allowed. Allowed: .pdf.")
        ]
    );
}

/// As a `FieldNode` does: the dev's validator runs only on a value formoxus's
/// own checks passed.
#[gtest]
fn a_devs_validator_does_not_run_on_a_file_already_refused() {
    let spec = pdf_only().with_max_size(&path!(Assignment.essay), 2_000_000);
    let mut form = form_for(&assignment(existing("essay.docx"), None), spec);
    form.distribute_files(&files([("essay", meta(None, 3_000_000))]));
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "essay"),
        elements_are![eq(TOO_LARGE_2_MB)]
    );
}

/// An `Existing` file has no size or type to check, so a limit set for new
/// files never refuses the one already kept.
#[gtest]
fn an_existing_file_is_not_checked() {
    let spec = FormSpec::default()
        .with_max_size(&path!(Assignment.essay), 1)
        .with_accept(&path!(Assignment.essay), accept("image/*"));
    let mut form = form_for(&assignment(existing("essay.pdf"), None), spec);
    form.distribute_files(&FilesByPath::default());
    expect_that!(
        form.validate(),
        some(eq(&assignment(existing("essay.pdf"), None)))
    );
}

// ── The table replaces, and is always whole (11e.2) ─────────────────────

/// The Reset case from the plan: the name in the store never changes, so only
/// the table's losing the file can drop its `meta`.
#[gtest]
fn a_path_missing_from_the_table_drops_the_files_meta() {
    let mut form = limited_essay();
    form.distribute_files(&files([("essay", pdf(10_000_000))]));
    expect_that!(form.validate(), none());

    form.distribute_files(&FilesByPath::default());
    expect_that!(
        form.validate(),
        some(eq(&assignment(existing("essay.pdf"), None))),
        "no file in the table is the kept file, `Existing`, and nothing to check"
    );
}

#[gtest]
fn a_later_table_replaces_an_earlier_file() {
    let mut form = limited_essay();
    form.distribute_files(&files([("essay", pdf(10_000_000))]));
    form.distribute_files(&files([("essay", pdf(1_000))]));
    expect_that!(form.validate(), some(anything()));
}

/// Values first, then files: a new name in the store is a new file, so it drops
/// what the node knew about the old one, and the table given after it is what
/// counts.
#[gtest]
fn files_distributed_after_a_new_name_apply_to_it() {
    let mut form = limited_essay();
    form.distribute_values(
        &[(NodePath::new("essay"), "big.pdf".to_string())]
            .into_iter()
            .collect(),
    );
    form.distribute_files(&files([("essay", pdf(10_000_000))]));
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "essay"),
        elements_are![eq(TOO_LARGE_2_MB)]
    );
}

/// A table entry at a path no file leaf has (an item since removed, a variant
/// since unchosen, a hand-made request) is ignored, never a panic.
#[gtest]
fn a_file_at_no_leafs_path_is_ignored() {
    let mut form = limited_essay();
    form.distribute_files(&files([
        ("title", pdf(10_000_000)),
        ("drafts.#7", pdf(10_000_000)),
        ("nowhere", pdf(10_000_000)),
    ]));
    expect_that!(form.validate(), some(anything()));
}

// ── The table reaches every file leaf (11e.2) ───────────────────────────

#[gtest]
fn the_table_reaches_an_optional_file() {
    let spec = FormSpec::default().with_max_size(&path!(Assignment.notes?), 2_000_000);
    let model = assignment(existing("essay.pdf"), Some(existing("notes.txt")));
    let mut form = form_for(&model, spec);
    form.distribute_files(&files([("notes", meta(Some("text/plain"), 3_000_000))]));
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "notes"),
        elements_are![eq(TOO_LARGE_2_MB)]
    );
}

#[gtest]
fn the_table_reaches_each_file_in_a_list_by_its_item_key() {
    let spec = FormSpec::default().with_max_size(&path!(Portfolio.drafts[]), 2_000_000);
    let model = Portfolio {
        drafts: vec![existing("draft-1.pdf"), existing("draft-2.pdf")],
    };
    let mut form = form_for(&model, spec);
    form.distribute_files(&files([
        ("drafts.#0", pdf(1_000)),
        ("drafts.#1", pdf(3_000_000)),
    ]));
    expect_that!(form.validate(), none());
    expect_that!(field_errors(&form, "drafts.#0"), is_empty());
    expect_that!(
        field_errors(&form, "drafts.#1"),
        elements_are![eq(TOO_LARGE_2_MB)]
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Course {
    unit: Assignment,
}

#[gtest]
fn the_table_reaches_a_file_in_a_nested_struct() {
    let spec = FormSpec::default().with_max_size(&path!(Course.unit.essay), 2_000_000);
    let model = Course {
        unit: assignment(existing("essay.pdf"), None),
    };
    let mut form = form_for(&model, spec);
    form.distribute_files(&files([("unit.essay", pdf(3_000_000))]));
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "unit.essay"),
        elements_are![eq(TOO_LARGE_2_MB)]
    );
}

#[derive(Facet, Clone, Debug, PartialEq)]
#[repr(u8)]
enum Work {
    Link { url: String },
    Upload { file: FileUpload },
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Coursework {
    work: Work,
}

fn uploaded_work() -> FormState<Coursework> {
    let spec =
        FormSpec::default().with_max_size(&path!(Coursework.work.$Work::Upload.file), 2_000_000);
    let model = Coursework {
        work: Work::Upload {
            file: existing("work.pdf"),
        },
    };
    form_for(&model, spec)
}

#[gtest]
fn the_table_reaches_a_file_in_the_chosen_variant() {
    let mut form = uploaded_work();
    form.distribute_files(&files([("work.$Upload.file", pdf(3_000_000))]));
    expect_that!(form.validate(), none());
    expect_that!(
        field_errors(&form, "work.$Upload.file"),
        elements_are![eq(TOO_LARGE_2_MB)]
    );
}

/// Only the chosen variant's nodes exist, so a file left in the table from
/// before the user switched variants has nowhere to land.
#[gtest]
fn a_file_under_an_unchosen_variant_is_ignored() {
    let mut form = uploaded_work();
    form.distribute_values(
        &[
            (NodePath::new("work"), "Link".to_string()),
            (
                NodePath::new("work.$Link.url"),
                "https://example.com".to_string(),
            ),
        ]
        .into_iter()
        .collect(),
    );
    form.distribute_files(&files([("work.$Upload.file", pdf(3_000_000))]));
    expect_that!(
        form.validate(),
        some(eq(&Coursework {
            work: Work::Link {
                url: "https://example.com".to_string()
            }
        }))
    );
}
