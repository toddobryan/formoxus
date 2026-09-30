---
name: mvp-scope
description: "The agreed MVP scope and ORDER for formoxus, settled with Todd 2026-09-29. Ten items, sequenced by one principle: everything that moves the public surface lands before the doc pass. Dates/Decimal/Uuid are deliberately OUT. Supersedes formoxus_roadmap.md, which predates the reflection rewrite"
metadata:
  type: project
---

**Settled with Todd 2026-09-29.** Supersedes [[formoxus-roadmap]] (stale — it
describes the deleted derive path). The survey this was drawn from is
[[formoxus-feature-parity]].

## The two principles that set the order

1. **A breaking change is free now and costs a major version after the first
   publish.** Several long-deferred items are breaking, which is the strongest
   argument for pulling them into MVP rather than leaving them in the queue.
2. **Everything that moves the public surface lands before the doc pass.**
   Otherwise the 205-item pass documents names that then change. This is why
   documenting the `form!` grammar comes *after* the three items that add
   grammar to it (C6, C4, C5), not before.

## The list, in order

1. **Breaking API changes.** Three of them.
   - **`SelectChoice` → `Choice` — DONE 2026-09-29.** Renamed, MOVED from
     `widgets/select.rs` to `widgets/types.rs` (it was never select's type), and
     documented with the value-choice/shape-choice contrast that the more general
     name made necessary — `Choice` sits in the same crate as `VariantChoice`, so
     the rename made that ambiguity worse and the doc comment now does the work
     the old name did. 482 tests green. See [[chooser-ideas]].
   - **Namespace every emitted CSS class name.** Scope GREW 2026-09-29 from "the
     button classes" to all 15: `ButtonType::default_class()`'s bare
     `primary`/`outline danger`/`outline secondary`/`danger`/`secondary` are
     DELETED outright, not made configurable — they are styled by nothing in
     `examples/assets/main.css`, absent from that file's "complete vocabulary"
     comment, correct only for Pico, and `formoxus-button-<type>` carries the
     semantic intent instead. Sweep in `add-row`/`remove-row` (buttons too,
     un-namespaced, OUTSIDE `.formoxus-buttons` so the descendant selector misses
     them) and rename `formoxus-button-problems`, which reads as "a button of
     type *problems*" once `formoxus-button-<type>` is a pattern. `.required` is
     the worst collision risk of the 15.
     **The CONFIG half is deferred to issue #10**, because `Formoxus` is
     `#[non_exhaustive]` and its own doc says that makes a new setting additive —
     so renaming defaults must happen in this window and overriding them need
     not. Also recorded there: class renaming does NOT buy framework parity
     (Bootstrap wants `form-control` on the `<input>`; formoxus puts `form-field`
     on the wrapping `<label>`), so the override is a convenience, not a fix.
   - **`FieldError`/`FormError` → one `Verdict<T>`** ([[error-model-design]] —
     already decided, unbuilt). The one with reach: it is public, it is
     `Serialize`/`Deserialize` so it crosses the wire, and `FormErrors`,
     `WireForm`, `push_field_error` and `Submission::reject_field` all touch it.
     **C4 depends on it.**
2. **Issue #6** — a checkbox cannot be required to be TICKED. A payload on
   `ValueKind::Bool` plus `check`/`attrs`/a compile gate, NOT forwarding
   `required` (the presence sense is load-bearing elsewhere). "I agree to the
   terms" currently submits unticked and formoxus accepts it.
3. **Issue #9** — `aria-describedby`. Message text currently folds into the
   input's accessible name on every labeled field. Touches `FieldProps`.
4. **C6 — author-supplied widget attributes** (`class`, `placeholder`, `rows`,
   `data-*`). The `attrs: Vec<Attribute>` plumbing already exists from
   [[constraint-attributes-gap]], so this is mostly `form!` grammar.
   **Todd moved this ahead of per-field validators**, and it is the right call
   twice over: it is nearly done, and it is the escape hatch that makes every
   remaining presentation gap survivable by the consumer instead of by us.
5. **C4 — per-field validators.** Design fully settled in
   [[error-model-design]] (`fn` pointer not `Box<dyn Fn>`, downcast made
   unreachable by a witness line, keyword `validator` at both levels).
   **Depends on `Verdict<T>` from step 1.** Its syntax goes in the same `form!`
   brace block C6 settles — which is why C6 goes first.
6. **C5 — `help_text`.** Tiny, and very commonly wanted.
7. **A3 — the facet version decision.** Publish against 0.46.5, or wait for 0.50
   to leave rc. Todd's call; [[facet-050-compatibility]] verified 0.50 is a
   drop-in, so this is a release-policy question, not a technical one.
8. **A1 — document the `form!` grammar.** The TODO at
   `formoxus-macros/src/lib.rs:36`. The one thing a user MUST read, and it can
   only be written once 4–6 stop changing the grammar.
9. **A2 — the doc pass.** 205 undocumented public items in `formoxus` as of
   2026-09-29 (186 on 2026-09-21, 179 before that — it grows as the surface
   does, which is the other reason to do it last). End by turning on
   `#![warn(missing_docs)]`. Count it with
   `cargo rustc -p formoxus --lib -- -W missing_docs 2>&1 | grep -c "missing documentation"`.
10. **A4 — `wasm-opt`.** A `just` recipe and a README note. See
    [[pre-publish-checklist]], which has the rest of the release mechanics.

## Deliberately OUT

- **Dates, times, `Decimal`, `Uuid`** — the largest functional hole and the most
  likely thing to be asked for (`<input type=date>` renders, but the field must
  be modelled as a `String`). Out because it is purely ADDITIVE: it pays no
  breaking-change tax, so it is the cheapest thing to defer. **The one to
  revisit if the line moves.**
- `select_multiple`, `checkbox_multiple`, `file` — `form!` now refuses them at
  compile time, which is honest rather than silent, so shippable
  ([[widget-table-and-choice]]). `file` is much larger than the other two.
- Dynamic choices / `Provider` ([[choice-fields-design]] — the `Box<dyn Fn>`
  carrier is still open).
- Issue #4 (valid attribute/element pairs), #7 (aria-invalid third state),
  #8 (warnings), #10 (configurable class NAMES — the renaming is in step 1).
- All of tier D: `ErrorsByPath`, live/blur validation, change tracking, draft
  persistence, formset bounds and stable row ids, [[widget-registry-idea]],
  `Chooser`, [[enum-as-a-value-choice]], the error-string half of
  [[emitted-classes-and-strings]], [[shrink-fieldprops-idea]].
