# formoxus

Reflection-based forms for Dioxus 0.7, built from a model's [`facet`](https://facet.rs)
shape at runtime rather than a hand-written form struct. Extracted 2026-09-19
from `apcsp-dioxus` (a course-site app that was formoxus's original, and
still only, real consumer) — full commit history preserved via
`git filter-repo`. `apcsp-dioxus`'s own `crates/formoxus{,-macros}` are now a
stale mirror; this repo is the source of truth.

## Workspace layout

```
formoxus/           — the library
formoxus-macros/    — proc macros: form! and using_fns!, nothing else
formoxus-attrs/     — the HTML attribute table both depend on; no dependencies (2026-10-03)
```

**One way to build a form, as of 2026-09-19.** A model derives `Facet` and
nothing else; `form!` declares the form over its shape and `use_form` makes it
live. The original `#[derive(Form)]`/`#[derive(FieldSet)]` path was deleted that
day and the `reflect` module it coexisted with was flattened into the crate
root — so `formoxus::Form`, not `formoxus::reflect::Form`. Anything still
saying "the derive path" or `formoxus::reflect::` is stale; see
`.claude/memory/derive_path_removal.md`.

The macros are function-like, never derives, and that is load-bearing rather
than stylistic: a derive can only expand in the crate that *defines* the type,
while these expand at the call site, which is what lets a form name a model from
one crate and a widget from another without tripping the orphan rule.

## Vocabulary: **widget**, never "control"

One word for anything that renders a field, at every size: `WidgetType`,
`WidgetProps`, `src/widgets/`, `widget:` in `form!`. Not "control" — nothing
here renders a bare element (even `Input` wraps a label, caption, required
marker and error list), and Django, whose Field/Widget split this design came
from, calls the whole range `Widget`. A brief widget→control rename on
2026-09-19 was reversed the next day; anything between those two commits reads
"control" and is stale. See `.claude/memory/widget_is_the_umbrella_word.md`,
which also records the Control-vs-Widget split that was considered and why it
does not work.

**One deliberate exception: the CSS class `fx-control`.** It names the bare
`<input>`/`<select>`/`<textarea>` — the one thing the rule's reason does not
cover — and "form control" is the HTML spec's own term. The CSS vocabulary is
web-facing, not Rust-API-facing; do not let the word back into the Rust API on
its strength. All emitted classes are `fx-`-prefixed; the full list and its
reasoning are in `.claude/memory/css_class_vocabulary.md`.

## Exports: the prelude is the one list

`formoxus::prelude` holds what a consumer writing forms uses, and the crate
root is only `pub use prelude::*;`. Everything else is reached through its
defining module (`formoxus::members::FormMember`, `formoxus::buttons::ButtonSpec`);
`formoxus::widgets::*` is the separate glob for writing a custom widget.
**Inside the crate, the macros and intra-doc links, always use the full module
path** — never a name through the prelude — so trimming the prelude cannot
break anything. There used to be a flat root list as well, and the two drifted
on the first export change. See `.claude/memory/prelude_is_the_one_list.md`.

## `.claude/memory/`

Start at its `MEMORY.md` index. Holds the accumulated design reasoning for
this library — why the reflection path is shaped the way it is, gotchas
found the hard way probing `facet`'s API, things tried and abandoned and
why. Check it at the start of substantial work; update it (and its index)
when something durable and non-obvious is learned.

Note that `.claude/memory/` (committed, authoritative) and the session
auto-memory under `~/.claude/projects/` have drifted; the committed copy is
ahead. Prefer it, and write new memories to both.

## Testing

Uses [`googletest`](https://docs.rs/googletest) throughout — `expect_that!`/
`assert_that!` with matchers, `#[gtest]` rather than `#[test]`. See the
memory files for the reasoning that shaped specific tests; there isn't yet a
CLAUDE.md-level testing-tiers section the way `apcsp-dioxus` has one, because
this repo has no DB/server boundary of its own to tier against.

Four homes:
- `formoxus/tests/suite.rs` + `formoxus/tests/suite/` — nearly all of them, as
  modules of ONE integration target. They run against the public surface a
  dependent crate gets, so an item that is unreachable or un-nameable from
  outside fails here instead of passing quietly. One target rather than one
  per file because the modules share `render_to_html`, the `Harness` and the
  models, and cargo turns every `tests/*.rs` into a separate binary — which is
  also why each `mod` needs a `#[path]`.
- **Inline `#[cfg(test)] mod tests`** — fine, and the right home for tests of a
  pure function that need no form, model or runtime (see `label_case.rs`). What
  the repo does NOT want is a *file that is only tests* sitting in `src/`.
- `formoxus/tests/ui/` — trybuild goldens pinning `form!`'s compile-time
  diagnostics, including spans. Regenerate with `TRYBUILD=overwrite`, then
  *read the diff*. A toolchain bump is what invalidates these.
- `e2e/` — browser tests through `playwright-rs`, run with **`just e2e`** (which
  builds, serves, waits and tears down) or **`just e2e-watch [filter]`** to watch
  one happen in a real window. NOT part of `just ci`, because CI has no browser;
  every test is `#[ignore]`d, so `cargo test --workspace` compiles them — keeping
  selectors from rotting silently — and runs none.

  They exist for what the other three structurally cannot reach. The suite
  renders to an HTML string, which proves *markup*; only a browser proves that a
  click lands, that a value survives `onsubmit`, or that a `pattern` actually
  blocks a submit — and that last one is the assumption behind formoxus emitting
  constraint attributes at all.

  They drive the small single-purpose forms at `/t/<slug>` in
  `examples/src/test_forms.rs`, **not** the gallery: the gallery is for someone
  evaluating formoxus, these are for testing, and one feature per form is what
  keeps selectors unambiguous. Nothing human keeps them from rotting, so a test
  form without a test is dead weight — add them in pairs. Before writing a
  selector, read the `get_by_label` vs `get_by_role` section of
  `.claude/memory/e2e_harness.md`; the two differ exactly where formoxus's
  `aria-hidden` required marker lives, and reaching for the wrong one makes
  working code look broken.
