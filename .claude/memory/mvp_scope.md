---
name: mvp-scope
description: "The agreed MVP scope and ORDER for formoxus. CURRENT as of 2026-11-04: ErrorsStore + live validation, then range/checkbox_multiple/select_multiple/file (file is for apcsp's assignment uploads), then docs. First settled 2026-09-29 on one principle: everything that moves the public surface lands before the doc pass. Dates/Decimal/Uuid are still deliberately OUT. Supersedes formoxus_roadmap.md, which predates the reflection rewrite"
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

## ROAD TO PUBLISH — Todd, 2026-11-04, supersedes everything below

What is left, in order:

1. **`ErrorsStore` + live validation**, together. `formoxus/ERRORS_PLAN.md`,
   not written yet.
2. **The missing widgets, all four:** `range`, `checkbox_multiple`,
   `select_multiple`, `file`. After 1 so they are written against the final
   `WidgetProps`; before 3 so the doc pass covers them.
3. **A1 (`form!` grammar docs), then A2 (the doc pass).**
4. **A4 (`wasm-opt`)**, assumed still last.

A3 (facet version): wait for 0.50 to go stable. #18, #7, #8 and #19 (the GOV.UK
preset) come after.

**Why `file` is in:** "I really need File so that students can upload
assignments" (Todd), which makes it an apcsp requirement, not parity. It needs
its own design: a file-carrying model type and multipart handling
([[widget-table-and-choice]]). The other two multi-pick widgets were blocked on
"a leaf holds ONE string" ([[choice-fields-design]]), a note older than sets:
`SetNode` may make them a new way to render a set, not checked yet.

This reverses "Deliberately OUT" below for those widgets, as the 2026-11-03
road to publish already had for `ErrorsStore`, live validation, #4 and item
counts. The list below is kept as history; dates, `Decimal` and `Uuid` are still
out.

## ORDER AS OF 2026-09-30 (history) — supersedes the numbering below

Step 1 is **DONE** (all its breaking changes landed, last commit `3f04386`).
Todd reordered the rest as "4, 3, 6, 5, 7, 8, 9", with #6 still next:

1. ~~Breaking API changes~~ — done
2. ~~Issue #6~~ — done 2026-10-01 (`required: true` on a bool)
3. **C6 — author-supplied widget attributes** (was item 4) ← **NEXT**
   - **RESHAPED 2026-10-02 by Todd: one attribute table with per-attribute
     handling rules**, which absorbs issue #4 and collapses `Constraints` +
     `FieldAttrs` + `ValueKind`'s constraint fields. Server checks stay. See
     [[attribute-rules-design]].
   - **QUEUED right after C6 lands (Todd, 2026-10-02): split `fields.rs`.** It
     was 1,387 lines (712 code + 675 inline tests). Move `ValueKind`, `Bound`,
     `Constraints`, `FieldAttrs`, `check` and `attrs` (lines ~56–308) and THEIR
     tests into `fields/value_kind.rs` (the `form.rs` + `form/` layout); the
     `FormField` side and its `validate` tests stay. Mechanical, so I do it,
     between commits so it doesn't tangle with a feature diff.
   - **PULLED FORWARD 2026-10-10 (Todd), inside C6, before its Step 4:**
     #9 as `<label for>` un-nesting (P1), `BuiltinWidget` (P2), and #10's
     class-config SYSTEM with only `fx()` + `bare()` presets (P3; framework
     presets are post-MVP). See `formoxus/ATTRIBUTES_PLAN.md` "Before Step 4".
4. **Issue #9 — `aria-describedby`** (was item 3) — now P1 of C6, above
5. ~~**C5 — `help_text`**~~ DONE 2026-11-03 (was item 6) — moved AHEAD of per-field validators, so
   help text arrives right after the describedby mechanism it will join
6. ~~**C4 — per-field validators**~~ DONE 2026-11-03 (was item 5)
7. **A3 — the facet version decision**
8. **A1 — document the `form!` grammar**
9. **A2 — the doc pass**

**A4 (`wasm-opt`) was not in Todd's reordered list.** Assumed still last; confirm
before treating it as dropped. The item descriptions below are unchanged and
still accurate — only their order moved.

## The list, in order (original numbering, kept for the item descriptions)

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
   - **DONE 2026-09-30 — `FieldError`/`FormError` → `ValidationError<T>` (producer) + `ValidationMessage` (stored)**, with the error walk `distribute_errors`, the `collect_*`/`distribute_*` renames and the prelude made the one export list ([[prelude-is-the-one-list]]). Originally: **`FieldError`/`FormError` → one `ValidationError<T>`** ([[error-model-design]] —
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
   **Depends on `ValidationError<T>` from step 1.** Its syntax goes in the same `form!`
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

## Deliberately OUT (as of 2026-09-29; see ROAD TO PUBLISH for what has moved in)

- **Dates, times, `Decimal`, `Uuid`** — the largest functional hole and the most
  likely thing to be asked for (`<input type=date>` renders, but the field must
  be modelled as a `String`). Out because it is purely ADDITIVE: it pays no
  breaking-change tax, so it is the cheapest thing to defer. **The one to
  revisit if the line moves.**
- **IN since 2026-11-04.** `select_multiple`, `checkbox_multiple`, `file` — `form!` now refuses them at
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
