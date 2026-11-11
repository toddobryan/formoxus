---
name: indexed-paths
description: "2026-11-03 BUILT: path!(M.list[i]) names ONE Vec item, path!(M.set[= v]) ONE set item by value, for an error; FormState resolves to the item key (`ModelPath::resolve` -> `NodePath` since 2026-11-10); ModelPath is a Cow, no longer Copy"
metadata:
  type: project
---

**`path!(Model.groups[g].$Group::All.questions[q])`** names one item of a
`Vec`, for an error about it (a `validator:`, `reject_field`,
`push_field_error`, `WireForm::errors_at`). Built 2026-11-03 for apcsp's
"already in group 1" (`ORDERED_LISTS_PLAN.md` box 7), design agreed with Todd.

- **Index, not key.** A validator sees the model, so indices; errors are
  keyed by item KEY (`groups.#3`), which moves and removals decouple from
  the index. The path is spelled `groups[1]…` and `ModelPath::resolve`
  (pub(crate); the free fn `resolve_indices` until 2026-11-10, #21) turns
  each `[n]` into the nth key, reading each container's structure entry
  (`"#0,#3"`) by the path resolved so far, and returns a `NodePath`: the
  model path goes in, a live path comes out, and that is the only bridge. `FormState` resolves in
  `validate` and `push_field_error`; `WireForm::errors_at` from its values.
  `FormErrors::messages_at` CANNOT (no structure) and says so.
- **Past the end** falls through to the existing unclaimed fallback, a
  form-level "At path: groups[5]…, msg" (Todd: report, don't panic).
- **Vec only.** `[i]` on a set fails to compile via `path::Indexable`
  (`on_unimplemented`): a `HashSet` has no order and a `BTreeSet` iterates
  by `Ord`, which `sort_by` can disagree with, so an index could name the
  wrong item. A `Vec` marked `set` is fine: its model order IS the shown order.
- **`ModelPath` is a `Cow<'static, str>`**, `Clone` not `Copy`, `as_str() -> &str`
  (not `const`). Three by-value `ModelPath` params carry `#[expect(needless_pass_by_value)]`.
- **`form!` still refuses `[0]`** (its settings apply to every item); the
  message now points at `path!`. The shared grammar is `SpecPath::parse`
  (no index) vs `SpecPath::parse_indexed` (path! only), `Suffix::At(Expr)`.
- **Golden gotcha:** the witness call had to be respanned WHOLE onto the
  segment (`form::respan`, `__s` included) and use the trait method
  (`Indexable::items`), or rustc reported the set error twice, once under
  the whole invocation.

**A set's item by VALUE, `path!(Post.tags[= "rust"])`** (same day, Todd:
"before we need it", for a choice that stops being legal at run time). A set
item is unique by value, so a value names it where an index cannot.
- The value is a REFERENCE to the item or to what it borrows as (`&str` for
  `String`): `path::ItemOf::is_item` with `Item: Borrow<Q>`, set types only
  (`BTreeSet`, `HashSet`; a `Vec`, even one marked `set`, uses `[i]`).
- Matched by TEXT, not typed equality, so `WireForm` can resolve it too:
  `path::item_value_key` writes `[=text]`, the text the item node stores
  (newtype → inner scalar, unit enum → variant name, else facet display;
  mirrors `build::node_for_shape`), with `%`, `.`, `[`, `]` percent-encoded.
  The resolver looks for the key whose entry is that text, so `FormState`
  now passes `collect_values` (leaves AND structure), not structure only.
- Last in the path (a set item is a leaf). Its witness is a CLOSURE, never
  called, since a `fn` item cannot capture the author's local value.
- Static `choices:` need none of it: an item outside them is already
  refused by the field, on that item.

Not tied to [[error-model-design]]'s ErrorsStore (Step 7): resolution happens
before errors are stored, so Step 7 just calls the resolver. Related:
[[structure-on-the-wire]].
