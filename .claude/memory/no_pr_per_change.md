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

## Plans and memory ride along with a code commit (2026-09-26)

Do NOT make a standalone commit for a `*_PLAN.md` or a `.claude/memory/` edit.
Write them, leave them in the working tree, and include them in the next commit
that carries code. Todd: "No need to commit when you create them. When we commit
the project, please do commit them."

**END OF SESSION exception (Todd, 2026-10-09):** when he stops for the day,
"Commit and push the latest, even if it is just the next_session file." The
handoff has to reach the other machine (home ↔ school), so at a stopping
point offer to commit and push memory on its own rather than leaving it
uncommitted.

So they DO still get committed and pushed, which is what keeps `.claude/memory/`
authoritative and able to cross machines — the radio-group handover worked
because the plan and the resume entry had been pushed. The rule is about commit
granularity, not about keeping them out of git.


## The pre-commit hook needs `just hooks` on each machine (2026-09-29)

`cargo fmt --all --check` runs as a pre-commit hook, added after an unformatted
commit reached `main` and turned `just ci` red. Two things to know:

- **It lives in `.githooks/`, not `.git/hooks/`**, because the latter is not part
  of the repository and so exists on one machine only — which is the problem it
  was added to solve. Git finds it via `core.hooksPath`, which is per-clone
  config, so **run `just hooks` once on each machine**. A fresh clone has no hook
  until you do.
- **`git commit --no-verify` bypasses it**, which Todd asked for explicitly:
  work-in-progress commits should not be blocked on formatting. The hook says so
  in its own failure message.

Only `fmt` is checked, deliberately — it needs no compilation, so it costs under
a second. Adding clippy or the tests would make every commit slow enough that
bypassing became habit, which defeats the point. `just ci` is still the real gate.
