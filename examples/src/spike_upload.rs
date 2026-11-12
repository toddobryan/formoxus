//! THROWAWAY: WIDGETS_PLAN box 10's spike. Not formoxus; raw Dioxus, to learn
//! what a multipart submit can carry before `file` is built on one.
//!
//! Two client routes to the same server-side reader:
//!
//! - **event**: `MultipartFormData::from(FormEvent)`. The browser's own
//!   `new FormData(form)`, so it carries every NAMED input, a hidden one
//!   holding the wire JSON included, in DOM order.
//! - **hand**: a `web_sys::FormData` built part by part, the file taken out of
//!   a `FileData` picked in `onchange` (`FileData::inner()` is the
//!   `web_sys::File`). Order is whatever the code appends.
//!
//! Each server fn reads the parts and reports, per part: its name, file name,
//! content type, bytes, chunk count and largest chunk; and the server's RSS,
//! to compare buffering a file whole against streaming it.

#![allow(dead_code, unreachable_pub, clippy::all, clippy::pedantic)]

use dioxus::fullstack::RequestError;
use dioxus::fullstack::{ClientRequest, ClientResponse, IntoRequest, MultipartFormData};
use dioxus::fullstack::{
    extract::{FromRequest, Request},
    response::Response,
};
use dioxus::prelude::*;
use std::future::Future;

const WIRE: &str = r#"{"title":"hello","n":3}"#;

// ---------------------------------------------------------------- server

#[cfg(feature = "server")]
fn rss_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

/// How a file part is consumed: counted chunk by chunk (and written to a temp
/// file), or buffered whole with `bytes()`.
#[cfg(feature = "server")]
#[derive(Clone, Copy)]
enum Mode {
    Stream,
    Buffer,
}

/// One part's line of the report. Generic over the field type would need the
/// `Multipart` borrow to outlive the loop, so the loop is written out twice
/// (once per extractor) through this macro instead.
#[cfg(feature = "server")]
macro_rules! read_parts {
    ($form:expr, $mode:expr) => {{
        let mode: Mode = $mode;
        let before = rss_kib();
        let mut out = vec![format!("rss_before_kib={before}")];
        let mut peak = before;
        loop {
            let field = match $form.next_field().await {
                Ok(Some(f)) => f,
                Ok(None) => break,
                Err(e) => {
                    out.push(format!("NEXT_FIELD_ERROR status={} {e}", e.status()));
                    break;
                }
            };
            let name = field.name().unwrap_or("?").to_string();
            let file = field.file_name().map(str::to_string);
            let ty = field.content_type().map(str::to_string);
            let mut field = field;
            let (bytes, chunks, max_chunk, err, text) = match mode {
                Mode::Stream => {
                    let mut bytes = 0usize;
                    let mut chunks = 0usize;
                    let mut max_chunk = 0usize;
                    let mut err = None;
                    let mut text = None;
                    let mut sink = file.as_ref().map(|_| {
                        std::fs::File::create(std::env::temp_dir().join("fx-spike-upload")).unwrap()
                    });
                    let mut first = Vec::new();
                    loop {
                        match field.chunk().await {
                            Ok(Some(c)) => {
                                bytes += c.len();
                                chunks += 1;
                                max_chunk = max_chunk.max(c.len());
                                if let Some(s) = sink.as_mut() {
                                    use std::io::Write;
                                    s.write_all(&c).unwrap();
                                } else {
                                    first.extend_from_slice(&c);
                                }
                                peak = peak.max(rss_kib());
                            }
                            Ok(None) => break,
                            Err(e) => {
                                err = Some(format!("status={} {e}", e.status()));
                                break;
                            }
                        }
                    }
                    if file.is_none() {
                        text = Some(String::from_utf8_lossy(&first).into_owned());
                    }
                    (bytes, chunks, max_chunk, err, text)
                }
                Mode::Buffer => match field.bytes().await {
                    Ok(b) => {
                        peak = peak.max(rss_kib());
                        let text = file
                            .is_none()
                            .then(|| String::from_utf8_lossy(&b).into_owned());
                        (b.len(), 1, b.len(), None, text)
                    }
                    Err(e) => (0, 0, 0, Some(format!("status={} {e}", e.status())), None),
                },
            };
            out.push(format!(
                "part name={name} file={file:?} type={ty:?} bytes={bytes} chunks={chunks} \
                 max_chunk={max_chunk} text={text:?} err={err:?}"
            ));
            if err.is_some() {
                break;
            }
        }
        out.push(format!(
            "rss_peak_kib={peak} delta_kib={}",
            peak.saturating_sub(before)
        ));
        out.join("\n")
    }};
}

/// Route (a), at axum's default body limit.
#[post("/api/spike/event")]
async fn spike_event(mut form: MultipartFormData) -> Result<String, ServerFnError> {
    Ok(read_parts!(form, Mode::Stream))
}

/// Route (b), at axum's default body limit, streaming.
#[post("/api/spike/hand")]
async fn spike_hand(mut form: HandBuilt) -> Result<String, ServerFnError> {
    Ok(read_parts!(form, Mode::Stream))
}

/// Route (b), the limit raised to 64 MiB on this endpoint alone, streaming.
#[post("/api/spike/hand_big")]
#[middleware(dioxus::fullstack::axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024))]
async fn spike_hand_big(mut form: HandBuilt) -> Result<String, ServerFnError> {
    Ok(read_parts!(form, Mode::Stream))
}

/// Route (b), the limit raised, each part buffered whole with `bytes()`.
#[post("/api/spike/hand_big_buffer")]
#[middleware(dioxus::fullstack::axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024))]
async fn spike_hand_big_buffer(mut form: HandBuilt) -> Result<String, ServerFnError> {
    Ok(read_parts!(form, Mode::Buffer))
}

/// Route (b), the limit raised, but the server STOPS reading the file after
/// 1 MiB and answers, as a `max_size:` check would. What does the client see?
#[post("/api/spike/hand_stop")]
#[middleware(dioxus::fullstack::axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024))]
async fn spike_hand_stop(mut form: HandBuilt) -> Result<String, ServerFnError> {
    let mut out = Vec::new();
    while let Ok(Some(mut field)) = form.next_field().await {
        let name = field.name().unwrap_or("?").to_string();
        let mut bytes = 0usize;
        let mut sizes = Vec::new();
        while let Ok(Some(c)) = field.chunk().await {
            bytes += c.len();
            sizes.push(c.len());
            if bytes > 1024 * 1024 {
                out.push(format!(
                    "part {name}: STOPPED at {bytes} bytes; chunks {sizes:?}"
                ));
                let mut drained = 0usize;
                while let Ok(Some(c)) = field.chunk().await {
                    drained += c.len();
                }
                while let Ok(Some(mut f)) = form.next_field().await {
                    while let Ok(Some(c)) = f.chunk().await {
                        drained += c.len();
                    }
                }
                out.push(format!("drained {drained} more bytes, then answered"));
                out.push("rss_peak_kib=0 (n/a)".into());
                return Ok(out.join("\n"));
            }
        }
        out.push(format!("part {name}: {bytes} bytes"));
    }
    Ok(out.join("\n"))
}

// ---------------------------------------------------------------- the hand-built payload

/// One part of a hand-built multipart body.
pub enum Part {
    Text(String, String),
    File(String, dioxus::html::FileData),
}

/// What formoxus would own: a payload type that is `IntoRequest` on the client
/// (parts appended in the order given) and `FromRequest` on the server (axum's
/// `Multipart`, streamed).
pub struct HandBuilt {
    parts: Vec<Part>,
    #[cfg(feature = "server")]
    server: Option<dioxus::fullstack::axum::extract::Multipart>,
}

impl HandBuilt {
    fn new(parts: Vec<Part>) -> Self {
        Self {
            parts,
            #[cfg(feature = "server")]
            server: None,
        }
    }

    #[cfg(feature = "server")]
    async fn next_field(
        &mut self,
    ) -> Result<
        Option<dioxus::fullstack::axum::extract::multipart::Field<'_>>,
        dioxus::fullstack::axum::extract::multipart::MultipartError,
    > {
        match &mut self.server {
            Some(m) => m.next_field().await,
            None => Ok(None),
        }
    }
}

impl IntoRequest for HandBuilt {
    fn into_request(
        self,
        req: ClientRequest,
    ) -> impl Future<Output = Result<ClientResponse, RequestError>> + 'static {
        async move {
            #[cfg(all(feature = "web", target_arch = "wasm32"))]
            {
                use wasm_bindgen::JsCast;
                let fd = web_sys::FormData::new()
                    .map_err(|e| RequestError::Builder(format!("{e:?}")))?;
                for part in self.parts {
                    match part {
                        Part::Text(k, v) => fd.append_with_str(&k, &v),
                        Part::File(k, f) => {
                            let file: &web_sys::File = f
                                .inner()
                                .downcast_ref()
                                .ok_or_else(|| RequestError::Builder("not a web File".into()))?;
                            fd.append_with_blob_and_filename(&k, file.unchecked_ref(), &file.name())
                        }
                    }
                    .map_err(|e| RequestError::Builder(format!("{e:?}")))?;
                }
                return req.send_js_value(fd.into()).await;
            }
            #[allow(unreachable_code)]
            {
                let _ = (self, req);
                Err(RequestError::Builder("spike: web only".into()))
            }
        }
    }
}

impl<S: Send + Sync + 'static> FromRequest<S> for HandBuilt {
    type Rejection = Response;

    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        #[cfg(feature = "server")]
        return async move {
            use dioxus::fullstack::axum::response::IntoResponse;
            let m = dioxus::fullstack::axum::extract::Multipart::from_request(req, state)
                .await
                .map_err(|e| e.into_response())?;
            Ok(HandBuilt {
                parts: Vec::new(),
                server: Some(m),
            })
        };
        #[cfg(not(feature = "server"))]
        async move {
            let _ = (req, state);
            unreachable!("server only")
        }
    }
}

// ---------------------------------------------------------------- the page

#[component]
pub fn SpikeUpload() -> Element {
    let mut report = use_signal(String::new);
    let mut picked = use_signal(|| None::<dioxus::html::FileData>);

    let show = move |r: Result<String, ServerFnError>| match r {
        Ok(s) => s,
        Err(e) => format!("CLIENT_ERROR {e:?}"),
    };

    let hand = move |wire_first: bool, endpoint: &'static str| async move {
        let Some(f) = picked() else {
            report.set("NO FILE PICKED".into());
            return;
        };
        let wire = Part::Text("__wire".into(), WIRE.into());
        let file = Part::File("upload".into(), f);
        let parts = if wire_first {
            vec![wire, file]
        } else {
            vec![file, wire]
        };
        let body = HandBuilt::new(parts);
        let r = match endpoint {
            "hand" => spike_hand(body).await,
            "hand_big" => spike_hand_big(body).await,
            "hand_big_buffer" => spike_hand_big_buffer(body).await,
            "hand_stop" => spike_hand_stop(body).await,
            _ => unreachable!(),
        };
        report.set(format!("[{endpoint} wire_first={wire_first}]\n{}", show(r)));
    };

    rsx! {
        main { class: "container",
            h1 { "SPIKE: multipart upload" }

            h2 { "(a) event, hidden wire input BEFORE the file" }
            form {
                id: "event-before",
                onsubmit: move |e: FormEvent| async move {
                    e.prevent_default();
                    let r = spike_event(e.into()).await;
                    report.set(format!("[event-before]\n{}", show(r)));
                },
                input { r#type: "hidden", name: "__wire", value: WIRE }
                input { r#type: "text", name: "title", value: "a visible text input" }
                input { r#type: "file", name: "upload", id: "file-event-before" }
                button { r#type: "submit", id: "send-event-before", "Send" }
            }

            h2 { "(a) event, hidden wire input AFTER the file" }
            form {
                id: "event-after",
                onsubmit: move |e: FormEvent| async move {
                    e.prevent_default();
                    let r = spike_event(e.into()).await;
                    report.set(format!("[event-after]\n{}", show(r)));
                },
                input { r#type: "file", name: "upload", id: "file-event-after" }
                input { r#type: "hidden", name: "__wire", value: WIRE }
                button { r#type: "submit", id: "send-event-after", "Send" }
            }

            h2 { "(b) hand-built from a FileData picked in onchange" }
            input {
                r#type: "file",
                id: "file-hand",
                onchange: move |e: FormEvent| picked.set(e.files().into_iter().next()),
            }
            button { id: "hand-wire-first", onclick: move |_| hand(true, "hand"), "wire first" }
            button { id: "hand-file-first", onclick: move |_| hand(false, "hand"), "file first" }
            button { id: "hand-big", onclick: move |_| hand(true, "hand_big"), "big, stream" }
            button {
                id: "hand-big-buffer",
                onclick: move |_| hand(true, "hand_big_buffer"),
                "big, buffer"
            }

            button { id: "hand-stop", onclick: move |_| hand(true, "hand_stop"), "big, server stops at 1 MiB" }
            pre { id: "report", "{report}" }
        }
    }
}
