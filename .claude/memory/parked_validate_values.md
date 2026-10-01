---
name: parked-validate-values
description: "PARKED 2026-09-30 at Todd's request — make FormState's distribute→validate order hard to get wrong: add validate_values(values) that does both, keep validate() for no-new-input, and fold distribute_form_values into distribute_values via impl IntoIterator. Not decided; come back to it"
metadata:
  type: project
---

**Parked 2026-09-30** ("We'll come back to the distribute -> validate discussion.
Hang onto it."). Todd's goal: callers of a plain `FormState` should not have to
be *quite so* conscious of the workflow's details — not hide them entirely.

**Who is exposed.** `Form::validate` (store → `distribute_values` → `validate`)
and `Submission::accept` (`empty_form` → `distribute_values` → `validate`)
already wrap it. The exposure is anyone holding a bare `FormState`: the suite,
and server code that skips `Submission`. What they must know unprompted: values
go in BEFORE `validate`, and `validate` on stale values passes silently.

**Proposed, not decided:**

1. `FormState::validate_values(values) -> Option<T>` — distribute then validate
   in one call; the name says both halves happen. `validate()` stays for "judge
   what is already there" (a seeded edit form, after pushing errors), with a doc
   line saying it reads no new input.
2. `distribute_values` takes `impl IntoIterator<Item = (String, String)>` and
   `distribute_form_values` is deleted — a map and a submitted `<form>`'s
   `(name, value)` pairs both fit, so there is one entry point, not two that
   differ only in container. `distribute_leaves` needs keyed lookup, so it
   collects into a `ValuesByPath` internally.

**Rejected:** making `validate` REQUIRE values. Tidier, but it breaks the
legitimate no-new-input uses and makes them pass `&form.collect_values()` back
to themselves.

**Expect:** many of the suite's ~31 `distribute_form_values(…); validate()`
pairs collapse to one call; some deliberately inspect state between the two
and keep both.

Context: the 2026-09-30 rename to `collect_X` / `distribute_X` pairs
(`collect_leaves`/`distribute_leaves`, `FormState::collect_values`/
`distribute_values`, `collect_errors`/`distribute_errors`).

## Also parked 2026-09-30 — two more Todd wants to come back to, unexamined

1. **`collect_leaves` building a `ValuesByPath` directly**, to parallel
   `distribute_leaves` (which takes one). Today it pushes into a
   `Vec<(String, String)>` and `FormState::collect_values` converts.
2. **Typed path keys instead of strings.** Todd is uneasy with the amount of
   string manipulation (`qualify`, `model_path`, `$Variant` segments, `#n`
   rows). He would rather values and errors were keyed by a Path STRUCTURE that
   can be error-checked, converting to and from `String` only at the wire
   boundary. Relates to [[error-model-design]] (storage positional, path erased
   at routing) and to `Path<T>`, whose `&'static str` is exactly why it cannot
   cross the wire today.
