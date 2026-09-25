---
name: constraint-attributes-gap
description: "AGREED 2026-09-25 to build right after radio_group: only `required` reaches the DOM, so pattern/min/max/lengths are Rust-side only — and `novalidate` means they need no gating on browser_validation"
metadata:
  type: project
---

**Todd's call, 2026-09-25: build this as soon as `radio_group` is done.**

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
- **`<textarea>` has no `pattern` attribute** in HTML, though formoxus validates
  one on a textarea field, and `fields.rs`'s
  `anchors_and_dot_follow_javascript_not_perl` was written specifically for the
  newline semantics that case needs. So a `pattern` on a `Textarea` stays
  Rust-only and the two widgets diverge. `minlength`/`maxlength` DO exist on a
  textarea.

Worth a test pinning whatever the answer is, in both directions, since the whole
point of `regress` is that server and browser cannot disagree.
