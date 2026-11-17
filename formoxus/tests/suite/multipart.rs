//! `Form::to_multipart`, the client half of a submit with files
//! (`WIDGETS_PLAN.md` 11g, part (e)).
//!
//! What these pin is what goes IN: the wire part, the sizes it states
//! (decision AA), and which files ride with it, in what order. Only files the
//! form still has are sent: one picked inside a since-removed list item, or
//! under a since-unchosen variant, stays in the table but must not go. The
//! bytes on the wire are 11i's to test, and the browser's send box 13's.
//!
//! Seen through the page, like `file_input.rs`: a Pack button calls
//! `to_multipart` the way a dev's submit handler would, and shows what came
//! back through the test-only `wire()` and `files()` (decision LL).

use std::path::PathBuf;

use dioxus::prelude::*;
use dioxus_html::SerializedFileData;
use facet::Facet;
use formoxus::file::FileUpload;
use formoxus::prelude::*;
use googletest::prelude::*;

use super::{Harness, new_since};

#[derive(Facet, Clone, Debug, PartialEq)]
struct Essay {
    essay: FileUpload,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Submission {
    essay: FileUpload,
    notes: Option<FileUpload>,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Portfolio {
    drafts: Vec<FileUpload>,
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

fn existing(name: &str) -> FileUpload {
    FileUpload::Existing {
        name: name.to_string(),
    }
}

fn pdf(name: &str, size: u64) -> SerializedFileData {
    SerializedFileData {
        path: PathBuf::from(name),
        size,
        last_modified: 0,
        content_type: Some("application/pdf".to_string()),
        contents: None,
    }
}

/// The form, then a Pack button that calls `to_multipart` and shows, in
/// three `<pre>`s: the file parts (`path=name`, in sending order), the
/// sizes the wire part states (`path=size`, or `absent` when it has no
/// `file_sizes` key at all), and the wire part's values.
fn probe<T>(form: &Form<T>) -> Element
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static> + 'static,
{
    let form = *form;
    let mut files = use_signal(String::new);
    let mut sizes = use_signal(String::new);
    let mut values = use_signal(String::new);
    let pack = "Pack";
    rsx! {
        {form.render(using_fns! {})}
        button {
            aria_label: "{pack}",
            onclick: move |_| {
                let multipart = form.to_multipart();
                files.set(
                    multipart
                        .files()
                        .map(|(path, file)| format!("{path}={}", file.name()))
                        .collect::<Vec<_>>()
                        .join(" "),
                );
                // Through serde, so this is what the wire part's JSON says,
                // including whether it has the key at all.
                let json = serde_json::to_value(multipart.wire()).expect("serializes");
                sizes.set(match json.get("file_sizes") {
                    None => "absent".to_string(),
                    Some(map) => map
                        .as_object()
                        .expect("an object")
                        .iter()
                        .map(|(path, size)| format!("{path}={size}"))
                        .collect::<Vec<_>>()
                        .join(" "),
                });
                values.set(
                    multipart
                        .wire()
                        .values()
                        .iter()
                        .map(|(path, raw)| format!("{path}={raw}"))
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            },
            "Pack"
        }
        pre { id: "files", "{files}" }
        pre { id: "sizes", "{sizes}" }
        pre { id: "values", "{values}" }
    }
}

fn between<'a>(html: &'a str, open: &str, close: &str) -> &'a str {
    let start = html
        .find(open)
        .unwrap_or_else(|| panic!("no {open} in {html}"))
        + open.len();
    &html[start..start + html[start..].find(close).expect("closed")]
}

/// Press Pack, then read the three `<pre>`s: (files, sizes, values).
fn pack(app: &mut Harness) -> (String, String, String) {
    app.click(app.labelled("click", "Pack"));
    let html = app.html();
    let pre = |id: &str| between(&html, &format!(r#"<pre id="{id}">"#), "</pre>").to_string();
    (pre("files"), pre("sizes"), pre("values"))
}

// ── The forms ───────────────────────────────────────────────────────────

/// Required, nothing in it.
#[component]
fn NewEssay() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Essay>::default()));
    probe(&form)
}

/// `old.pdf` kept, nothing picked.
#[component]
fn EditEssay() -> Element {
    let form = use_form(|| {
        form_for(
            &Essay {
                essay: existing("old.pdf"),
            },
            FormSpec::default(),
        )
    });
    probe(&form)
}

/// Two file fields, both empty, so a test can pick them in either order.
#[component]
fn NewSubmission() -> Element {
    let form = use_form(|| empty_form(FormSpec::<Submission>::default()));
    probe(&form)
}

/// Two drafts already kept, so the list has two file inputs from the start.
#[component]
fn EditPortfolio() -> Element {
    let form = use_form(|| {
        form_for(
            &Portfolio {
                drafts: vec![existing("a.pdf"), existing("b.pdf")],
            },
            FormSpec::default(),
        )
    });
    probe(&form)
}

/// `Link` chosen, so the file input appears only once `Upload` is.
#[component]
fn LinkedWork() -> Element {
    let form = use_form(|| {
        form_for(
            &Coursework {
                work: Work::Link {
                    url: "https://example.com".to_string(),
                },
            },
            FormSpec::default(),
        )
    });
    probe(&form)
}

// ── No file ─────────────────────────────────────────────────────────────

/// `skip_serializing_if`: a form with nothing picked sends the wire part
/// exactly as it did before files existed, with no `file_sizes` key.
#[gtest]
fn an_empty_form_sends_only_the_wire_part() {
    let mut app = Harness::mount(NewEssay);
    let (files, sizes, values) = pack(&mut app);
    expect_that!(files, eq(""));
    expect_that!(sizes, eq("absent"));
    expect_that!(values, eq("essay="));
}

/// A kept file has only its name, which the wire part carries. There is no
/// file to send.
#[gtest]
fn a_kept_file_is_not_sent() {
    let mut app = Harness::mount(EditEssay);
    let (files, sizes, values) = pack(&mut app);
    expect_that!(files, eq(""));
    expect_that!(sizes, eq("absent"));
    expect_that!(values, eq("essay=old.pdf"));
}

// ── A picked file ───────────────────────────────────────────────────────

#[gtest]
fn a_picked_file_is_sent_under_its_path_with_its_size() {
    let mut app = Harness::mount(NewEssay);
    app.pick_files(app.only_listener("change"), &[pdf("essay.pdf", 1_000)]);
    let (files, sizes, values) = pack(&mut app);
    expect_that!(files, eq("essay=essay.pdf"));
    expect_that!(sizes, eq("essay=1000"));
    expect_that!(values, eq("essay=essay.pdf"));
}

/// The FORM's order, not the order the user picked in, so the same form
/// sends the same request whatever was done first.
#[gtest]
fn files_go_in_the_forms_order_not_the_order_picked() {
    let mut app = Harness::mount(NewSubmission);
    let [essay, notes] = app.listeners("change")[..] else {
        panic!("expected two file inputs");
    };
    app.pick_files(notes, &[pdf("notes.pdf", 200)]);
    app.pick_files(essay, &[pdf("essay.pdf", 100)]);
    let (files, sizes, _) = pack(&mut app);
    expect_that!(files, eq("essay=essay.pdf notes=notes.pdf"));
    expect_that!(sizes, eq("essay=100 notes=200"));
}

#[gtest]
fn files_in_a_list_go_in_item_order() {
    let mut app = Harness::mount(EditPortfolio);
    let [first, second] = app.listeners("change")[..] else {
        panic!("expected two file inputs");
    };
    app.pick_files(second, &[pdf("b2.pdf", 200)]);
    app.pick_files(first, &[pdf("a2.pdf", 100)]);
    let (files, sizes, _) = pack(&mut app);
    expect_that!(files, eq("drafts.#0=a2.pdf drafts.#1=b2.pdf"));
    expect_that!(sizes, eq("drafts.#0=100 drafts.#1=200"));
}

// ── Stale files stay out ────────────────────────────────────────────────

/// The table still holds the removed item's file (nothing but a file's own
/// Remove and `reset` takes one out); `to_multipart` must not send it, nor
/// state its size.
#[gtest]
fn a_file_under_a_removed_list_item_is_not_sent() {
    let mut app = Harness::mount(EditPortfolio);
    let [first, second] = app.listeners("change")[..] else {
        panic!("expected two file inputs");
    };
    app.pick_files(first, &[pdf("a2.pdf", 100)]);
    app.pick_files(second, &[pdf("b2.pdf", 200)]);
    app.click(app.labelled("click", "Remove item 1"));
    let (files, sizes, values) = pack(&mut app);
    expect_that!(files, eq("drafts.#1=b2.pdf"));
    expect_that!(sizes, eq("drafts.#1=200"));
    expect_that!(values, not(contains_substring("a2.pdf")));
}

/// The control for the next test: under the chosen variant, the file goes.
#[gtest]
fn a_file_under_the_chosen_variant_is_sent() {
    let mut app = Harness::mount(LinkedWork);
    let select = app.only_listener("change");
    let before = app.listeners("change");
    app.fire("change", select, "Upload");
    let [input] = new_since(&before, app.listeners("change"))[..] else {
        panic!("expected one new file input");
    };
    app.pick_files(input, &[pdf("work.pdf", 300)]);
    let (files, sizes, _) = pack(&mut app);
    expect_that!(files, eq("work.$Upload.file=work.pdf"));
    expect_that!(sizes, eq("work.$Upload.file=300"));
}

#[gtest]
fn a_file_under_a_variant_since_unchosen_is_not_sent() {
    let mut app = Harness::mount(LinkedWork);
    let select = app.only_listener("change");
    let before = app.listeners("change");
    app.fire("change", select, "Upload");
    let [input] = new_since(&before, app.listeners("change"))[..] else {
        panic!("expected one new file input");
    };
    app.pick_files(input, &[pdf("work.pdf", 300)]);
    app.fire("change", select, "Link");
    let (files, sizes, values) = pack(&mut app);
    expect_that!(files, eq(""));
    expect_that!(sizes, eq("absent"));
    expect_that!(values, not(contains_substring("work.pdf")));
}
