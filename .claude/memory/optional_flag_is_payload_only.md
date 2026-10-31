---
name: optional-flag-is-payload-only
description: "GOTCHA 2026-10-30 (both instances fixed): the walk's `optional` means 'this node IS an Option's payload'; set only by option_node for the one node it wraps, never inherited by rows or fields. Passing a container's own `optional` down made bool leaves tri-state selects"
metadata:
  type: project
---

The shape walk's `optional: bool` (`node_for_shape`, `ListSet.optional`,
`FieldSet.optional`, `VariantSet.optional`, `FieldNode.optional`) means
**this node is itself the `T` of an `Option<T>`**. `option_node` sets it
(`build.rs:470`) for exactly the one node it wraps, and every container
stops it there: `struct_node` builds fields with `false`, `list_node` rows
with `false`, `enum_node` a variant's fields with `false`. A child that can
be absent says so in its OWN shape (`Vec<Option<T>>`), which the walk peels.

**Why it matters:** a leaf's default widget follows it. `bool` with
`optional: true` is the tri-state `--none--`/True/False select instead of a
checkbox. `String` rows hide the bug: `TextInput` ignores the flag, `""`
fails `validate` either way, and nothing inside an `Option` renders
`required` (`OptionNode` renders through `ctx.optional()`).

**Found twice on 2026-10-30**, both in code that REBUILDS children after
construction and passed the container's own `optional` down:
- `ListSet::add_row`/`set_rows` → fixed: `new_row` always passes `false`,
  parameter removed. Test: `optional_containers.rs`, an `Option<Vec<bool>>`.
- `VariantSet::set_variant` → fixed the same way; `VariantSet.optional`
  (read only there) deleted with `enum_node`'s parameter. Test:
  `a_chosen_variants_field_gets_the_same_widget_as_a_populated_one`.

**How to apply:** any new code that builds nodes after construction
(an edit, a wire rebuild) must pass `false` for children, matching what
the construction walk passes. Test it with a `bool` leaf, never a `String`.
See [[facet-option-is-an-enum]].
