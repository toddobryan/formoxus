//! `FileInput`, the one widget for a `FileUpload` field, driven through its
//! events (`WIDGETS_PLAN.md` 11f, step f.12).
//!
//! `files.rs` covers what the STATE does with a file table it is handed. These
//! cover what the widget and `Form` do to get one there: a pick through the
//! input's `change` reaches the value store (the name) and the file table (the
//! file), and from the table `validate` and the live check (f.11), `reset`
//! (f.10) and the markup (f.8, f.9). No browser is needed: a `change` carrying
//! dioxus-html's own `SerializedFileData`, which implements `NativeFileData`,
//! is what `evt.files()` reads on every platform.
//!
//! Everything here is seen through the page, never through the table, which a
//! test can't reach (decision CC): what `validate` returned, the errors shown,
//! the wire, the markup.

use std::path::PathBuf;

use dioxus::prelude::*;
use dioxus_html::SerializedFileData;
use facet::Facet;
use formoxus::file::{FileType, FileUpload};
use formoxus::prelude::*;
use googletest::prelude::*;

use super::{Harness, render_to_html};

const TOO_LARGE_2_MB: &str = "This file is too large. The largest allowed is 2 MB.";

#[derive(Facet, Clone, Debug, PartialEq)]
struct Essay {
    essay: FileUpload,
}

#[derive(Facet, Clone, Debug, PartialEq)]
struct Notes {
    notes: Option<FileUpload>,
}

fn existing(name: &str) -> FileUpload {
    FileUpload::Existing {
        name: name.to_string(),
    }
}

/// A file as the browser hands it over. The name is the path's last part.
fn file(name: &str, content_type: &str, size: u64) -> SerializedFileData {
    SerializedFileData {
        path: PathBuf::from(name),
        size,
        last_modified: 0,
        content_type: Some(content_type.to_string()),
        contents: None,
    }
}

fn pdf(name: &str, size: u64) -> SerializedFileData {
    file(name, "application/pdf", size)
}

/// What a `validate()` came back with, short enough to match in the page.
fn describe(upload: &FileUpload) -> String {
    match upload {
        FileUpload::Chosen { name, size, .. } => format!("Chosen {name} {size}"),
        FileUpload::Received { name, .. } => format!("Received {name}"),
        FileUpload::Existing { name } => format!("Existing {name}"),
    }
}

fn describe_essay(model: &Essay) -> String {
    describe(&model.essay)
}

fn describe_notes(model: &Notes) -> String {
    model
        .notes
        .as_ref()
        .map_or("no notes".to_string(), describe)
}

/// The form, then a Check button that runs `validate` and shows what it
/// returned in `#checked`, a Reset button, and the wire in `#wire`.
///
/// The buttons' labels are formatted so they cross as mutations, which is what
/// `Harness::labelled` reads.
fn probe<T>(form: &Form<T>, describe: fn(&T) -> String) -> Element
where
    T: Clone + std::fmt::Debug + PartialEq + Facet<'static> + 'static,
{
    // A copy, for the two button handlers to own.
    let form = *form;
    let mut checked = use_signal(String::new);
    let (check, reset) = ("Check", "Reset");
    let wire = form
        .to_wire()
        .values()
        .iter()
        .map(|(path, raw)| format!("{path}={raw}"))
        .collect::<Vec<_>>()
        .join(" ");
    rsx! {
        {form.render(using_fns! {})}
        button {
            aria_label: "{check}",
            onclick: move |_| {
                checked.set(match form.validate() {
                    Some(model) => format!("valid: {}", describe(&model)),
                    None => "invalid".to_string(),
                });
            },
            "Check"
        }
        button { aria_label: "{reset}", onclick: move |_| form.reset(), "Reset" }
        pre { id: "checked", "{checked}" }
        pre { id: "wire", "{wire}" }
    }
}

fn checked(html: &str) -> &str {
    between(html, r#"<pre id="checked">"#, "</pre>")
}

fn wire(html: &str) -> &str {
    between(html, r#"<pre id="wire">"#, "</pre>")
}

fn between<'a>(html: &'a str, open: &str, close: &str) -> &'a str {
    let start = html
        .find(open)
        .unwrap_or_else(|| panic!("no {open} in {html}"))
        + open.len();
    &html[start..start + html[start..].find(close).expect("closed")]
}

/// The `<input type="file">` tag, from `<input` to its `>`.
fn file_input_tag(html: &str) -> &str {
    let at = html
        .find(r#"type="file""#)
        .unwrap_or_else(|| panic!("no file input in {html}"));
    let start = html[..at].rfind("<input").expect("inside an <input>");
    let end = at + html[at..].find('>').expect("the tag closes");
    &html[start..=end]
}

/// The `<output>` tag beside the input, from `<output` to its `>`.
fn output_tag(html: &str) -> &str {
    let start = html
        .find("<output")
        .unwrap_or_else(|| panic!("no <output> in {html}"));
    let end = start + html[start..].find('>').expect("the tag closes");
    &html[start..=end]
}

/// What the `<output>` beside the input says.
fn output_text(html: &str) -> &str {
    let tag = output_tag(html);
    between(html, tag, "</output>")
}

fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let open = format!(r#" {name}=""#);
    let start = tag.find(&open)? + open.len();
    Some(&tag[start..start + tag[start..].find('"')?])
}

fn the_input(app: &Harness) -> dioxus::core::ElementId {
    app.only_listener("change")
}

fn check(app: &mut Harness) {
    app.click(app.labelled("click", "Check"));
}

fn reset(app: &mut Harness) {
    app.click(app.labelled("click", "Reset"));
}

// ── The forms ───────────────────────────────────────────────────────────

fn limited() -> FormSpec<Essay> {
    FormSpec::default().with_max_size(&path!(Essay.essay), 2_000_000)
}

/// Required, nothing in it, 2 MB at most, checked live.
#[component]
fn NewEssay() -> Element {
    let form = use_form(|| empty_form(limited().with_live_validation(true)));
    probe(&form, describe_essay)
}

/// As `NewEssay`, but checked only by `validate`.
#[component]
fn NewEssayNotLive() -> Element {
    let form = use_form(|| empty_form(limited().with_live_validation(false)));
    probe(&form, describe_essay)
}

/// An edit form: `old.pdf` is the file already kept, 2 MB at most.
#[component]
fn EditEssay() -> Element {
    let form = use_form(|| {
        form_for(
            &Essay {
                essay: existing("old.pdf"),
            },
            limited().with_live_validation(false),
        )
    });
    probe(&form, describe_essay)
}

/// Optional, nothing in it, 2 MB at most, checked live.
#[component]
fn NewNotes() -> Element {
    let form = use_form(|| {
        empty_form(
            FormSpec::<Notes>::default()
                .with_max_size(&path!(Notes.notes?), 2_000_000)
                .with_live_validation(true),
        )
    });
    probe(&form, describe_notes)
}

/// Optional, with `notes.txt` already kept.
#[component]
fn EditNotes() -> Element {
    let form = use_form(|| {
        form_for(
            &Notes {
                notes: Some(existing("notes.txt")),
            },
            FormSpec::default(),
        )
    });
    probe(&form, describe_notes)
}

// ── The markup (f.8, f.9) ───────────────────────────────────────────────

#[gtest]
fn a_file_field_renders_a_file_input_named_by_its_path() {
    let html = render_to_html(NewEssay);
    expect_that!(attr(file_input_tag(&html), "name"), some(eq("essay")));
}

/// A script can only set a file input's value to `""`, and a file kept from
/// before has no file in the browser to show, so the input never gets one.
#[gtest]
fn the_file_input_never_has_a_value() {
    let html = render_to_html(EditEssay);
    expect_that!(attr(file_input_tag(&html), "value"), none());
}

#[gtest]
fn the_output_is_for_the_file_input() {
    let html = render_to_html(NewEssay);
    let id = attr(file_input_tag(&html), "id").expect("the input has an id");
    expect_that!(attr(output_tag(&html), "for"), some(eq(id)));
}

/// Decision EE: nothing in the table is the kept file.
#[gtest]
fn a_kept_file_reads_current_file() {
    let html = render_to_html(EditEssay);
    expect_that!(output_text(&html), eq("Current file: old.pdf"));
}

#[gtest]
fn an_empty_file_field_shows_no_file() {
    let html = render_to_html(NewEssay);
    expect_that!(html, not(contains_substring("Current file")));
}

/// `.pdf` only, for decision DD.
#[component]
fn PdfEssay() -> Element {
    let form = use_form(|| {
        empty_form(FormSpec::<Essay>::default().with_accept(
            &path!(Essay.essay),
            FileType::parse_list(".pdf").expect("parses"),
        ))
    });
    form.render(using_fns! {})
}

/// Decision DD: `accept` waits for box 12's row in the attribute table.
/// `validate` checks the type either way (`files.rs`).
#[gtest]
fn no_accept_attribute_yet() {
    let html = render_to_html(PdfEssay);
    expect_that!(attr(file_input_tag(&html), "accept"), none());
}

#[gtest]
fn a_required_file_has_no_remove_button() {
    let html = render_to_html(EditEssay);
    expect_that!(html, not(contains_substring("fx-button-remove-file")));
}

#[gtest]
fn an_optional_file_holding_one_has_a_remove_button() {
    let html = render_to_html(EditNotes);
    expect_that!(html, contains_substring("fx-button-remove-file"));
}

/// Nothing to remove, so no button that does nothing.
#[gtest]
fn an_empty_optional_file_has_no_remove_button() {
    let html = render_to_html(NewNotes);
    expect_that!(html, not(contains_substring("fx-button-remove-file")));
}

/// `form!` refuses any other widget on a file at compile time; a hand-built
/// `FormSpec` can still give one, and the walk refuses it (f.8).
#[gtest]
#[should_panic(
    expected = "essay: a `FileUpload` field needs a widget that takes files, such as `file`, not `text`"
)]
fn a_hand_built_spec_with_a_widget_that_takes_no_files_panics() {
    let spec = FormSpec::<Essay>::default()
        .with_widget(&path!(Essay.essay), formoxus::widgets::input::TextInput);
    let _ = empty_form(spec);
}

#[gtest]
fn a_hand_built_spec_may_name_file_input() {
    let spec = FormSpec::<Essay>::default().with_widget(
        &path!(Essay.essay),
        formoxus::widgets::file_input::FileInput,
    );
    let _ = empty_form(spec);
}

// ── A pick (f.7) ────────────────────────────────────────────────────────

#[gtest]
fn a_pick_shows_the_files_name() {
    let mut app = Harness::mount(NewEssay);
    app.pick_files(the_input(&app), &[pdf("essay.pdf", 1_000)]);
    let html = app.html();
    expect_that!(output_text(&html), eq("essay.pdf"));
}

/// From `files()`, not the event's `value`, which a browser gives as
/// `C:\fakepath\essay.pdf`.
#[gtest]
fn a_pick_writes_the_files_name_to_the_store() {
    let mut app = Harness::mount(NewEssay);
    app.pick_files(the_input(&app), &[pdf("essay.pdf", 1_000)]);
    expect_that!(wire(&app.html()), eq("essay=essay.pdf"));
}

#[gtest]
fn a_pick_replaces_the_kept_file() {
    let mut app = Harness::mount(EditEssay);
    app.pick_files(the_input(&app), &[pdf("new.pdf", 1_000)]);
    let html = app.html();
    expect_that!(output_text(&html), eq("new.pdf"));
    expect_that!(wire(&html), eq("essay=new.pdf"));
}

/// A pick is finished the moment it happens, like a chooser's change: no
/// waiting for a blur, which on a file input means nothing.
#[gtest]
fn a_pick_is_checked_at_once() {
    let mut app = Harness::mount(NewEssay);
    app.pick_files(the_input(&app), &[pdf("essay.pdf", 3_000_000)]);
    expect_that!(app.html(), contains_substring(TOO_LARGE_2_MB));
}

#[gtest]
fn a_pick_within_the_limit_shows_no_error() {
    let mut app = Harness::mount(NewEssay);
    app.pick_files(the_input(&app), &[pdf("essay.pdf", 2_000_000)]);
    let html = app.html();
    expect_that!(html, not(contains_substring(TOO_LARGE_2_MB)));
    expect_that!(html, not(contains_substring(r#"aria-invalid="true""#)));
}

/// A later pick that fixes it clears the error, at once again.
#[gtest]
fn a_smaller_pick_clears_the_error() {
    let mut app = Harness::mount(NewEssay);
    app.pick_files(the_input(&app), &[pdf("big.pdf", 3_000_000)]);
    app.pick_files(the_input(&app), &[pdf("small.pdf", 1_000)]);
    expect_that!(app.html(), not(contains_substring(TOO_LARGE_2_MB)));
}

/// Only `reset` makes the input again (f.10). A pick that did would also
/// lose the browser's own display of the file it just took.
#[gtest]
fn a_pick_keeps_the_same_input() {
    let mut app = Harness::mount(NewEssay);
    let before = app.times_registered("change");
    app.pick_files(the_input(&app), &[pdf("essay.pdf", 1_000)]);
    expect_that!(app.times_registered("change"), eq(before));
}

/// A browser may send a `change` with no file when the user cancels the
/// dialog. That is not a choice to drop the file: Remove is.
#[gtest]
fn a_change_with_no_file_keeps_the_file() {
    let mut app = Harness::mount(EditEssay);
    app.pick_files(the_input(&app), &[]);
    let html = app.html();
    expect_that!(output_text(&html), eq("Current file: old.pdf"));
    expect_that!(wire(&html), eq("essay=old.pdf"));
}

/// The FIRST file. A file input without `multiple` sends one; this pins what
/// happens if a hand-made event sends more.
#[gtest]
fn a_change_with_two_files_takes_the_first() {
    let mut app = Harness::mount(NewEssay);
    app.pick_files(
        the_input(&app),
        &[pdf("first.pdf", 1_000), pdf("second.pdf", 1_000)],
    );
    expect_that!(wire(&app.html()), eq("essay=first.pdf"));
}

// ── validate (f.11) ─────────────────────────────────────────────────────

#[gtest]
fn validate_sees_the_picked_file() {
    let mut app = Harness::mount(NewEssayNotLive);
    app.pick_files(the_input(&app), &[pdf("essay.pdf", 1_000)]);
    check(&mut app);
    expect_that!(checked(&app.html()), eq("valid: Chosen essay.pdf 1000"));
}

/// The live check is off, so this error can only have come from `validate`
/// reading the table.
#[gtest]
fn validate_refuses_an_oversized_pick() {
    let mut app = Harness::mount(NewEssayNotLive);
    app.pick_files(the_input(&app), &[pdf("essay.pdf", 3_000_000)]);
    expect_that!(app.html(), not(contains_substring(TOO_LARGE_2_MB)));
    check(&mut app);
    let html = app.html();
    expect_that!(checked(&html), eq("invalid"));
    expect_that!(html, contains_substring(TOO_LARGE_2_MB));
}

#[gtest]
fn with_nothing_picked_a_kept_file_stays_existing() {
    let mut app = Harness::mount(EditEssay);
    check(&mut app);
    expect_that!(checked(&app.html()), eq("valid: Existing old.pdf"));
}

// ── Remove (f.9) ────────────────────────────────────────────────────────

/// Labelled like a list item's Remove, by what it removes.
#[gtest]
fn remove_empties_an_optional_file() {
    let mut app = Harness::mount(EditNotes);
    app.click(app.labelled("click", "Remove “notes.txt”"));
    check(&mut app);
    let html = app.html();
    expect_that!(checked(&html), eq("valid: no notes"));
    expect_that!(html, not(contains_substring("notes.txt")));
    expect_that!(html, not(contains_substring("fx-button-remove-file")));
}

/// Remove takes the file out of the table too, so the oversized file it
/// removed is not checked.
#[gtest]
fn remove_after_a_pick_drops_the_file() {
    let mut app = Harness::mount(NewNotes);
    app.pick_files(the_input(&app), &[pdf("big.pdf", 3_000_000)]);
    expect_that!(app.html(), contains_substring(TOO_LARGE_2_MB));
    app.click(app.labelled("click", "Remove “big.pdf”"));
    expect_that!(app.html(), not(contains_substring(TOO_LARGE_2_MB)));
    check(&mut app);
    expect_that!(checked(&app.html()), eq("valid: no notes"));
}

// ── reset (f.10) ────────────────────────────────────────────────────────

/// If the table kept the oversized pick, the kept `old.pdf` would be checked
/// as that file and refused.
#[gtest]
fn reset_empties_the_table() {
    let mut app = Harness::mount(EditEssay);
    app.pick_files(the_input(&app), &[pdf("new.pdf", 3_000_000)]);
    reset(&mut app);
    check(&mut app);
    expect_that!(checked(&app.html()), eq("valid: Existing old.pdf"));
}

#[gtest]
fn reset_shows_the_kept_file_again() {
    let mut app = Harness::mount(EditEssay);
    app.pick_files(the_input(&app), &[pdf("new.pdf", 1_000)]);
    reset(&mut app);
    expect_that!(output_text(&app.html()), eq("Current file: old.pdf"));
}

/// The browser keeps its own selection, so only a new `<input>` stops it
/// showing the file `reset` threw away. Counted by registrations, not by
/// element id: dioxus recycles ids, so the new input can have the old one's.
///
/// **A `key` on a lone element is ignored** (dioxus-core 0.7.10 compares keys
/// only among an iterator's children), so this fails if the key isn't on a
/// node from an iterator.
#[gtest]
fn reset_makes_the_input_again() {
    let mut app = Harness::mount(EditEssay);
    app.pick_files(the_input(&app), &[pdf("new.pdf", 1_000)]);
    let before = app.times_registered("change");
    reset(&mut app);
    expect_that!(app.times_registered("change"), eq(before + 1));
    let _ = the_input(&app);
}
