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
