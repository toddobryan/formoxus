# Constraint attributes: build steps

Written 2026-09-26 to be picked up in pieces. Each step ends with a check that
says it's done, and each is a safe place to stop. Commit after each one.

Background in `.claude/memory/constraint_attributes_gap.md`.

## The gap, in one line

`required` is the only constraint the browser is ever told about. `pattern`,
`min`, `max`, `min_length` and `max_length` are checked at compile time by
`field_kind` and validated in Rust by `ValueKind::check`, but never reach the
DOM — so there is no instant feedback and nothing blocks submit.

## What you already have, which is more than it looks

`FormField::render` hands `ScalarWidget` a `value_kind: ValueKind`, and
`ValueKind` **already carries every constraint**:

- `Text { min_length, max_length, pattern }`
- `Int { min, max }` / `Float { min, max }` — the author's bounds, merged from
  `Constraints` by `value_kind()` (`fields.rs:212`)

`ScalarWidget`'s `Input` arm then matches `ValueKind::Text { .. } | Int { .. } |
Float { .. }` and throws them away with the `..`. So nothing needs to be
threaded from further up: the data is already at the dispatch point. **This is
not a change to the widget boundary**, which stays `(path, label, required,
errors)` — see `FormField::validate`'s comment on why that is deliberate.

## Decisions already made

- **Render them unconditionally; do NOT gate on `browser_validation`.**
  `required` is already rendered unconditionally, and `novalidate` on the
  `<form>` disables constraint validation form-wide — so it neutralizes these
  for free. Threading the flag into widgets would duplicate what one attribute
  on the form already does.
- **Do NOT re-anchor `pattern`.** HTML implicitly wraps it as `^(?:…)$`, and
  `ValueKind::check` wraps it Rust-side *specifically so the two agree*
  (`fields.rs`, `a_pattern_must_match_the_entire_value`). The DOM attribute takes
  the author's raw pattern. Wrapping it twice would be a silent behaviour change
  on the server side of a pair that is meant to match exactly.
- **A `<textarea>` gets `minlength`/`maxlength` but NOT `pattern`.** HTML has no
  `pattern` attribute on a textarea. So `pattern` on a textarea field stays
  Rust-only and the two widgets legitimately diverge —
  `anchors_and_dot_follow_javascript_not_perl` was written for exactly that
  case's newline semantics.

## Decisions to make (recommendations, change them if you disagree)

1. **How the constraints reach `Input`.** *Recommended:* a new prop struct, one
   `Option` per attribute, built in `ScalarWidget` from `value_kind` and passed
   as a single prop. Name it something other than `Constraints` — that is taken
   by `fields::Constraints`, which is the *spec* side; this is the *render* side.
   `HtmlConstraints` or `ConstraintAttrs`.
   - Not on `FieldProps`: that would hand them to `Checkbox`, `Select` and
     `RadioGroup`, none of which can use one, and it would break the documented
     four-field boundary.
   - Not `value_kind` itself: that couples a widget to the parse-side enum, and
     a `custom(…)` widget would then need it too.
2. **`step` on a float.** `<input type="number">` has an implicit `step=1`, so a
   browser REJECTS `2.5` in a number input unless `step` says otherwise. Today
   that is invisible because nothing constrains the input; the moment you emit
   `min`/`max` you are inviting the browser to validate, and it will reject valid
   `f64` values. *Recommended:* emit `step="any"` for a `Float` kind. Decide it
   here rather than discovering it later — it is the one case where this feature
   can make things WORSE than not shipping it.
3. **A constraint the input type ignores.** A `String` with `max_length` rendered
   as `widget: color`, or an `i64` with `min` rendered as `widget: text`. HTML
   ignores an inapplicable attribute, so emitting it is harmless but inert.
   *Recommended:* emit unconditionally and do not gate on input type. Gating
   means encoding a table of which of the 14 input types honours which
   attribute, which is real HTML knowledge but a separate piece of work. Note it
   in a comment so the next reader knows it was a choice.

## Steps

**1. The prop struct.** In `formoxus/src/widgets/types.rs`, beside `FieldProps`,
add the struct from decision 1 with one field per attribute (`min_length`,
`max_length`, `pattern`, `min`, `max`, and `step` if you took decision 2).
Derive `Clone, Debug, PartialEq, Default` — `Default` is what lets a widget that
has no constraints pass `..Default::default()`.
- Give it a `From<&ValueKind>` (or an inherent `fn from_value_kind`) so the
  mapping lives in one place rather than in each dispatch arm.
- Done when: `cargo check -p formoxus` passes and a unit test in `types.rs`
  shows `Text { max_length: Some(10), .. }` producing the right struct and
  `Bool` producing an empty one.

**2. Render them in `Input`.** Add the prop to `Input`, destructure it, and put
the attributes on the `<input>`. Bind each as `Option` so Dioxus omits the
attribute when it is `None` — the same mechanism `aria_invalid` already uses, and
the reason `novalidate` is `Option`-shaped rather than `false`.
- The hidden-input early return at `input.rs:60` should NOT get them: a hidden
  input is not user-editable and browsers do not validate it.
- Done when: `cargo run -q -p formoxus-examples --bin widget_matrix` still
  prints 46/84 — this step must not change which pairs render.

**3. Pass them from dispatch.** In `formoxus/src/widgets/scalar.rs`, the `Input`
arm currently discards the constraints via `..`. Build the struct from
`value_kind` and pass it. The `Textarea` arm gets one too, with `pattern`
cleared per the decision above.
- Leave the `Checkbox`, `Select`, `RadioGroup` and `Custom` arms alone.
- Done when: a scratch SSR render of a `String` field with
  `max_length: 10, pattern: r"\d+"` shows both attributes on the `<input>`.

**4. Tests.** A new `formoxus/tests/suite/constraint_attrs.rs`, registered in
`suite.rs` with its `#[path]` (cargo makes one binary per `tests/*.rs`, which is
why every module needs one).
- one test per attribute reaching the DOM, from `form!` not a hand-built spec;
- `pattern` reaches the DOM **unwrapped** — assert `pattern="\d{5}"` and NOT
  `^(?:`. This is the one that catches a double-anchor;
- a `<textarea>` gets `maxlength` and no `pattern`;
- a `Float` gets `step="any"` and an `Int` does not;
- **the crossing that started this:** with `browser_validation: off` the
  attributes are STILL rendered and `novalidate` is on the form; with it `on`,
  attributes rendered and no `novalidate`. Four assertions, and they are the
  reason this feature makes `tests/suite/browser_validation.rs` meaningful;
- a hidden input gets none of them.
- Done when: `cargo test -p formoxus --test suite constraint_attrs` passes.

**5. Check it for real.** `just serve`. The address example already has
`zip => { pattern: r"\d{5}(-\d{4})?" }` — but it also sets
`browser_validation: off`, so **the browser will not enforce it there**.
Temporarily flip that to `on`, type a bad zip, and confirm the browser blocks
submit. Flip it back: the example turns validation off on purpose, so the
gallery can show formoxus's own error rendering.
- Done when: a bad zip is blocked with `on` and reported by formoxus's own
  errors with `off`.

**6. Tidy.**
- `.claude/memory/constraint_attributes_gap.md` — rewrite as BUILT, keeping the
  `novalidate`-means-no-gating finding and recording what decisions 2 and 3
  landed on. Update its `MEMORY.md` line.
- `.claude/memory/formoxus_feature_parity.md` lists this as a gap; it isn't one
  any more.
- Delete this file.
- Done when: `just ci` passes.
