---
name: multipart-upload-behaviour
description: Measured 2026-11-12 (WIDGETS_PLAN box 10 spike): axum's body limit fails MID-STREAM, a server answering before draining an upload gives the client nothing, buffering costs the file's size in RSS
metadata:
  type: project
---

Found by the box 10 spike (`file-spike` branch, `0564e46`, pushed, never merged), on
dioxus 0.7.10 / axum 0.8.9, through `dx serve`. Full numbers in
`formoxus/WIDGETS_PLAN.md` box 10's note.

1. **axum's 2 MB `Multipart` default does not refuse a request up front.** The
   handler runs and earlier parts read fine; `Field::chunk()` then returns
   `413 Payload Too Large` partway into the file. Whoever reads the parts must
   turn that into a form error. Raise it per server fn with
   `#[middleware(DefaultBodyLimit::max(…))]`, which works.
2. **A server that answers before reading the whole request usually gives the
   client NO answer** (`TypeError: Failed to fetch`, or a 500 from dx's proxy):
   the browser is still sending when the connection closes. Reading and
   discarding the rest first made the answer arrive every time. So a server-side
   size check must drain, and the client-side check (`FileData::size`) is the one
   that gives the user a fast message.
3. **Buffering costs the file's size; streaming doesn't.** 20 MB via
   `Field::bytes()`: +21 MB RSS. Via `chunk()` to a temp file: +0.06–0.5 MB.
   Chunks ≤ ~0.9 MB when logged (one unlogged run implied a ~10 MB chunk,
   probably dx's dev proxy).
4. **Client side: wasm never has to touch the bytes.** `FileData::inner()`
   downcasts to `web_sys::File`; appended to a `web_sys::FormData` and sent with
   `ClientRequest::send_js_value` (dioxus-fullstack `web` feature only), the
   browser streams it from disk. Parts arrive in append order (or DOM order, via
   `MultipartFormData::from(FormEvent)`, which also sends every named input).

**Why it matters:** decisions J/K/M of WIDGETS_PLAN part 3 rest on these.
**How to apply:** don't design a `max_size:` check that returns early, or an
`Upload` that buffers whole files server-side, without re-measuring.
