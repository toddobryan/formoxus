---
name: no-shared-string-type
description: DECIDED 2026-11-12: no imstr/Arc<str>/Cow swap for formoxus's Strings; a third-party string type would leak into the public API (WidgetCommon's pub fields)
metadata:
  type: project
---

Todd asked (2026-11-12) whether all the `String` clones are worth replacing with
an immutable shared string (`imstr`). **Decided: no.** Todd: the fact that it
would infect the API is reason enough to avoid it.

**Why:** `WidgetCommon` has `pub id: String`, `pub label: Option<String>`, which
custom widgets read, so a crate's string type there ties formoxus's semver to
that crate. Nothing was measured: one keystroke re-renders one widget, and the
clones are a few dozen bytes each.

**How to apply:** don't re-propose a string-type swap on clone counts alone. The
one unexplored escape hatch, if a profile ever shows it: `NodePath(String)`
(`path.rs`) has a PRIVATE field, so `Arc<str>` inside it changes no API.
