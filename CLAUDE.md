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

Three homes:
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
