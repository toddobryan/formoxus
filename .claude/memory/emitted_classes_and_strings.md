---
name: emitted-classes-and-strings
description: "Todd 2026-09-26: the class names and the user-facing English formoxus emits should both move into the Formoxus config. Nothing built — but the mechanism already exists, so this is work rather than design. The button classes are the urgent part: `primary`/`danger`/`secondary` will collide with any CSS framework"
metadata:
  type: project
---

Todd's call, 2026-09-26, raised while the constraint-attributes plan was being
written. **Nothing built.** It is not blocked on a design decision: the cascade
shipped ([[config-cascade]]), so both of these are a matter of adding fields to
`Formoxus` and reading them at the render point — resolved at the `Form`
boundary and handed down, per that entry's rule.

## Part 1: class names

Fourteen distinct names, emitted across `form.rs`, `form/state.rs`,
`members/{list_set,variant_set}.rs` and every widget module. Three different
naming conventions, which is itself the finding:

- `form`, `form-title`, `form-field`, `field-label`, `required`, `field-errors`,
  `field-error`, `form-errors`, `form-error`, `form-row`, `add-row`,
  `remove-row`, `radio-group` — unprefixed
- `formoxus-buttons`, `formoxus-button-problems` — prefixed
- **`primary`, `danger`, `secondary`, `outline`** — from
  `ButtonType::default_class()` (`buttons.rs:95`), and these are the urgent
  ones. They are generic single words that Bootstrap, Bulma, Pico and most
  frameworks already define, so a consumer's button styling silently fights the
  library's. They are also **absent from the vocabulary list** in
  `examples/assets/main.css`'s header comment, which claims to be complete — so
  they are undocumented AND collision-prone.

Worth deciding at the same time whether the defaults become uniformly prefixed.
That is a breaking change for the gallery stylesheet, which is the worked example
of theming, so the stylesheet and the defaults move together.

## Part 2: the English

Every user-facing string is baked in, and `ABSENT_DISPLAY`'s own doc comment
(`select.rs:173`) already called this out and predicted the fix: "The eventual
shape is configuration — most likely alongside whatever mechanism makes error
rendering overridable." That mechanism is now the `Formoxus` config.

- `"--none--"` — `select.rs:173`, the absent entry in an optional chooser
- `"Choose..."` — `select.rs:136` AND `structure.rs:52`, the unselectable
  placeholder, spelled twice
- `"This field is required."` — `fields.rs:400`
- the constraint messages — `fields.rs:128/133/140/159/164/184/189`:
  `"length must be at least {min}"`, `"length must be at most {max}"`,
  `"input should match the regular expression {patt}"`,
  `"number must be at least {min}"`, `"number must be at most {max}"`

**The one that is structurally different:** the constraint messages and
`"This field is required."` are produced by `ValueKind::check` and
`FormField::validate`, which run **server-side through `Submission` with no
Dioxus runtime**. So they cannot read `defaults()` at all — the same constraint
that forced `ButtonSpec::label` to take its case as a parameter, and the reason
the cascade resolves on `Form` and not `FormState`. Error text therefore needs a
different route from the class names and the two placeholders, which are
render-side only. Do not assume one mechanism covers all of it.

Also note `"input should match the regular expression {patt}"` leaks a raw regex
to a user, which is a message-quality problem independent of localization.
