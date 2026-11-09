---
name: pre-publish-checklist
description: "Things Todd asked to be reminded of before formoxus is published to crates.io. Chief among them: NOTHING runs wasm-opt today, so every consumer ships a larger bundle than they need to. Add to this file rather than starting a new one when another pre-publish item turns up"
metadata:
  type: project
---

**Raise these when publishing comes up.** Todd asked for the reminder on
2026-09-21 while choosing a regex engine; the list is cumulative, so append
rather than replace.

## 1. Nothing runs `wasm-opt` — measured, not assumed

`dx build --help` has no wasm-opt flag, there is no `wasm-opt` binary on the
machine, and no dioxus cache holding one. Dioxus 0.7 does manipulate the wasm
(`--wasm-split`, name-section control) but that looks like Rust-native tooling,
not a Binaryen shell-out.

Two consequences:

- **Every wasm size measured in this repo is a shipped size**, not a
  pre-optimization one. Do not discount them by the usual "10–20% off after
  wasm-opt".
- **There is an easy win going unclaimed.** `wasm-opt -Oz` typically takes
  10–30% off Rust wasm output. On the examples' 3.80 MiB / 990 KiB-gzipped
  baseline that is very likely worth more than the entire regex-engine choice
  (`regress` = +143 KiB gzipped, the largest single decision on the table).

What to do: a `just` recipe that runs it, a note in the README so consumers know
to, and a CI check if bundle size is ever something this repo asserts on.
Binaryen installs via apt/brew/npm, or the `wasm-opt` cargo crate which wraps a
bundled copy.

## 2. The doc pass

186 undocumented public items as of 2026-09-21 (was 179 before `path!`,
`WireForm` and choices landed). Then turn on `#![warn(missing_docs)]`. Deferred
deliberately while the public surface is still moving — see
[[widget-table-and-choice]], which first recorded it.

## 3. The facet version to publish against

Currently pinned to 0.46.5. [[facet-050-compatibility]] verified 0.50.0-rc.7 is
a drop-in — one version string, zero code changes. The open question is whether
to publish against a released 0.46.5 or wait for 0.50 to leave rc, since
publishing against an rc would force every consumer onto it.

## 4. Check the size story end to end

`regress` is in both `formoxus` and `formoxus-macros` as of 2026-09-21. Before
publishing, confirm the macro-side copy does NOT reach the wasm bundle — a
proc-macro dependency is host-only and should not, but it is worth verifying
rather than assuming, since it would silently double the cost.

Also worth re-checking: `facet-reflect` has `regex` as a DEFAULT feature for its
own `matches_pattern` validator. It is dead-stripped today (verified: adding a
`regex` call grew the bundle by the full 784 KiB raw). If formoxus ever routes
validation through facet's `ValidatorKind`, consumers would ship two regex
engines.

`unicase` joined `formoxus` on 2026-11-03 for the case-insensitive set
orders. It was already in the lock, but only through `mime_guess`, which is
server-only, so it is NEW to the wasm bundle: measure what its fold tables
cost (expected small) alongside the `regress` check.

## 5. Decide the public API: what is re-exported, and from where

Since 2026-11-09 nothing new gets re-exported ([[prelude-is-the-one-list]], "No new
re-exports"). Each item is `pub` at its defining module and nowhere else, apart from the
prelude and the `widgets::*` kit as they stood then. Before publishing, in one pass:
1. Decide what the prelude holds, for a dev writing forms, and what the `widgets::*` kit
   holds, for a dev writing a custom widget.
2. Decide whether the defining modules stay `pub` or go `pub(crate)` behind those two
   lists. Hiding them shrinks the semver surface but takes away paths devs may already use.
3. Then update the suite, the examples and apcsp to the result. The suite runs against
   the public surface, so it is what proves the lists are enough.
Do this before the doc pass (item 2): it decides which items are public and need docs.

## 6. Scan the built docs for dead links — no lint can see two kinds

Added 2026-11-09 (Todd: "checklist", not a `just docs` step, for now). rustdoc's
`-D warnings` catches broken and private intra-doc links, but two kinds pass it:

1. **A link to a `#[doc(hidden)]` item.** It resolves, so no lint fires, but the
   page is never generated. formoxus has 13 hidden items (mostly `path.rs`'s
   macro support) and no links to them as of 2026-11-09.
2. **A hand-written HTML link,** `[x](../formoxus/…/struct.X.html)`. Never
   checked. `formoxus-macros` cannot intra-doc link into `formoxus` (no
   dependency), and the one it had (`path!`'s `ModelPath`) was dead from the
   prelude page and would have been dead on docs.rs, which documents each crate
   separately. Fixed 2026-11-09 by deleting it: formoxus's re-export of `path`
   already says it with a working link.

What to do in the doc pass, after the last doc is written: build the public docs
(`cargo doc --no-deps -p formoxus -p formoxus-spec -p formoxus-macros`, public,
since that is what a reader gets) and check every local `href` in
`target/doc/{formoxus,formoxus_spec,formoxus_macros}/**/*.html` resolves to a file.
About ten lines of Python did it (regex `href="([^"#?]+\.html)`, skip `http`, resolve
against the page's directory). `cargo-deadlinks` is the packaged version. If it
keeps finding things, promote it to a `just docs` step.

Leftover from the fix, for the doc pass to settle: the prelude's `path!` page shows
only the macros crate's doc (its `pub use formoxus_macros::{form, path, using_fns}`
carries none), so its summary line now opens "Emits a witness borrow…". `form!`
avoids this by keeping a plain-text summary in the macros crate (`FormSpec`, no
link), which the root page then shows twice.
