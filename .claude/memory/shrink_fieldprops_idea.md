---
name: shrink-fieldprops-idea
description: "Todd's TODO 2026-09-26: now that widgets take an attributes map, consider moving path/required/choices into it and shrinking FieldProps to label+errors. Recorded with the obstacle each move hits — the dividing line is 'emitted without reasoning' vs 'reasoned about'. The aria_invalid step DID ship 2026-09-29, as a resolved PROP not a map entry, which corrects the test this file proposed"
metadata:
  type: project
---

**Todd's idea, 2026-09-26, recorded as a TODO — nothing decided.** Once the
constraint attributes land as an `IndexMap<&'static str, Attribute>`
([[constraint-attributes-gap]]), the map is a general channel for "things that
end up on the element". So: move `path` (as `name`), `required` and `choices`
into it, and shrink `FieldProps` to `label` + `errors`.

The appeal is real — `FieldProps` stops growing every time a widget needs one
more thing, and a consumer's `custom(…)` widget gets the same open-ended channel
the built-ins use.

## What each move runs into

Not objections to the idea, just what has to be answered first. Each of these is
a case where the value is **used for a decision**, not merely emitted.

- **`path`** is not only `name="…"`. It is the key for
  `get_current(&path, values)` and `write_value(&path, values, …)`, it names the
  field in every dispatch panic, and `RadioGroup` clones it once per radio for
  its handlers. Buried in an `Attribute` it would have to be pulled back out and
  matched out of `AttributeValue::Text` to do a store read — round-tripping
  through an enum to recover a `String` the caller already had, and making store
  access depend on an attribute being present.
- **`required`** drives rendering, not just the attribute. `Select` branches on
  it to choose between an unselectable placeholder and a real `--none--` entry;
  the ` *` marker is gated on it; `RadioGroup` asserts on it. And the decisive
  case: **`Checkbox` deliberately does NOT emit it** — HTML `required` on a
  checkbox means "must be ticked", which is not what a required `bool` asks for,
  and the comment saying so is load-bearing. In a map that becomes "delete this
  key", which is a much weaker way to state an intentional exception. A possible
  middle: the bool stays a prop and the *attribute* is derived from it.
- **`choices`** is structured data that generates CHILD ELEMENTS — `<option>`s,
  or one `<input type="radio">` per entry — not an attribute. It could ride in
  `AttributeValue::Any(Rc<dyn AnyValue>)`, but that is smuggling data through an
  attribute channel rather than using one.

## The line this suggests

**Attributes are what a widget emits without reasoning about. Props are what a
widget reasons about.** By that rule the constraints belong in the map (no widget
branches on `maxlength`), and so would `placeholder`, `autocomplete`,
`inputmode`, `spellcheck` and any `data-*`/`aria-*` pass-through — which is the
author-attributes feature from the same plan, and the strongest version of this
idea. `path`, `required`, `choices` and `errors` are all reasoned about.

`label` is the interesting edge: mostly emitted, but widgets branch on its
`Option`, and it becomes element *content* rather than an attribute, so it is not
a candidate either.

## `aria_invalid` — DONE 2026-09-29, but NOT into the map

It was duplicated as `let invalid = (!errors.is_empty()).then_some("true")` in
all five of `Input`, `Textarea`, `Checkbox`, `Select` and `RadioGroup`, and it is
purely emitted, so it looked like the smallest first step *into the attrs map*.
**That was wrong, and the reason matters for the rest of this idea.**

It landed instead as a resolved PROP: `FieldProps::aria_invalid:
Option<&'static str>`, derived once in `FormField::render` and forwarded by every
widget. Two things ruled the map out:

- **`RadioGroup` puts it somewhere the spread cannot reach.** `..attrs` spreads
  onto the `<fieldset>`, while `aria-invalid` belongs on each `<input type=radio>`.
  Via the map it would land on the group container, and `FieldErrors`'s whole
  placement rationale — that a plain `input[aria-invalid="true"] + *` sibling
  selector reaches the message with no framework and no class — would stop
  working.
- **The merge rule is caller-wins.** `ScalarWidget` lets a consumer's attribute
  displace a derived one, which is right for `maxlength` and wrong for an
  accessibility attribute a consumer should not be able to silently override.

So "purely emitted" is necessary but NOT sufficient. The real test is also
**whether every widget emits it on the same element the spread lands on**, and
whether a consumer overriding it is acceptable. That kills the map for anything
a widget places structurally rather than on its one main element.

The precedent it does set is the useful half: `FieldProps` already carries
`label` as a pre-cased `Option<String>` rather than a `LabelCase`, so
**resolving at the boundary and handing the answer down** is the established
shape for this struct — the same rule as [[config-cascade]]. A third tier of
member, between "attribute in the map" and "prop the widget reasons about":
a prop the widget merely forwards, resolved once by the form.

Follow-on: the third ARIA state (`aria-invalid="false"`) is issue **#7** — it
needs validation state, not `FieldValue`'s parse state, plus a validate-on-load
policy decision. Also noted there: `aria_invalid` is redundant with `errors`
today, so a hand-built `FieldProps` can hold contradictory values and nothing
stops it (Todd flagged this as annoying, deferred) — #7 dissolves it, since
`Some("false")` stops being derivable from an empty `errors`.

**UPDATE 2026-10-08 (Todd): `required` and `aria_invalid` DO move into the
typed map**, folded into `ATTRIBUTES_PLAN.md` 3d. What changed since the
objections above: ownership in the attribute table stops a `form!` author
overriding either, and widgets now get the TYPED map, so `Select` asks
`contains(Attr::Required)` instead of reading a bool. `FormField::render`
puts both FIRST in a fresh map, then copies the field's attrs. The
`RadioGroup` objection still holds and is answered by hard-placing
`aria-invalid` on each radio, NOT by `is_valid_on` routing (the `AriaInvalid`
row lists `Fieldset`). `VariantSelect` gets a typed map too (Todd: "map").
