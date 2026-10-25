---
name: feedback-one-function-not-two
description: "Todd prefers one function that handles both cases over two near-duplicate functions doing the same traversal, even when the split would be cheaper to migrate to"
metadata:
  type: feedback
---

**"I really don't like the idea of two functions doing what is basically the
same thing. Let's take the cost to create one function that handles both."**
Todd, 2026-10-24, on #13 Q1 (the #13 plan, now only in git: `git show 4dcf0fb:formoxus/ISSUE_13_PLAN.md`): I recommended a second node
walk, `collect_structure`, beside `collect_values`, because it left existing
callers and ~34 test uses untouched. He chose ONE walk with tagged entries
instead, accepting the filter at the callers and the test churn.

**Why:** it is the same "fewer concepts" goal behind the whole fields refactor
([[django-parallel-design]]): two functions that traverse the same tree drift,
and a reader has to learn both.

**How to apply:** when a design adds a parallel function, trait method or type
that mostly duplicates an existing one, recommend extending the existing one
(a tag, a parameter, an enum) first, and present the split only as the
alternative, with migration cost as its one argument. Migration cost alone is
not a reason he accepts.
