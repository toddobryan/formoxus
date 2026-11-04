---
name: indexed-paths
description: "2026-11-03 BUILT: path!(M.list[i]) names ONE Vec item for an error; FormState resolves [n] -> item key; Vec only (sets refused); Path is a Cow, no longer Copy"
metadata:
  type: project
---

**`path!(Model.groups[g].$Group::All.questions[q])`** names one item of a
`Vec`, for an error about it (a `validator:`, `reject_field`,
`push_field_error`, `WireForm::errors_at`). Built 2026-11-03 for apcsp's
"already in group 1" (`ORDERED_LISTS_PLAN.md` box 7), design agreed with Todd.

- **Index, not key.** A validator sees the model, so indices; errors are
  keyed by item KEY (`groups.#3`), which moves and removals decouple from
  the index. The path is spelled `groups[1]…` and `crate::path::resolve_indices`
  turns each `[n]` into the nth key, reading each container's structure
  entry (`"#0,#3"`) by the path resolved so far. `FormState` resolves in
  `validate` and `push_field_error`; `WireForm::errors_at` from its values.
  `FormErrors::messages_at` CANNOT (no structure) and says so.
- **Past the end** falls through to the existing unclaimed fallback, a
  form-level "At path: groups[5]…, msg" (Todd: report, don't panic).
- **Vec only.** `[i]` on a set fails to compile via `path::Indexable`
  (`on_unimplemented`): a `HashSet` has no order and a `BTreeSet` iterates
  by `Ord`, which `sort_by` can disagree with, so an index could name the
  wrong item. A `Vec` marked `set` is fine: its model order IS the shown order.
- **`Path` is a `Cow<'static, str>`**, `Clone` not `Copy`, `as_str() -> &str`
  (not `const`). Three by-value `Path` params carry `#[expect(needless_pass_by_value)]`.
- **`form!` still refuses `[0]`** (its settings apply to every item); the
  message now points at `path!`. The shared grammar is `SpecPath::parse`
  (no index) vs `SpecPath::parse_indexed` (path! only), `Suffix::At(Expr)`.
- **Golden gotcha:** the witness call had to be respanned WHOLE onto the
  segment (`form::respan`, `__s` included) and use the trait method
  (`Indexable::items`), or rustc reported the set error twice, once under
  the whole invocation.

Not tied to [[error-model-design]]'s ErrorsStore (Step 7): resolution happens
before errors are stored, so Step 7 just calls the resolver. Related:
[[structure-on-the-wire]].
