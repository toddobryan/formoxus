---
name: enum-as-a-value-choice
description: "Found 2026-09-25 writing radio_group's example: a unit-only enum is exactly what a VALUE choice wants, but formoxus forces it down the SHAPE-choice path where no widget can apply — so a radio group over an enum is impossible today"
metadata:
  type: project
---

Found by trying to write `radio_group`'s example over a three-variant `Gender`
enum. It does not compile, and the error is formoxus's own:

```text
error[E0080]: evaluation panicked: a widget applies only to a single-value field,
              not a struct, list or enum
   |    widget: radio_group {
   |            ^^^^^^^^^^^
```

**The gap.** An enum field is a SHAPE choice: it becomes a `VariantSet` with a
`VariantSelect`, which is not a leaf — no path, selection arrives as a prop, and
it emits `ChooseVariant` edits rather than writing a value. So `is_single_value`
rejects every widget on it, `radio_group` included. A value choice needs a leaf,
which today means a `String` field plus a `const` table of pairs. That is what
the example now does, and it works — at the cost of throwing away the enum.

**Why this is worth fixing rather than just documenting.** A *unit-only* enum is
precisely the type a value-choice field wants: the variant names ARE the choice
values, the type makes an invalid value unrepresentable, and the author has
already written the list once. Today it is the one shape that cannot be a value
choice, which is backwards.

Also worth knowing (it surprised me): a bare enum field with NO widget
declaration already renders a `<select>` over its variant names, automatically.
So the enum path is not unsupported, it is just *unconfigurable* — you get a
`<select>` and no say in it.

**The narrow version of the fix** is to let a `VariantSet` accept a widget, so an
enum's chooser can be radios instead of a `<select>`. That is a smaller change
than making unit-only enums into leaves, and it keeps the shape/value line
[[widget-table-and-choice]] draws. It sits right beside the `Chooser` idea in
[[chooser-ideas]], which would want the same thing — a `Chooser` over an enum is
the obvious case, and it cannot reach one either.

Open question either way: a `VariantSet` can be `Unchosen` (behind an
`OptionMember`), and `radio_group` refuses an `Option` because a picked radio
cannot be un-picked. So "radios for a variant choice" inherits that restriction
and the two rules have to agree.
