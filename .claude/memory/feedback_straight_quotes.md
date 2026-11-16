---
name: feedback-straight-quotes
description: Emitted English (aria-labels, live-region messages) quotes with straight double quotes, never curly or single
metadata:
  type: feedback
---

Text formoxus emits that quotes a value — `Remove "big.pdf"`, `Edit "rust"`,
`"rust" was added successfully.` — uses straight DOUBLE quotes. Not curly
(“ ”), which Todd hates; not single, which clash with apostrophes in names
(`Remove 'Todd's essay.pdf'`). Switched everywhere 2026-11-16 (commit 1321ccc).

**Why:** Todd's preference (curly), plus readability (single).

**How to apply:** in Rust source write `\"{text}\"`; in a doc comment put the
example in backticks (`` `Replace "rust"` ``) so quotes don't nest. dioxus-ssr
escapes `"` as `&#34;` in both attributes and text, so a suite test matching
rendered HTML spells it `&#34;` (sets.rs `region()` escapes for you), while
`Harness::labelled` and Playwright see the plain `"`. See [[emitted-classes-and-strings]].
