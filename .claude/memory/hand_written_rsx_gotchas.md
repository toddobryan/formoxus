---
name: hand-written-rsx-gotchas
description: "Two rsx! traps hit writing the RadioGroup widget by hand (2026-09-25) — no statements in a `for` body, and what a forgotten `rsx!` wrapper looks like; will recur with every new widget in src/widgets/"
metadata:
  type: project
---

Hand-written `rsx!` in `formoxus/src/widgets/`, as opposed to the
macro-*generated* rsx traps in [[formoxus-darling-gotchas]]. These will recur:
the roadmap still has `select_multiple`, `checkbox_multiple` and `file` to write.

**An `rsx!` `for` body takes nodes, not statements.** `for x in xs { let y = …; … }`
inside an `rsx!` fails with a bare `error: expected identifier` and the caret on
the `=`. This bites whenever each iteration needs its own setup — the case that
found it was `RadioGroup`, where every radio needs its own `path.clone()` to move
into its own `onchange` handler, and there is nowhere in the loop to make the
clone. `Select` hides the problem by having exactly one handler for the whole
widget.

Fix: build the children as a `Vec<Element>` with `.map()` *outside* the `rsx!`,
where statements are legal, then splice with `{ items.into_iter() }`.
`members/field_set.rs` already does this for its members, so it is the house
idiom rather than a workaround.

**A forgotten `rsx! { … }` wrapper reads as a struct-literal parse error, and
masks everything else.** Without it, rustc parses `fieldset { class: "…" }` as a
struct literal and says so:

```text
error: expected one of `,`, `:`, or `}`, found `{`
   |     fieldset { class: "form-field radio-group",
   |     -------- while parsing this struct
   |         legend {
   |         ------ ^ while parsing this struct field
help: try naming a field
   |         legend: legend {
```

The tells: rustc talking about **structs and fields** where you wrote markup, and
offering to name an element as a field. Two traps in reading it:

- It is a *parse* error, so type checking never runs and NOTHING downstream is
  reported. A large rsx block that yields only two errors is itself the signal —
  a genuinely tangled one yields many.
- The accompanying `unused_imports` warning names imports the body visibly uses
  (`ValuesByPath`, `FieldProps`, `SelectChoice`, `get_current` were all reported
  unused), because the body they appear in never parsed. Do not chase it.

**Text vs nodes, for reference:** a text child is a quoted literal with the
interpolation inside the quotes (`"{text}"`, formatted like `format!`); a node
child is a braced expression (`{ element }`). An `if` with no `else` renders
nothing, which is how `Select` and `VariantSet` gate an optional label and the
` *` required marker — wrapper always present, contents gated.
