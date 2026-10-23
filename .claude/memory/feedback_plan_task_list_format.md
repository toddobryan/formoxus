---
name: feedback-plan-task-list-format
description: "Todd reads plan files in VS Code's Markdown Preview: write numbered to-do items as `- [ ] **N.** …` with a blank line before each list, never `N. [ ] …`, or the numbers vanish and lists run together"
metadata:
  type: feedback
---

**Todd reads `*_PLAN.md` boxes in VS Code's Markdown Preview** (2026-10-22).
Two rendering rules broke the "Delete `BuiltinWidget`" box for him:

1. **A task item hides its list number.** `5. [ ] …` renders as a checkbox
   with the numbering styled away, so "item 5" pointed at nothing.
2. **An ordered list can only interrupt a paragraph if it starts at `1.`**
   (CommonMark). A checkpoint's list starting at `5.` right under its heading
   line ran into the paragraph.

**Why:** he refers to items by number ("items 1–6"), so the number has to be
visible; and he had to add blank lines by hand to see the boxes at all.

**How to apply:** write numbered to-dos as `- [ ] **5.** text`, number in
bold, with a blank line between any paragraph or heading line and the list.
Continuation lines indent to the item's text. Older boxes in
`FIELDS_PLAN.md` use `N. [ ]`; convert one when editing it anyway. See
[[feedback-numbered-not-bulleted]], which is about REPLIES, not files.
