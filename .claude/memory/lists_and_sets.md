---
name: lists-and-sets
description: "2026-10-31 DECIDED: lists vs sets replaces allow_reordering; Vec = list (always <ol> + ↑/↓), Def::Set = set (<ul>, canonical order, no dups); plan in formoxus/SETS_PLAN.md"
metadata:
  type: project
---

**2026-10-31, Todd: a collection is a list (order matters) or a set (no
duplicates, canonical order). `allow_reordering` goes away**, a day after
box 5 of `ORDERED_LISTS_PLAN.md` shipped (`314bcd4`). Build plan:
`formoxus/SETS_PLAN.md`, on a branch.

- **Type-driven:** `Vec<T>` is a list, `BTreeSet`/`HashSet` (`Def::Set`) a
  set; a `set` key (name not yet confirmed) makes a `Vec` a set. No override
  the other way.
- **Every list** is an `<ol>` with ↑/↓; a set is a `<ul>` of display-only
  members with Edit and Remove.
- **Order, one rule:** the dev's comparator, else `T: PartialOrd` via facet,
  else `Debug` string. Todd REJECTED a case-insensitive default for strings
  ("no exception, make the common case easy"): strings sort by `Ord`, and
  formoxus exports a `case_insensitive` comparator.
- **Edit goes through the adder:** Edit copies the value into the adder's
  text box, Add replaces the original, cancelling keeps it. Editing in place
  would re-sort while the user types.
- **A duplicate** is refused with a message beside the adder.

- **Vocabulary: an element of a list or set is an ITEM** (`Items` in
  `nodes/items.rs`, `add_item`, …), after facet's `begin_list_item`/
  `begin_set_item`. NOT "element" (Dioxus's `Element`, the return type of
  every `render`, and HTML's; Todd picked it first and dropped it) and NOT
  "member" (this crate's word for a node until 2026-10-16, all over old
  memory). A set's items are still shown as `fx-set-member`s.

**Why:** the question was never "may the user reorder this?" but "does
its order mean anything?" Django splits the same way (formsets with
`can_order` and sortedm2m, vs `MultipleChoiceField` and django-taggit, whose
parser dedups and sorts).

**How to apply:** don't propose per-list ordering flags again; anything
saying `allow_reordering` predates this. Related:
[[structure-on-the-wire]], [[django-parallel-design]].
