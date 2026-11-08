---
name: feedback-hard-to-misuse
description: When a dev-facing API has a wrong way that compiles, Todd prefers reshaping it so the wrong way takes effort; documenting the pitfall is the fallback
metadata:
  type: feedback
---

When a public API can be used wrongly and still compile (two calls that must go
together, a raw field that skips a step), propose a shape that makes the wrong way hard
before proposing docs. Todd, 2026-11-08, on a custom widget calling `write_value` without
the live-check trigger: "If we can make the API more ergonomic, so devs would have to
work hard to do it wrong, that would be great. Otherwise, we document."

**Why:** formoxus is about to be published, and every footgun becomes an issue Todd has
to answer ([[feedback-todd-owns-the-runtime]]).

**How to apply:** the moves that have worked: make the raw thing `pub(crate)` and route
through one method that does every step (`base_attrs` → `current_attrs` in ERRORS_PLAN
4c; `values`/`write_value` → the per-field handle in 6c); name methods after what the
dev is already handling (DOM events). Say what can STILL go wrong afterwards, and put
that in the docs. Related: [[feedback-macro-first]], [[feedback-one-function-not-two]].
