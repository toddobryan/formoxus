---
name: structure-on-the-wire
description: "DESIGN, from issue #13 (fixed 4dcf0fb, 2026-10-24): an enum's variant and a list's rows cross the wire as STRUCTURE values at the container's own path; the state owns them, never the store. RawValue, ordered_values, Unknown/rejected_keys"
metadata:
  type: project
---

**The rule.** `Submission::accept` rebuilds from `empty_form` (no variant
chosen, no rows), and a leaf can only land in a node that exists. So anything
that decides WHICH nodes exist crosses as a value of its own, at the
container's own path:

| container | entry | example |
|---|---|---|
| enum (`VariantSet`) | variant name, `""` unchosen | `shape` = `Circle` |
| list (`ListSet`) | row keys in ROW order, `""` none | `tags` = `#2,#0` |

Row order is why a list sends its keys rather than letting the server infer
rows from leaf paths: `AddRow { before }` gives a front row the NEWEST key,
and a row can have no leaves at all (`[[]]`, a `None` row).

**The state owns structure; the store holds leaves only (decision A).**
`VariantSelect` and the row buttons send `Edit`s to the state and never write
the store. `Node::collect_values` pushes `RawValue::{Leaf, Structure}` in ONE
walk (Todd rejected a second `collect_structure` walk, see
[[feedback-one-function-not-two]]). `FormState::raw_values` (private) is that
walk; `collect_values` = all of it (the wire), `leaf_values` (pub(crate)) =
leaves (store seeding: `use_form`, `use_form_values`, `reset`), and
`ordered_values` (behind `Form::to_wire` / `Form::validate`) takes a Leaf's
value from the STORE and a Structure's from the ENTRY (the state). Reading
structure from the store sends `""` after a pick on an empty form and the OLD
variant after a switch; mutation-checked in the suite and e2e.

**Distributing (decision B).** Structure first, then the leaves beneath.
Absent entry → leave as is (a leaves-only map must never unchoose); same
name/keys → no-op; `""` → unchosen / no rows; another real name → `set_variant`.

**Bad input is REMEMBERED, not pushed (decision C).** `validate` clears
`errors`, and `OptionNode::validate` clears an absent inner's, so a pushed
error would vanish and `Option<Title>` given "Sir" would be accepted as None.
So: `VariantChoice::Unknown(String)` (counted present by `is_present`) and
`ListSet::rejected_keys`, from which `validate` re-derives the error
(`unknown_variant_message`, no path). Not only forgeries: a stale tab after a
deploy renaming a variant.

**`absorb` writes a reply's structure into the store anyway** (Todd skipped
box 15): nothing reads a structure path from the store, and a key that turns
up where one should have been read is easier to debug present than filtered.

**The last silent case, pinned on purpose:** a hand-built payload that OMITS a
list's entry gets an empty list, `Ok`
(`submissions::a_missing_row_list_is_an_empty_list`). Do not build a
structure-less submit path (a native `<form>` post to `accept`) on it; the
variant `<select>` has no `name` either. See
[[feedback-raise-parked-gaps-when-built-on]].

**The property test:** `tests/suite.rs` `across_the_wire` is asserted beside
every in-memory round trip in the suite; the in-memory check alone "holds by
construction" and stayed green through #13. Full build record:
`formoxus/ISSUE_13_PLAN.md`, deleted once #13
closed; read it with `git show 4dcf0fb:formoxus/ISSUE_13_PLAN.md`.
