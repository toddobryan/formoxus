---
name: shrink-fieldprops-idea
description: "Todd's TODO 2026-09-26: now that widgets take an attributes map, consider moving path/required/choices into it and shrinking FieldProps to label+errors. Recorded with the obstacle each move hits — the dividing line is 'emitted without reasoning' vs 'reasoned about'"
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

`aria_invalid` is currently derived inside each widget from `errors.is_empty()`.
That one IS purely emitted and is duplicated across `Input`, `Select`, `Textarea`
and `RadioGroup` — the most defensible thing to move into the map, and a smaller
first step than any of the three above.
