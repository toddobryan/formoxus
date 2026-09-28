---
name: constraint-attributes-gap
description: "AGREED 2026-09-25 to build right after radio_group: only `required` reaches the DOM, so pattern/min/max/lengths are Rust-side only — and `novalidate` means they need no gating on browser_validation"
metadata:
  type: project
---

**BUILT 2026-09-27.** `ValueKind::attrs` maps each constraint to an
`Attribute`, `ScalarWidget` merges it with any incoming pass-through attributes
(caller wins) and every widget spreads the result, so `minlength`, `maxlength`,
`pattern`, `min` and `max` now reach the DOM and the browser enforces them.
16 tests: 7 unit beside `attrs` in `fields.rs`, 9 end-to-end from `form!` in
`tests/suite/constraint_attrs.rs`.

**Browser enforcement is now PROVEN, 2026-09-28**, by the browser tests in
`e2e/tests/constraints.rs` — a bad `pattern`, a short `min_length` and an
out-of-range `min` are each blocked before `onsubmit` runs, and the same values
reach formoxus's own error rendering once `browser_validation: off` sets
`novalidate`. That pair is also the proof that `novalidate` neutralizes these,
which is the assumption behind emitting them ungated. `pattern` anchoring is
checked end to end too: a value with five digits embedded in it is refused by the
browser, matching what `check` does in Rust. See [[e2e-harness]].

Originally: Todd's call, 2026-09-25, to build it as soon as `radio_group` was done.

**The gap.** `Input` emits `type`, `name`, `value`, `required`, `aria_invalid`
and `oninput`, and nothing else (`widgets/input.rs`). A grep of all of
`src/widgets/` for `pattern`, `minlength`, `maxlength`, `min`, `max` as rendered
attributes returns ZERO hits. So `required` is the only constraint the browser is
ever told about: `pattern`, `min`, `max`, `min_length` and `max_length` are
compile-checked (`field_kind`) and validated in Rust (`fields.rs`) but never
reach the DOM. No instant feedback, and nothing blocks submit. Django renders
these; leptos_form has no constraint support at all, so this is a Django parity
gap only.

**The finding that makes it cheap: no gating is needed.** `required` is rendered
*unconditionally* today, and `novalidate` on the `<form>` is what neutralizes it.
HTML's `novalidate` disables constraint validation form-wide, so it neutralizes
every other constraint attribute for free. Render them always; `browser_validation:
off` already makes them inert. Do NOT thread the flag down to the widgets.

That also answers the question that started this (are there tests crossing
`pattern` with `browser_validation` on and off?): **no, and there is nothing to
cross today** — `browser_validation` only controls `novalidate` on the `<form>`
(`tests/suite/browser_validation.rs`, whose `Contact` model carries no
constraints), and `pattern` never renders. Those tests become meaningful only
once this ships.

**Two things to get right when building it:**

- **Do not re-anchor `pattern`.** HTML implicitly wraps it as `^(?:…)$`, and
  `fields.rs` already wraps it Rust-side *so the two agree* — see
  `a_pattern_must_match_the_entire_value` and
  `an_alternation_is_grouped_before_it_is_anchored`. The DOM attribute takes the
  author's RAW pattern; the wrap is the server's job only.
- **Validity is deferred to issue #4** (<https://github.com/toddobryan/formoxus/issues/4>,
  opened 2026-09-27). formoxus emits every derived attribute onto whatever element
  the widget renders and relies on the browser ignoring what does not apply. This
  is knowingly invalid HTML: the W3C Nu validator (26.9.26) flags **50 of the 70**
  combinations of the 14 `InputType` variants and the five attributes. Measured,
  not guessed — the issue carries the table.

  Two corrections to earlier notes in this file, both from running the validator:
  `maxlength` on `<input type="color">` is NOT merely inapplicable, it is an
  error, so it is the same case as `pattern` on `<textarea>` and there was no
  useful distinction between them. And the date family DOES accept `min`/`max` —
  what breaks there is the VALUE, since formoxus's bounds are `i128`/`f64` and a
  date input wants a date literal. That last one is why applicability gating alone
  does not finish the job, and it probably waits on typed date support.

Worth a test pinning whatever the answer is, in both directions, since the whole
point of `regress` is that server and browser cannot disagree.

## `step` on a float: decided, and a doc item (2026-09-26)

**Do not emit `step`.** Todd's call. `<input type="number">` has an implicit
`step=1`, so a browser rejects `2.5` unless `step` says otherwise — but formoxus
never defaults to `type="number"`. `Int` and `Float` default to `Text`, because
`type="number"` eats a half-typed value (the reasoning is in `scalar.rs`'s
dispatch comment). `number` IS in the macro's vocabulary
(`widget.rs:71`, `number => Input(Number)`), so the hole exists, but only for an
author who explicitly writes `widget: number` on a float that also carries `min`
or `max`.

Instead: a comment where the `Float` arm builds its attributes, and **a rustdoc
note in the doc pass** — an author who wants `widget: number` on a float supplies
`step` themselves. Recorded here because the plan file gets deleted and the doc
pass has not happened yet ([[pre-publish-checklist]] tracks it, 200 items).
