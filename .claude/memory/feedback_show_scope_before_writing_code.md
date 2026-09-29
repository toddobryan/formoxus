---
name: feedback-show-scope-before-writing-code
description: "Recurring feedback: a short 'yeah' answering a narrow question is NOT approval to write code. Show the scope first. Todd raised this twice on 2026-09-28 in formoxus, and apcsp-dioxus records the same thing as a recurring issue"
metadata:
  type: feedback
---

**"You took off and starting writing code without asking me, again."** Todd,
2026-09-28. And `apcsp-dioxus/.claude/memory/feedback_check_before_implementing.md`
already recorded it as recurring, in his words: "a narrow answered question isn't
blanket approval; show plan/scope before writing substantial code or risky system
actions."

**Why:** he writes the core library code himself ([[user-role]]). Code I write
unbidden is not a shortcut for him — it is work he now has to review, and it takes
away the thing he was doing.

**How to apply:**

- A "yeah" or "OK" answering a specific question is agreement about the ANSWER,
  not authorization for the next step. The two instances both looked like
  approval: he agreed with a direction, and I read it as "build it."
- When he says he is doing something — "I'm plumbing it through" — that is him
  claiming the work, even if he then agrees with a suggestion about how.
- Asking for a *hint* means a hint. The `radio_group` case: he asked how to write
  a `legend`, and I rewrote the whole component.
- Say what I would change and where, in a sentence or two, and wait. "Want me to
  do that, or are you taking it?" has worked every time it was asked.
- It does NOT apply to tests, plans, memory, issues, justfile recipes or
  investigation — he asks for those directly and they have never caused friction.
- When he does delegate explicitly ("I'll leave you to work on steps 1-5"), build
  it, and surface the decisions I made on his behalf afterwards.
