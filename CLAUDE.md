# formoxus

Reflection-based forms for Dioxus 0.7, built from a model's [`facet`](https://facet.rs)
shape at runtime rather than a hand-written form struct. Extracted 2026-09-19
from `apcsp-dioxus` (a course-site app that was formoxus's original, and
still only, real consumer) — full commit history preserved via
`git filter-repo`. `apcsp-dioxus`'s own `crates/formoxus{,-macros}` are now a
stale mirror; this repo is the source of truth.

## Workspace layout

```
formoxus/          — the library: reflect/ (the live path) + derive-path types being retired
formoxus-macros/    — proc macros: form2!, using_fns!, the retiring #[derive(Form)]/#[derive(FieldSet)]
```

Two coexisting form-building approaches, not yet fully merged:
- **`reflect` module** — the active path. Build a form at runtime from a model's
  `#[derive(Facet)]` shape via `form2!` + `use_form`. No hand-written form struct.
- **Everything outside `reflect`** — the original derive-path (`#[derive(Form)]`),
  being retired. Nothing new should be added to it.

## `.claude/memory/`

Start at its `MEMORY.md` index. Holds the accumulated design reasoning for
this library — why the reflection path is shaped the way it is, gotchas
found the hard way probing `facet`'s API, things tried and abandoned and
why. Check it at the start of substantial work; update it (and its index)
when something durable and non-obvious is learned.

## Testing

Uses [`googletest`](https://docs.rs/googletest) throughout — `expect_that!`/
`assert_that!` with matchers, `#[gtest]` rather than `#[test]`. See the
memory files for the reasoning that shaped specific tests; there isn't yet a
CLAUDE.md-level testing-tiers section the way `apcsp-dioxus` has one, because
this repo has no DB/server boundary of its own to tier against.
