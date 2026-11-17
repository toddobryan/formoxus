---
name: server-fn-futures-need-not-be-send
description: dioxus-server 0.7.10 runs server functions via LocalPoolHandle::spawn_pinned, so their futures need not be Send; FormState/Submission are !Send and that is fine
metadata:
  type: project
---

A Dioxus 0.7 server function's future does NOT have to be `Send`. dioxus-server
0.7.10 runs each one with `tokio_util::task::LocalPoolHandle::spawn_pinned`
(`dioxus-server-0.7.10/src/serverfn.rs`, line 76; the pool is built in
`server.rs`, line 229). Checked 2026-11-17 while expanding WIDGETS_PLAN 11h.

This matters because `FormState<T>` and `Submission<T>` are not `Send`: a node is
`Box<dyn Node>`, and `Node: Debug` only. `FormSpec<T>` IS `Send` (all its closures
are `Send + Sync`). Probed with a throwaway `is_send::<…>()` test. apcsp already
depends on this: `perform_password_change` (apcsp `crates/api/src/auth.rs`, from
line 181) holds a `Submission` across `matches_password(…).await`.

**Why:** an earlier assumption (WIDGETS_PLAN 11c) was that a server function's
future must be `Send`, which would have made `Submission::accept_multipart`, which
must hold a `FormState` across awaits, impossible to write.

**How to apply:** an async formoxus API used inside a server function may hold a
`FormState` across `.await` and need not promise `Send`. Things that are NOT run
through `spawn_pinned` still need `Send`: axum's `FromRequest::from_request` future
(the trait says so), and the `FileUploadHandler`/`FileUploadWriter` futures, kept
`Send` by choice so a handler also works from a plain axum route. Before making
`Node: Send`, check this again: it would be a big change for no current need. If
dioxus ever stops using `spawn_pinned`, `Submission` across an await breaks in apcsp
first. Related: [[multipart-upload-behaviour]].
