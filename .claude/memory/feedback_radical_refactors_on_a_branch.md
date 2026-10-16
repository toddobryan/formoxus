---
name: feedback-radical-refactors-on-a-branch
description: "Todd 2026-10-16: for a big refactor on a branch, prefer one radical change that breaks the build over a series of small always-compiling steps; compiler errors are the to-do list"
metadata:
  type: feedback
---

Todd, 2026-10-16, about the Django-parallel refactor on `widget`: in a big
refactor he would rather do "radical surgery" in one go (e.g. fold
`FieldProps`, `WidgetProps`, `FieldAttrs` and `ScalarWidget`'s props into one
struct at once) than a chain of small changes that each keep everything
compiling.

**Why:** when he keeps the build green across many small steps he loses
track of which call sites he has not yet changed. A broken build lists them
for him; the compiler errors are the progress meter. Precondition: good
tests and the work on a branch, not `main`.

**How to apply:** my default is to keep each step compiling and preserve
existing names and call sites. Inside a big refactor on a branch, drop that:
propose the end-state shape directly and let it break. Still keep the tests
as the safety net (rewrite them deliberately, don't delete coverage), and
still show scope before writing code ([[feedback-show-scope-before-writing-code]]).
Outside big branch refactors, small contained changes remain the default.
