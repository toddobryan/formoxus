---
name: feedback-raise-parked-gaps-when-built-on
description: "When new code depends on a path that a known-limitation comment or characterization test warns about, raise it then. #13 was a parked gap ('create mode yields zero rows') that Submission::accept was later built on"
metadata:
  type: feedback
---

**A parked gap protects nothing once something is built on it.** `vecs.rs`
said "create mode yields zero rows today… Swap it once step 4 lands", and
`create_mode_yields_no_rows_yet` pinned the silence on purpose. Then the
2026-09-18 wire rework built `Submission::accept` on `empty_form`, exactly the
case the comment warned about, and nobody (me included: I wrote tests on both
sides) connected them. Result, found 2026-10-24 as #13: every enum's choice and
every list's rows were lost on the way to a server, two of them silently
(`Ok([])`, `Ok(None)`). Todd asked "how did we miss so many bugs?"; this was
point 2 of the answer, and he said to keep it.

**Why:** the comment and the characterization test made the gap look handled,
so the new code's author had no reason to look, and the suite stayed green
because the common round-trip test (`form_for(&m).validate() == Some(m)`)
"holds by construction" and never rebuilds anything.

**How to apply:**

- When writing or reviewing code that starts a new path through a node tree,
  a builder or a boundary (a server, a reload, a fresh form), grep for the
  known-limitation comments, `*_yet` characterization tests and `DEFERRED`
  plan boxes on that path, and say so if the new code leans on one.
- A characterization test that pins a limitation should name what must not be
  built on it until it is lifted, not just that it SHOULD start failing later.
- Prefer a property over examples at a boundary: `tests/suite.rs`
  `across_the_wire` is now asserted beside every in-memory round trip
  (`ISSUE_13_PLAN.md` box 19a). See [[feedback-one-function-not-two]] for
  the related "fewer parallel concepts" instinct.
