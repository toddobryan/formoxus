//! `Submission::accept_multipart`, the server half of a submit with files
//! (`WIDGETS_PLAN.md` 11h, tested as 11i asks).
//!
//! Each test builds the request a browser would send, in memory: the wire
//! part first, then one part per file, through axum's own extractor (so with
//! its 2 MB default body limit), run under `futures::executor::block_on`.
//! The wire part is built from a model with `WireForm::from_model`, which is
//! what `to_multipart` sends for that model's values; the sizes it states
//! (decision AA) are added by hand, since only `to_multipart` sets them.
//!
//! The handler keeps each file in memory and logs every call, which is how a
//! test sees what was stored, aborted and deleted. Its `Stored` is the file's
//! text, so a model here holds `FileUpload<String>`; the last section runs the
//! real `TemporaryFileUploadHandler` on a model with the default `S`.
//!
//! A separate module from `multipart.rs` (the client half), which names a
//! model `Submission`.

use std::collections::VecDeque;
use std::future::{Future, ready};
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::task::Poll;

use dioxus_fullstack::body::Body;
use dioxus_fullstack::extract::FromRequest;
use dioxus_fullstack::http::{Request, header::CONTENT_TYPE};
use facet::Facet;
use formoxus::file::{
    FileType, FileUpload, FileUploadHandler, FileUploadWriter, Multipart,
    TemporaryFileUploadHandler,
};
use formoxus::path::NodePath;
use formoxus::prelude::*;
use futures::executor::block_on;
use googletest::prelude::*;

/// `WIRE_PART`, which is `pub(crate)` (decision MM): spelled out here, so a
/// change to it is a change a hand-built request would notice.
const WIRE_PART: &str = "__formoxus_wire_form";
const BOUNDARY: &str = "fx-suite-boundary";

const REQUIRED: &str = "This field is required.";
const NOT_FROM_FORMOXUS: &str = "Expected a Multipart value with wire_form as first field";
const TOO_LARGE_TOGETHER: &str = "These files are too large to send together.";
const BROKEN_OFF: &str = "The upload was interrupted. Please try again.";
const NOT_SAVED: &str = "This upload couldn't be saved. Please try again.";
const UNUSABLE_FILE_NAME: &str =
    "That file name can't be used. Please rename the file and try again.";

#[derive(Facet, Clone, Debug, PartialEq)]
struct Assignment {
    title: String,
    essay: FileUpload<String>,
    notes: Option<FileUpload<String>>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Portfolio {
    drafts: Vec<FileUpload<String>>,
}

/// The default `S`, for the real handler.
#[derive(Facet, Clone, Debug, PartialEq)]
struct OnDisk {
    title: String,
    essay: FileUpload,
}

// ── The handler ──────────────────────────────────────────────────────────

/// Keeps each file's bytes, as text, and logs `new_file`, `finish`, `abort`
/// and `delete`. `fail_at` makes `new_file` fail for that one path.
#[derive(Default)]
struct Memory {
    log: Arc<Mutex<Vec<String>>>,
    fail_at: Option<&'static str>,
}

impl Memory {
    fn failing_at(path: &'static str) -> Self {
        Self {
            fail_at: Some(path),
            ..Self::default()
        }
    }

    fn log(&self) -> Vec<String> {
        self.log.lock().expect("unpoisoned").clone()
    }
}

struct MemoryWriter {
    path: String,
    bytes: Vec<u8>,
    log: Arc<Mutex<Vec<String>>>,
}

impl FileUploadHandler for Memory {
    type Stored = String;
    type Writer = MemoryWriter;

    fn new_file(
        &self,
        path: &NodePath,
        name: &str,
        content_type: Option<&str>,
    ) -> impl Future<Output = io::Result<Self::Writer>> + Send {
        self.log.lock().expect("unpoisoned").push(format!(
            "new_file {path} {name} {}",
            content_type.unwrap_or("-")
        ));
        ready(if self.fail_at == Some(path.as_str()) {
            Err(io::Error::other("no space"))
        } else {
            Ok(MemoryWriter {
                path: path.to_string(),
                bytes: Vec::new(),
                log: Arc::clone(&self.log),
            })
        })
    }

    fn delete(&self, stored: Self::Stored) -> impl Future<Output = ()> + Send {
        self.log
            .lock()
            .expect("unpoisoned")
            .push(format!("delete {stored}"));
        ready(())
    }
}

impl FileUploadWriter for MemoryWriter {
    type Stored = String;

    fn write_chunk(&mut self, chunk: &[u8]) -> impl Future<Output = io::Result<()>> + Send {
        self.bytes.extend_from_slice(chunk);
        ready(Ok(()))
    }

    fn finish(self) -> impl Future<Output = io::Result<Self::Stored>> + Send {
        self.log
            .lock()
            .expect("unpoisoned")
            .push(format!("finish {}", self.path));
        ready(Ok(String::from_utf8_lossy(&self.bytes).into_owned()))
    }

    fn abort(self) -> impl Future<Output = ()> + Send {
        self.log
            .lock()
            .expect("unpoisoned")
            .push(format!("abort {}", self.path));
        ready(())
    }
}

// ── Building the request ─────────────────────────────────────────────────

/// One part. `filename` and `content_type` are a file part's.
fn part(name: &str, filename: Option<&str>, content_type: Option<&str>, bytes: &[u8]) -> Vec<u8> {
    let filename = filename.map_or(String::new(), |f| format!("; filename=\"{f}\""));
    let content_type = content_type.map_or(String::new(), |c| format!("Content-Type: {c}\r\n"));
    let head = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"{filename}\r\n\
         {content_type}\r\n"
    );
    let mut out = head.into_bytes();
    out.extend_from_slice(bytes);
    out.extend_from_slice(b"\r\n");
    out
}

/// The wire part for `model`, as `to_multipart` sends it, with `sizes` as
/// the stated file sizes.
fn wire_part<T>(model: &T, sizes: &[(&str, u64)]) -> Vec<u8>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    let mut json = serde_json::to_value(WireForm::from_model(model, FormSpec::default()))
        .expect("a wire form serializes");
    if !sizes.is_empty() {
        json["file_sizes"] = sizes
            .iter()
            .map(|(path, size)| ((*path).to_string(), serde_json::json!(size)))
            .collect::<serde_json::Map<_, _>>()
            .into();
    }
    part(WIRE_PART, None, None, json.to_string().as_bytes())
}

/// A file part at `path`, as a browser sends one.
fn file_part(path: &str, content_type: &str, bytes: &[u8]) -> Vec<u8> {
    part(path, Some("upload.bin"), Some(content_type), bytes)
}

fn pdf(path: &str, text: &str) -> Vec<u8> {
    file_part(path, "application/pdf", text.as_bytes())
}

/// The parts, then the closing boundary.
fn body(parts: &[Vec<u8>]) -> Vec<u8> {
    let mut out = parts.concat();
    out.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    out
}

/// `body` through axum's extractor, in 1,000-byte frames with a `Pending`
/// before each, as a socket delivers them. Without the `Pending`s, multer's
/// buffer takes the whole body on its first poll (see `file.rs`'s tests).
fn multipart<T>(body: &[u8]) -> Multipart<T> {
    let mut frames: VecDeque<Result<Vec<u8>, io::Error>> =
        body.chunks(1_000).map(|c| Ok(c.to_vec())).collect();
    let mut wait = true;
    let trickle = futures::stream::poll_fn(move |cx| {
        if wait {
            wait = false;
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            wait = true;
            Poll::Ready(frames.pop_front())
        }
    });
    let request = Request::builder()
        .header(
            CONTENT_TYPE,
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from_stream(trickle))
        .expect("a valid request");
    block_on(Multipart::<T>::from_request(request, &()))
        .unwrap_or_else(|_| panic!("a multipart request"))
}

fn accept_with<T, H>(
    spec: FormSpec<T>,
    body: &[u8],
    handler: &H,
    existing: Option<&T>,
) -> std::result::Result<T, FormErrors>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
    H: FileUploadHandler,
{
    block_on(Submission::accept_multipart(
        spec,
        multipart(body),
        handler,
        existing,
    ))
    .map(Submission::into_model)
}

fn accept<T>(
    body: &[u8],
    handler: &Memory,
    existing: Option<&T>,
) -> std::result::Result<T, FormErrors>
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static>,
{
    accept_with(FormSpec::default(), body, handler, existing)
}

/// Every field error that isn't empty, as `(path, messages)`.
fn field_errors(errors: &FormErrors) -> Vec<(String, Vec<String>)> {
    errors
        .fields
        .iter()
        .filter(|(_, found)| !found.is_empty())
        .map(|(path, found)| {
            (
                path.to_string(),
                found.iter().map(|m| m.0.clone()).collect(),
            )
        })
        .collect()
}

fn chosen(name: &str) -> FileUpload<String> {
    FileUpload::Chosen {
        name: name.to_string(),
        content_type: Some("application/pdf".to_string()),
        size: 0,
    }
}

fn existing(name: &str) -> FileUpload<String> {
    FileUpload::Existing {
        name: name.to_string(),
    }
}

fn received(name: &str, text: &str) -> FileUpload<String> {
    FileUpload::Received {
        name: name.to_string(),
        content_type: Some("application/pdf".to_string()),
        size: text.len() as u64,
        stored: text.to_string(),
    }
}

fn assignment(essay: FileUpload<String>, notes: Option<FileUpload<String>>) -> Assignment {
    Assignment {
        title: "Unit 1".to_string(),
        essay,
        notes,
    }
}

// ── Files received ───────────────────────────────────────────────────────

#[gtest]
fn a_file_is_received_into_the_model() {
    let handler = Memory::default();
    let body = body(&[
        wire_part(&assignment(chosen("essay.pdf"), None), &[]),
        pdf("essay", "my essay"),
    ]);
    let model = accept::<Assignment>(&body, &handler, None);
    expect_that!(
        model,
        ok(eq(&assignment(received("essay.pdf", "my essay"), None)))
    );
    expect_that!(
        handler.log(),
        elements_are![
            eq("new_file essay essay.pdf application/pdf"),
            eq("finish essay")
        ]
    );
}

#[gtest]
fn two_files_are_both_received() {
    let handler = Memory::default();
    let body = body(&[
        wire_part(
            &assignment(chosen("essay.pdf"), Some(chosen("notes.pdf"))),
            &[],
        ),
        pdf("essay", "my essay"),
        pdf("notes", "my notes"),
    ]);
    let model = accept::<Assignment>(&body, &handler, None);
    expect_that!(
        model,
        ok(eq(&assignment(
            received("essay.pdf", "my essay"),
            Some(received("notes.pdf", "my notes"))
        )))
    );
}

#[gtest]
fn the_size_is_the_one_counted_not_the_one_stated() {
    let handler = Memory::default();
    let body = body(&[
        wire_part(&assignment(chosen("essay.pdf"), None), &[("essay", 3)]),
        pdf("essay", "my essay"),
    ]);
    let model = accept::<Assignment>(&body, &handler, None).expect("accepted");
    let FileUpload::Received { size, .. } = model.essay else {
        panic!("expected Received, got {:?}", model.essay);
    };
    expect_that!(size, eq(8));
}

#[gtest]
fn the_content_type_is_the_parts_own() {
    let handler = Memory::default();
    let body = body(&[
        wire_part(&assignment(chosen("essay.txt"), None), &[]),
        file_part("essay", "text/plain", b"plain"),
    ]);
    let model = accept::<Assignment>(&body, &handler, None).expect("accepted");
    let FileUpload::Received { content_type, .. } = model.essay else {
        panic!("expected Received, got {:?}", model.essay);
    };
    expect_that!(content_type, some(eq("text/plain")));
}

#[gtest]
fn a_file_inside_a_list_item_is_received() {
    let handler = Memory::default();
    let model = Portfolio {
        drafts: vec![chosen("first.pdf")],
    };
    let wire = wire_part(&model, &[]);
    let body = body(&[wire, pdf("drafts.#0", "first draft")]);
    let accepted = accept::<Portfolio>(&body, &handler, None);
    expect_that!(
        accepted,
        ok(eq(&Portfolio {
            drafts: vec![received("first.pdf", "first draft")],
        })),
        "the values build the item before the files reach it (h.10's order)"
    );
}

// ── The file's name ──────────────────────────────────────────────────────

#[gtest]
fn the_name_is_the_wires_not_the_parts_filename() {
    let handler = Memory::default();
    let body = body(&[
        wire_part(&assignment(chosen("essay.pdf"), None), &[]),
        part(
            "essay",
            Some("something-else.bin"),
            Some("application/pdf"),
            b"my essay",
        ),
    ]);
    let model = accept::<Assignment>(&body, &handler, None);
    expect_that!(
        model,
        ok(field!(
            Assignment.essay,
            eq(&received("essay.pdf", "my essay"))
        ))
    );
}

#[gtest]
fn the_name_is_cleaned_before_the_handler_sees_it() {
    let handler = Memory::default();
    let body = body(&[
        wire_part(&assignment(chosen("../../units/essay.pdf"), None), &[]),
        pdf("essay", "my essay"),
    ]);
    let model = accept::<Assignment>(&body, &handler, None);
    expect_that!(
        model,
        ok(field!(
            Assignment.essay,
            eq(&received("essay.pdf", "my essay"))
        ))
    );
    expect_that!(
        handler.log(),
        contains(eq("new_file essay essay.pdf application/pdf"))
    );
}

#[gtest]
fn a_name_with_nothing_usable_left_is_a_form_error() {
    let handler = Memory::default();
    let body = body(&[
        wire_part(&assignment(chosen(".."), Some(chosen("notes.pdf"))), &[]),
        pdf("notes", "my notes"),
        pdf("essay", "my essay"),
    ]);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(
        errors.form_messages(),
        elements_are![eq(UNUSABLE_FILE_NAME)]
    );
    expect_that!(
        handler.log(),
        elements_are![
            eq("new_file notes notes.pdf application/pdf"),
            eq("finish notes"),
            eq("delete my notes")
        ],
        "the earlier file is deleted, the refused one never stored"
    );
}

// ── Refused before or while storing ─────────────────────────────────────

#[gtest]
fn a_stated_size_over_max_size_is_refused_before_storing() {
    let handler = Memory::default();
    let spec = FormSpec::default().with_max_size(&path!(Assignment.essay), 5);
    let body = body(&[
        wire_part(&assignment(chosen("essay.pdf"), None), &[("essay", 50)]),
        pdf("essay", "tiny"),
    ]);
    let errors = accept_with(spec, &body, &handler, None).expect_err("rejected");
    expect_that!(
        field_errors(&errors),
        elements_are![(
            eq("essay"),
            elements_are![starts_with("This file is too large.")]
        )]
    );
    expect_that!(handler.log(), is_empty(), "never reached `new_file`");
}

#[gtest]
fn a_counted_size_over_max_size_is_aborted() {
    let handler = Memory::default();
    let spec = FormSpec::default().with_max_size(&path!(Assignment.essay), 5);
    let body = body(&[
        wire_part(&assignment(chosen("essay.pdf"), None), &[("essay", 3)]),
        pdf("essay", "much longer than five bytes"),
    ]);
    let errors = accept_with(spec, &body, &handler, None).expect_err("rejected");
    expect_that!(
        field_errors(&errors),
        elements_are![(
            eq("essay"),
            elements_are![starts_with("This file is too large.")]
        )],
        "the stated 3 is not believed"
    );
    expect_that!(
        handler.log(),
        elements_are![
            eq("new_file essay essay.pdf application/pdf"),
            eq("abort essay")
        ]
    );
}

#[gtest]
fn a_type_the_field_wont_take_is_refused_before_storing() {
    let handler = Memory::default();
    let accept_pdf = FileType::parse_list(".pdf").expect("parses");
    let spec = FormSpec::default().with_accept(&path!(Assignment.essay), accept_pdf);
    let body = body(&[
        wire_part(&assignment(chosen("essay.txt"), None), &[]),
        file_part("essay", "text/plain", b"plain"),
    ]);
    let errors = accept_with(spec, &body, &handler, None).expect_err("rejected");
    expect_that!(
        field_errors(&errors),
        elements_are![(
            eq("essay"),
            elements_are![eq("This file type isn't allowed. Allowed: .pdf.")]
        )]
    );
    expect_that!(handler.log(), is_empty(), "never reached `new_file`");
}

// ── Parts that are ignored ───────────────────────────────────────────────

#[gtest]
fn a_part_that_isnt_a_file_leaf_is_ignored() {
    let handler = Memory::default();
    let body = body(&[
        wire_part(&assignment(chosen("essay.pdf"), None), &[]),
        pdf("title", "not a file"),
        pdf("nowhere", "no such field"),
        pdf("essay", "my essay"),
    ]);
    let model = accept::<Assignment>(&body, &handler, None);
    expect_that!(
        model,
        ok(eq(&assignment(received("essay.pdf", "my essay"), None)))
    );
    expect_that!(
        handler.log(),
        elements_are![
            eq("new_file essay essay.pdf application/pdf"),
            eq("finish essay")
        ]
    );
}

#[gtest]
fn a_part_whose_wire_name_is_empty_is_ignored() {
    let handler = Memory::default();
    let body = body(&[
        wire_part(&assignment(chosen(""), None), &[]),
        pdf("essay", "my essay"),
    ]);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(
        field_errors(&errors),
        elements_are![(eq("essay"), elements_are![eq(REQUIRED)])]
    );
    expect_that!(handler.log(), is_empty());
}

// ── The wire part (h.7) ──────────────────────────────────────────────────

#[gtest]
fn a_request_without_a_wire_part_is_a_form_error() {
    let handler = Memory::default();
    let body = body(&[pdf("essay", "my essay")]);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(errors.form_messages(), elements_are![eq(NOT_FROM_FORMOXUS)]);
    expect_that!(errors.fields, is_empty());
    expect_that!(handler.log(), is_empty());
}

#[gtest]
fn a_request_with_no_parts_is_a_form_error() {
    let handler = Memory::default();
    let errors = accept::<Assignment>(&body(&[]), &handler, None).expect_err("rejected");
    expect_that!(errors.form_messages(), elements_are![eq(NOT_FROM_FORMOXUS)]);
}

#[gtest]
fn a_wire_part_that_doesnt_parse_is_a_form_error() {
    let handler = Memory::default();
    let body = body(&[
        part(WIRE_PART, None, None, b"{not json"),
        pdf("essay", "my essay"),
    ]);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(errors.form_messages(), elements_are![eq(NOT_FROM_FORMOXUS)]);
    expect_that!(handler.log(), is_empty());
}

#[gtest]
#[should_panic(expected = "Expected file handler of type")]
fn a_handler_that_stores_another_type_panics() {
    // The model stores `String`s; this handler stores `TemporaryUploadedFile`s.
    let handler = TemporaryFileUploadHandler::new();
    let body = body(&[
        wire_part(&assignment(chosen("essay.pdf"), None), &[]),
        pdf("essay", "my essay"),
    ]);
    let _ = accept_with::<Assignment, _>(FormSpec::default(), &body, &handler, None);
}

// ── Failures that delete what was stored ─────────────────────────────────

#[gtest]
fn a_handler_failure_is_a_form_error_and_deletes_what_was_stored() {
    let handler = Memory::failing_at("essay");
    let body = body(&[
        wire_part(
            &assignment(chosen("essay.pdf"), Some(chosen("notes.pdf"))),
            &[],
        ),
        pdf("notes", "my notes"),
        pdf("essay", "my essay"),
    ]);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(errors.form_messages(), elements_are![eq(NOT_SAVED)]);
    expect_that!(
        handler.log(),
        elements_are![
            eq("new_file notes notes.pdf application/pdf"),
            eq("finish notes"),
            eq("new_file essay essay.pdf application/pdf"),
            eq("delete my notes")
        ]
    );
}

#[gtest]
fn a_request_past_the_body_limit_is_a_form_error_and_deletes_what_was_stored() {
    let handler = Memory::default();
    let big = "x".repeat(2_100_000);
    let body = body(&[
        wire_part(
            &assignment(chosen("essay.pdf"), Some(chosen("notes.pdf"))),
            &[],
        ),
        pdf("notes", "my notes"),
        pdf("essay", &big),
    ]);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(
        errors.form_messages(),
        elements_are![eq(TOO_LARGE_TOGETHER)]
    );
    expect_that!(errors.fields, is_empty());
    expect_that!(
        handler.log(),
        elements_are![
            eq("new_file notes notes.pdf application/pdf"),
            eq("finish notes"),
            eq("new_file essay essay.pdf application/pdf"),
            eq("abort essay"),
            eq("delete my notes")
        ]
    );
}

/// The limit tripping in `next_field`, not in a file part: an ignored part
/// is never read by `handle_file`, so the next `next_field` reads through it
/// and meets the 413 itself.
#[gtest]
fn the_body_limit_met_between_files_is_a_form_error_too() {
    let handler = Memory::default();
    let big = "x".repeat(2_100_000);
    let body = body(&[
        wire_part(
            &assignment(chosen("essay.pdf"), Some(chosen("notes.pdf"))),
            &[],
        ),
        pdf("notes", "my notes"),
        pdf("title", &big),
        pdf("essay", "my essay"),
    ]);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(
        errors.form_messages(),
        elements_are![eq(TOO_LARGE_TOGETHER)]
    );
    expect_that!(
        handler.log(),
        elements_are![
            eq("new_file notes notes.pdf application/pdf"),
            eq("finish notes"),
            eq("delete my notes")
        ]
    );
}

/// The limit tripping while the wire part itself is read, in h.7.
#[gtest]
fn a_wire_part_past_the_body_limit_is_a_form_error() {
    let handler = Memory::default();
    let model = Assignment {
        title: "x".repeat(2_100_000),
        essay: chosen("essay.pdf"),
        notes: None,
    };
    let body = body(&[wire_part(&model, &[]), pdf("essay", "my essay")]);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(
        errors.form_messages(),
        elements_are![eq(TOO_LARGE_TOGETHER)]
    );
    expect_that!(handler.log(), is_empty());
}

#[gtest]
fn a_request_that_breaks_off_is_a_form_error_and_deletes_what_was_stored() {
    let handler = Memory::default();
    // No closing boundary, and the last part cut off mid-file.
    let mut body = [
        wire_part(
            &assignment(chosen("essay.pdf"), Some(chosen("notes.pdf"))),
            &[],
        ),
        pdf("notes", "my notes"),
        pdf("essay", "my essay"),
    ]
    .concat();
    body.truncate(body.len() - 6);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(errors.form_messages(), elements_are![eq(BROKEN_OFF)]);
    expect_that!(
        handler.log(),
        elements_are![
            eq("new_file notes notes.pdf application/pdf"),
            eq("finish notes"),
            eq("new_file essay essay.pdf application/pdf"),
            eq("abort essay"),
            eq("delete my notes")
        ]
    );
}

#[gtest]
fn a_form_that_fails_validation_deletes_what_was_stored() {
    let handler = Memory::default();
    let model = Assignment {
        title: String::new(),
        essay: chosen("essay.pdf"),
        notes: None,
    };
    let body = body(&[wire_part(&model, &[]), pdf("essay", "my essay")]);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(
        field_errors(&errors),
        elements_are![(eq("title"), elements_are![eq(REQUIRED)])]
    );
    expect_that!(
        handler.log(),
        elements_are![
            eq("new_file essay essay.pdf application/pdf"),
            eq("finish essay"),
            eq("delete my essay")
        ]
    );
}

// ── Keeping the file on the record (h.9: decision V, with NN) ───────────

#[gtest]
fn a_file_kept_on_an_edit_stays_existing() {
    let handler = Memory::default();
    let record = assignment(existing("essay.pdf"), None);
    let body = body(&[wire_part(&record, &[])]);
    let model = accept(&body, &handler, Some(&record));
    expect_that!(model, ok(eq(&record)));
    expect_that!(handler.log(), is_empty());
}

#[gtest]
fn a_kept_file_takes_the_records_name_not_the_clients() {
    let handler = Memory::default();
    let record = assignment(existing("essay.pdf"), None);
    let claimed = assignment(existing("../../secret.pdf"), None);
    let body = body(&[wire_part(&claimed, &[])]);
    let model = accept(&body, &handler, Some(&record));
    expect_that!(model, ok(eq(&record)), "NN: the record's name wins");
}

#[gtest]
fn a_claim_on_a_create_counts_as_missing() {
    let handler = Memory::default();
    let claimed = assignment(existing("essay.pdf"), None);
    let body = body(&[wire_part(&claimed, &[])]);
    let errors = accept::<Assignment>(&body, &handler, None).expect_err("rejected");
    expect_that!(
        field_errors(&errors),
        elements_are![(eq("essay"), elements_are![eq(REQUIRED)])]
    );
}

#[gtest]
fn an_optional_claim_the_record_lacks_becomes_none() {
    let handler = Memory::default();
    let record = assignment(existing("essay.pdf"), None);
    let claimed = assignment(existing("essay.pdf"), Some(existing("notes.pdf")));
    let body = body(&[wire_part(&claimed, &[])]);
    let model = accept(&body, &handler, Some(&record));
    expect_that!(model, ok(eq(&record)));
}

#[gtest]
fn a_claim_inside_a_list_item_counts_as_missing() {
    let handler = Memory::default();
    let record = Portfolio {
        drafts: vec![existing("first.pdf")],
    };
    let body = body(&[wire_part(&record, &[])]);
    let errors = accept(&body, &handler, Some(&record)).expect_err("rejected");
    expect_that!(
        field_errors(&errors),
        elements_are![(starts_with("drafts.#"), elements_are![eq(REQUIRED)])],
        "an item key is the form's own count, not an identity (V's trap)"
    );
}

#[gtest]
fn a_new_file_replaces_the_one_on_the_record() {
    let handler = Memory::default();
    let record = assignment(existing("old.pdf"), None);
    let body = body(&[
        wire_part(&assignment(chosen("new.pdf"), None), &[]),
        pdf("essay", "new essay"),
    ]);
    let model = accept(&body, &handler, Some(&record));
    expect_that!(
        model,
        ok(eq(&assignment(received("new.pdf", "new essay"), None)))
    );
}

// ── The real handler ─────────────────────────────────────────────────────

/// A directory of this test's own, empty, so a test can count what's left.
fn scratch_dir(test: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "formoxus-accept-multipart-{test}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

fn files_in(dir: &PathBuf) -> usize {
    std::fs::read_dir(dir).expect("readable").count()
}

#[gtest]
fn the_temporary_handler_stores_the_bytes_on_disk() {
    let dir = scratch_dir("stores");
    let handler = TemporaryFileUploadHandler::in_dir(&dir);
    let model = OnDisk {
        title: "Unit 1".to_string(),
        essay: FileUpload::Chosen {
            name: "essay.pdf".to_string(),
            content_type: None,
            size: 0,
        },
    };
    let body = body(&[wire_part(&model, &[]), pdf("essay", "my essay")]);
    let accepted =
        accept_with::<OnDisk, _>(FormSpec::default(), &body, &handler, None).expect("accepted");
    let FileUpload::Received { stored, .. } = &accepted.essay else {
        panic!("expected Received, got {:?}", accepted.essay);
    };
    expect_that!(std::fs::read_to_string(stored.path()), ok(eq("my essay")));
    drop(accepted);
    let _ = std::fs::remove_dir_all(&dir);
}

#[gtest]
fn the_temporary_handler_leaves_nothing_behind_when_rejected() {
    let dir = scratch_dir("rejected");
    let handler = TemporaryFileUploadHandler::in_dir(&dir);
    let model = OnDisk {
        title: String::new(),
        essay: FileUpload::Chosen {
            name: "essay.pdf".to_string(),
            content_type: None,
            size: 0,
        },
    };
    let body = body(&[wire_part(&model, &[]), pdf("essay", "my essay")]);
    let accepted = accept_with::<OnDisk, _>(FormSpec::default(), &body, &handler, None);
    expect_that!(accepted, err(anything()));
    expect_that!(files_in(&dir), eq(0));
    let _ = std::fs::remove_dir_all(&dir);
}
