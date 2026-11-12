---
name: lists-and-sets
description: "2026-11-03 BUILT on branch `sets`, merge-ready (2026-10-31 decided): lists vs sets replaces allow_reordering; Vec = list (always <ol> + ↑/↓), Def::Set = set (<ul>, canonical order, no dups); plan in formoxus/SETS_PLAN.md"
metadata:
  type: project
---

**2026-10-31, Todd: a collection is a list (order matters) or a set (no
duplicates, canonical order). `allow_reordering` goes away**, a day after
box 5 of `ORDERED_LISTS_PLAN.md` shipped (`314bcd4`). Build plan:
`formoxus/SETS_PLAN.md`, on a branch.

**Status 2026-11-03: BUILT, every box of `SETS_PLAN.md` done,** branch
`sets` ready to fast-forward onto `main`. What shipped, beyond the
decisions below: the `set` key in `form!` (`treat_as_set` on `FormSpec`)
makes a `Vec` a set; a set's adder is a DRAFT item rendered by the item
type's own widget, outside the wire and validation, with its constraint
attributes off its input but checked on Add (`maxlength` kept); Edit
copies an item into the draft, whose Add becomes Replace, with Cancel and
Escape; `item:` swaps the item's text view; "row" became "item" everywhere.
`ORDERED_LISTS_PLAN.md` boxes 6–8 are still open, for apcsp after the merge.

- **Type-driven:** `Vec<T>` is a list, `BTreeSet`/`HashSet` (`Def::Set`) a
  set; a `set` key (name not yet confirmed) makes a `Vec` a set. No override
  the other way.
- **Every list** is an `<ol>` with ↑/↓; a set is a `<ul>` of display-only
  items with Edit and Remove.
- **Order, one rule:** the dev's comparator, else `T: PartialOrd` via facet,
  else `Debug` string. Todd REJECTED a case-insensitive default for strings
  ("no exception, make the common case easy"): strings sort by `Ord`, and
  formoxus exports a `case_insensitive` comparator.
- **Edit goes through the adder:** Edit copies the value into the adder's
  text box, Add replaces the original, cancelling keeps it. Editing in place
  would re-sort while the user types. The item being edited is MARKED, not
  removed (Todd, 2026-11-01), so a submit mid-edit still sends it.
- **A duplicate** is refused with a message beside the adder.

- **Vocabulary: an element of a list or set is an ITEM** (`Items` in
  `nodes/items.rs`, `add_item`, …), after facet's `begin_list_item`/
  `begin_set_item`. NOT "element" (Dioxus's `Element`, the return type of
  every `render`, and HTML's; Todd picked it first and dropped it) and NOT
  "member" (this crate's word for a node until 2026-10-16, all over old
  memory). A set's classes follow: `fx-set-items` on the `<ul>`, `fx-set-item` on each `<li>`, `fx-button-edit-item` (2026-11-01; M first said `member`).

**Why:** the question was never "may the user reorder this?" but "does
its order mean anything?" Django splits the same way (formsets with
`can_order` and sortedm2m, vs `MultipleChoiceField` and django-taggit, whose
parser dedups and sorts).

**How to apply:** don't propose per-list ordering flags again; anything
saying `allow_reordering` predates this. Related:
[[structure-on-the-wire]], [[django-parallel-design]].

- **`sort_by` (2026-11-01, decision L in the plan):** any expression of
  type `fn(&T, &T) -> Ordering`, or `(order, name)` with the name any
  expression; unnamed, the name is `stringify!(order)`. Checked by type
  through `field_kind::set_order_for(|m| row(&m.tags), order, name)`, not
  `const`: a `Shape`'s `ConstTypeId` is not comparable in const code. An
  optional set is `tags? => { sort_by: … }` (explicit `?`), so no trait
  peels `Option`.

- **Default order is for SCALAR items only (2026-11-01):** every other
  item needs `sort_by` (`sort_by: Tag::cmp` for an `Ord` type); `set_node`
  panics without one, `form!` checks named fields (4c4). Reason: nodes can
  give back only a leaf's value (`Node::peek_value`), a newtype's leaf
  holds its INNER value, and re-wrapping without a `T` needs `alloc_shape`,
  which is `unsafe` (denied workspace-wide). `sort_by`'s `Custom<T>`
  rebuilds whole `T`s safely with `Partial::alloc::<T>()`. No `unsafe`
  anywhere in formoxus; keep it that way.

**2026-11-11, WIDGETS_PLAN decision O (Todd):** a set of a unit enum has a default
order, declaration order (`DeclaredOrder` in `set_node.rs`), both at run time
(`SetOrder::default_for`) and in `form!`'s const check (`items_have_default_order`).
A newtype over a unit enum still needs `sort_by`. Before this, only scalar sets
sorted without one.

**2026-11-11, decision T (WIDGETS_PLAN):** `max_items: 1` is refused on a list or set — exactly one (with `min_items` >= 1) should be a `T`, none or one an `Option<T>`. Compile error for a literal in `form!`, build panic otherwise. Also that day: every Add/Remove re-checks the container's own count live (`check_item_count` in `form.rs`).
