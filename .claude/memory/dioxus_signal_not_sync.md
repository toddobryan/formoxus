---
name: dioxus-signal-not-sync
description: A plain Dioxus `use_signal` is NOT Send/Sync (RefCell storage), so a Send + Sync hook in a FormSpec cannot capture one; use use_signal_sync. And `let _ = s;` in a closure captures nothing, so it proves nothing
metadata:
  type: project
---

Anything a `FormSpec` holds must be `Send + Sync`: the spec and `FormState`
reach the server, where `Submission` rebuilds a `FormState` and a handler's
future must be `Send` (see the `Rc<Classes>` note in `nodes.rs` and
[[config-cascade]]). So a dev's closure stored in the spec (first one: a
set's `feedback:`, SETS_PLAN.md I′, 2026-11-03) is `Send + Sync`.

A plain `use_signal` / `Signal<T>` (default `UnsyncStorage`) holds a
`RefCell` and is NOT `Sync`, so such a closure cannot capture it. A
`use_signal_sync` (`SyncStorage`) works, as do an `Arc`, a channel, or a
global signal. Writing through a captured signal in an `Fn` closure needs a
local copy first (`let mut s = s; s.write()…`), since `write` takes `&mut`.

**Why this is a memory:** I first told Todd a plain `Signal` WAS `Send +
Sync`, from a check whose closure body was `let _ = s;`. A `_` pattern does
not capture, so the closure held nothing and the check proved nothing. The
suite's real capture is what caught it.

**How to apply:** when designing a hook that lives in a spec, say up front
that it is `Send + Sync` and what that rules out. When checking whether a
closure can capture a type, USE the value in the body (`drop(s)` or a method
call), never `let _ = s;`.
