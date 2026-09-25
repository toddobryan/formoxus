---
name: chooser-ideas
description: "Deferred ideas for the chooser widgets, queued 2026-09-25, NOTHING BUILT — a Chooser that picks select-vs-radios by list length, hint text per choice, and renaming SelectChoice to Choice (namespace verified clear)"
metadata:
  type: project
---

Queued 2026-09-25 while finishing `radio_group`. Nothing built. Extends
[[choice-fields-design]], whose tier 1 (`widget: select { choices: EXPR }`)
shipped 2026-09-21.

**A `Chooser` widget that picks `select` vs `radio_group` by list length.**
Todd's idea. One widget name in `form!`, resolving at render time on
`choices.len()`. The appeal is that it replaces an *advisory* with a decision:
the same conversation started from "should we discourage radio groups with too
many choices", and the answer "the library just picks the right one" is strictly
better than warning the author about a thing the library could have handled.

If it ships, it changes the threshold question from a lint into a rendering
parameter, which is a much easier thing to make configurable — and it would want
the same per-form/per-field knob as everything in [[config-cascade]].

Open questions: whether `Chooser` is the default widget for a field that
declares `choices` with no widget named; what it does about the fact that the
two widgets differ in more than looks (a `select` supports the optional
`--none--` entry, a `radio_group` deliberately refuses an `Option` at all — see
[[widget-table-and-choice]]); and whether swapping widget across a re-render
tears down and rebuilds the control, losing focus, the way `VariantSet`'s
comment warns about.

**Per-choice hint text.** An optional third field on the choice struct, rendered
as a caption under each radio. Motivated by Adam Silver,
<https://adamsilver.io/blog/dont-be-afraid-of-a-long-list-of-radio-buttons/>,
whose thesis (from the title — not read in full) argues a long radio list is
fine and is itself a mark against the advisory idea above. The post reportedly
claims a `select` cannot show hint text; **Todd's read is that CSS can in fact
do it on a `select`, and that claim is UNVERIFIED.** Worth settling before using
"only radios can have hints" as an argument for anything, because if a `select`
can carry hints too, then hint text is a property of the choice list rather than
a reason to prefer one widget.

**Rename `SelectChoice` → `Choice`.** The type is already used by `RadioGroup`
as well as `Select`, and would be used by a `Chooser`, so the `Select` prefix is
now wrong. [[choice-fields-design]] called it `Choice` in the original design
anyway, so this is a return to that name rather than a new one.

*Namespace: verified clear, 2026-09-25.* A probe test glob-importing
`dioxus::prelude::*`, `formoxus::prelude::*`, `googletest::prelude::*` and
`facet::Facet` fails with `cannot find type Choice in this scope` — nothing a
consumer or our own tests import exports that name.

Two things to sequence around, neither blocking:

- `ValueKind::Choice` (`fields.rs:66`) occupies the word today, but it is never
  constructed and [[choice-fields-design]] already says to DELETE it rather than
  fill it in. Do the rename with or after that deletion, so `Choice` the type
  and `ValueKind::Choice` never coexist.
- **The semantic caveat, which is the real one:** `VariantChoice` is the SHAPE
  choice (which enum variant), and this type is a VALUE choice (one option in a
  list). Those are kept rigorously distinct. Naming the value one bare `Choice`
  makes it read as the general case of which `VariantChoice` is a special case,
  which is exactly backwards. Not a reason to abandon the rename — `Choice` is
  the better name for the common type — but its doc comment should disclaim the
  relationship explicitly.
