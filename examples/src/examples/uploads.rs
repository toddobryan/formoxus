//! A form with files, sent to a server function and stored there: `FileUpload`
//! fields, `max_size:` and `accept:`, `to_multipart` on the client and
//! `Submission::accept_multipart` on the server.

use dioxus::prelude::*;
use facet::Facet;
#[cfg(feature = "server")]
use formoxus::file::TemporaryFileUploadHandler;
use formoxus::file::{FileUpload, Multipart};
use formoxus::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct HandInEssay {
    title: String,
    essay: FileUpload,
    figure: Option<FileUpload>,
}

/// The same spec on both sides: the browser checks each pick against it, and
/// the server checks each file again as it arrives.
fn spec() -> FormSpec<HandInEssay> {
    form! {
        HandInEssay {
            title: "Hand in an essay",
            browser_validation: off,
            essay => {
                max_size: 5_000_000,
                accept: ".pdf, .docx, .txt",
                help_text: "A PDF, Word or text file, at most 5 MB.",
            },
            figure? => {
                label: "A figure (optional)",
                max_size: 2_000_000,
                accept: "image/*",
            },
            buttons: {
                hand_in: { type: submit, text: "Hand in" },
            }
        }
    }
}

/// Stores each file through `TemporaryFileUploadHandler` and says what it
/// stored. A real app would `move_to` each one somewhere permanent before
/// the submission is dropped, which deletes the temporary files.
///
/// axum allows a 2 MB body by default, less than these two files can be
/// together, so the limit is raised for this function alone.
#[post("/api/gallery/hand-in")]
#[middleware(dioxus::fullstack::axum::extract::DefaultBodyLimit::max(8 * 1024 * 1024))]
async fn hand_in(
    form: Multipart<HandInEssay>,
) -> Result<(WireForm<HandInEssay>, String), ServerFnError> {
    let handler = TemporaryFileUploadHandler::new();
    Ok(
        match Submission::accept_multipart(spec(), form, &handler, None).await {
            Ok(sub) => {
                let model = sub.model();
                let stored = |upload: &FileUpload| match upload {
                    FileUpload::Received { name, size, .. } => format!("{name} ({size} bytes)"),
                    other => other.name().to_string(),
                };
                let figure = model
                    .figure
                    .as_ref()
                    .map_or(String::new(), |figure| format!(", and {}", stored(figure)));
                let said = format!(
                    "Stored \"{}\": {}{figure}.",
                    model.title,
                    stored(&model.essay)
                );
                (sub.into_wire(), said)
            }
            Err(errors) => (WireForm::from_errors(errors), String::new()),
        },
    )
}

#[component]
pub fn HandIn() -> Element {
    let mut said = use_signal(String::new);
    let form = use_form(|| empty_form(spec()));

    rsx! {
        {form.render(using_fns! {
            // `|_model|`: arity says this runs only once the browser's checks
            // pass, so a file too large or of the wrong type is never sent.
            hand_in: move |_model: HandInEssay| async move {
                match hand_in(form.to_multipart()).await {
                    Ok((reply, line)) => {
                        let _ = form.absorb(reply);
                        said.set(line);
                    }
                    Err(e) => said.set(format!("Couldn't reach the server: {e}")),
                }
            },
        })}

        if !said().is_empty() {
            article { "{said}" }
        }
    }
}
