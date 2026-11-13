---
name: feedback-plans-spell-out-the-why
description: Plan bullets and explanations for steps Todd implements must be full plain sentences, not telegraphic shorthand
metadata:
  type: feedback
---

When a plan step or explanation is for Todd to carry out, write it in full plain sentences: what to change, in which function, and what goes wrong otherwise. Short code, when it helps, beats a compressed description of the code.

**Why:** on 2026-11-05, working through `ERRORS_PLAN.md` box 2, Todd was stopped three times by compressed bullets: "validates into a map", "`ChooseVariant` arm owns the enum's own error", and the `edit` pruning bullet ("Your description is too telegraphic"). Shorthand like `my_path` in a bullet also caused a real bug, because it read as the parameter's name when the parameter is the prefix.

**How to apply:** before finishing a bullet, ask whether it makes sense to someone who has not just been reading the code. Applies with [[feedback-plan-task-list-format]].

**Say how big a step is** (2026-11-12). WIDGETS_PLAN 11b read as one small sub-box ("a
small guard"; `move_to`'s modes and atomicity in one trap sentence) and came to ~250
lines; Todd: "part of the reason I was having so much trouble with it is that I didn't
realize it would involve so much." When a step hides real work (several modes, a
fallback path, an atomicity rule), name each piece and its rough size, or split it.
