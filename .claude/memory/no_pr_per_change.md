---
name: no-pr-per-change
description: Todd does NOT want a pull request for each piece of work; commit (and push when he says so) instead. PRs only when he asks for one
metadata:
  type: feedback
---

Don't open a GitHub PR for each piece of work in formoxus. Commit locally, and
push or merge to `main` when Todd says to. Open a PR only when he explicitly
asks for one.

**Why:** On 2026-09-25 he asked for ONE PR, for step 6a, to see what that
review flow looked like. I then opened PRs for 6b and for the widget checks
unasked, and he said: "I don't want a PR for every bit of work you do. I just
wanted to see that one example."

**How to apply:** A request like "make a PR for me to review" applies to that
piece of work only. Otherwise, finish with a commit on a branch or on `main`,
say what is committed and pushed, and ask before pushing if he hasn't said to.
