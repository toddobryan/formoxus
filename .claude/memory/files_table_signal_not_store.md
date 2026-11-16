---
name: files-table-signal-not-store
description: DECIDED 2026-11-16 — the file table is one Signal, not a Store; dioxus-stores remove/clear don't wake a key's readers
metadata:
  type: project
---

The form's file table (`Contents.files`, `FilesSignal = Signal<IndexMap<NodePath, FileData>>`)
stays a single `Signal`, so a pick re-renders every file widget (WIDGETS_PLAN
decision KK, Todd 2026-11-16).

**Why:** a `Store<HashMap<…>>` doesn't buy per-widget re-rendering. In
dioxus-stores 0.7.10, `insert` wakes the key's own readers, but `remove`,
`clear` and `retain` call only `mark_dirty_shallow` (the map's shape). A view
reading just its key misses Remove and reset — and tests don't notice, since
Remove also writes `""` to the name in the value store. Reading the shape
(`get()`/`contains_key`) wakes on every pick anywhere, same as the Signal.
Real isolation needs `Option<FileData>` written to `None` instead of removed.

**How to apply:** if asked again, this is why. The same `remove`-doesn't-wake
trap applies to the VALUE store (`ValuesStore`) if anything ever removes keys
from it. f.9b (planned 2026-11-16) makes `FilesSignal` `pub(crate)` (`WidgetProps::new` drops its
`files` param) so the choice can change without breaking anyone. Related:
[[indexmap-store-not-worth-it]].
