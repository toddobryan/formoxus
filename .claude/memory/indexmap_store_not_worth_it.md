---
name: indexmap-store-not-worth-it
description: "Todd asked (more than once) how hard a Dioxus Store over IndexMap would be. Answer 2026-10-24: feasible (~150 lines, an extension trait), but it would NOT remove ordered_values, so not worth it; an upstream dioxus-stores PR is the cleaner route"
metadata:
  type: project
---

**Asked again 2026-10-24 ("I think I asked this before"); this records the
answer so it is not re-derived.** Checked against `dioxus-stores` 0.7.10.

**Feasible.** `HashMap` support is not built into `Store`: `src/impls/hashmap.rs`
is an inherent `impl` on `Store<HashMap<K,V,St>, Lens>` plus a `GetWrite`
lens (`Readable` + `Writable`), and `btreemap.rs` is a copy of it. Its parts
are public (`selector()`, `into_selector()`, `SelectorScope::hash_child_unmapped`,
`map_writer`, `mark_dirty`, `track_shallow`, `write_untracked`,
`From<SelectorScope> for Store`); only `SelectorScope::as_ref` is `pub(crate)`,
and `.clone()` replaces it (`SelectorScope` is `Clone`). formoxus would need a
LOCAL extension trait (orphan rule: no inherent impls on a foreign type) with
`get_unchecked` + `insert`, and an `IndexMap` copy of `GetWrite`. Formoxus only
uses `get_unchecked(..).try_read()/.set()`, `insert`, and reading the whole map
(`widgets/values.rs`, `FormState::ordered_values`). ~150 lines. Risk: these are
low-level APIs, the likeliest to churn in Dioxus 0.8.

**Not worth it, because it would not remove `ordered_values`.** An `IndexMap`
keeps INSERTION order, and the store's insertion order is not the tree's: a
row added after mount, or a variant's fields revealed later, are inserted at
the end. So `ordered_values` still takes paths and order from the tree, drops
stale keys, and (since #13) picks each value's source by `RawValue` kind. The
switch would save only the `.collect::<HashMap<_,_>>()` at seeding and the
"the two maps differ" paragraph on `ValuesStore` (`nodes.rs`).

**If wanted anyway:** an upstream PR adding `impls/indexmap.rs` behind an
`indexmap` feature, modelled on `btreemap.rs`, rather than formoxus carrying
it. Related: `formoxus/dioxus_store_hygiene_issue.md` (another upstream draft).
