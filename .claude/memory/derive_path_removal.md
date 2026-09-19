---
name: derive-path-removal
description: "2026-09-19 — the #[derive(Form)]/#[derive(FieldSet)] path was DELETED and the reflect module FLATTENED into the crate root, so formoxus::Form not formoxus::reflect::Form. What the two paths actually shared (almost nothing), the one entanglement, the three non-obvious traps the deletion sprang, and the new test baseline of 317"
metadata:
  type: project
---

## What happened

Todd's call: remove the derive approach, promote the reflection approach to be
the only one. Both halves done in one pass, workspace green at **317 passed /
0 failed / 4 ignored**, zero warnings.

`formoxus::reflect::Form` is now `formoxus::Form`. The `reflect` name is gone
from the crate entirely — it only ever meant "not the derive path", so once it
was the only path the namespace was vestigial. Doing it at this exact moment
was nearly free: apcsp-dioxus is still pointed at its own stale mirror
(`crates/formoxus`), not at this repo, so no consumer import churn exists yet.
See [[next-up-two-todos]] — repointing apcsp is still open and is now also a
rename.

## The two paths shared almost nothing, which is why this was a day and not a week

Measured before cutting, and worth knowing if anything similar comes up:
`reflect/` imported exactly three things from outside itself — `error`
(`FieldError`/`FormError`/`FormAccessError`), `label_case`, and **one 11-line
`FieldErrors` component** from `widgets/base.rs`. That component was the single
entanglement in the whole crate; it moved into `widgets.rs` and that was it.
In the macro crate the separation was total: `form2.rs` and `using_fns.rs` had
**zero** references to the eight derive-path modules.

Flattening was correspondingly cheap because `reflect/`'s `form.rs`,
`fields.rs` and `widgets.rs` had the *same names* as the derive-path files
being deleted — so the move was delete-then-`git mv`, with no renames to
invent.

## Three traps it sprang, none of them visible from the file tree

1. **Dropping `darling` broke `form!`.** darling was pulling in `syn`'s
   `extra-traits` feature, which is what provides `Debug` on syn types —
   and `form2.rs` derives `Debug` across its parsed spec types, with 72 parser
   tests asserting on that output. Removing the derive path's only darling
   consumer silently removed the feature. Fix: declare
   `syn = { features = ["full", "extra-traits"] }` directly. **A transitive
   feature is an invisible dependency; deleting the crate that pulled it in
   is what exposes it.**
2. **The macros emit absolute paths, and paths are not code.** `form!` and
   `using_fns!` expanded to `::formoxus::reflect::…` in 12 places. The library
   compiled clean and only the *consumer* test target failed, because that is
   the only place macro output is actually resolved. One assertion also
   hard-coded the path with token spacing (`":: formoxus :: reflect :: …"`),
   which a `formoxus::reflect::` search does not match — grep for both
   spellings after any rename that macro output mentions.
3. **`tests/ui/` was bigger than it looked.** Four derive-path trybuild goldens
   (`missing_debug`, `reserved_field_name`, `tuple_struct`,
   `unknown_attribute`) survived the first sweep because an `ls` got truncated.
   `compile_fail.rs` globs `tests/ui/*.rs`, so they failed as a *golden
   mismatch* rather than as a missing-symbol error — which reads like a
   diagnostic regression, not like leftover files.

## Decisions taken along the way

- **`formoxus-macros` is no longer optional.** The `derive` feature (and
  `default = ["derive"]`) existed to make `#[derive(Form)]` skippable. `form!`
  is how a form is *declared*, so a build without it would be missing the
  crate's primary entry point, not trading convenience for compile time. The
  two `[[test]] required-features` blocks went with it.
- **`tests/reflect.rs` became `tests/consumer.rs`.** The target exists for
  tests that must be written the way a consumer writes them — anything touching
  `form!` has to live where the path `formoxus::` resolves, which rules out
  formoxus itself. The new name says *why* it exists rather than which path it
  covered.
- **`error::try_from` was deleted** — the derive path's `FromStr` scalar
  parser, dead once conversions go through facet's vtables.
- **13 dangling "same as the derive path" comments were rewritten, not
  deleted.** Each cited the derive path as precedent for a live choice; the
  reasoning was restated self-contained, because a justification pointing at
  deleted code is worse than no justification. `REFLECT_PLAN.md` and
  `BUTTONS_PLAN.md` got dated status banners instead of edits — they are
  records of decisions, and rewriting them would destroy what they are for.

## The number to compare against

**317** is the new green baseline. The drop from 359 is fully accounted for:
36 derive-path integration tests plus 6 macro unit tests in the deleted
modules. All 208 in-crate tests survived untouched, which is the real evidence
that nothing reflection-side was lost. (And 359 itself was never comparable to
[[next-up-two-todos]]'s 476, which measured the whole apcsp workspace before
extraction — see [[facet-050-compatibility]].)
