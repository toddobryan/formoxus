---
name: dev-vs-user-vocabulary
description: "Todd's rule from 2026-10-16: \"dev\" = a developer building a Dioxus app WITH formoxus; \"user\" = the person using that app; \"Formoxus dev\" = someone changing formoxus itself (only Todd). Correct him when he slips"
metadata:
  type: feedback
---

**dev**: a developer using formoxus to build a Dioxus app. The person who
writes `form!`, `use_form`, a custom widget.
**user**: the person interacting with that app in a browser: types into
fields, submits, sees errors.
**Formoxus dev**: someone changing formoxus itself. Today that is only Todd,
so it rarely needs saying.

**Why:** Todd kept using "user" for both a dev and an app user, and it made
design talk ambiguous ("users never see `FormSpec`" vs "the user picks a
section"). He asked, 2026-10-16, to be corrected when he slips.

**How to apply:**
1. Use these words myself, in replies, plans, memory and doc comments.
2. When Todd says "user" and means a dev (or the reverse), point it out
   briefly and go on. Don't stop the discussion over it.
3. Older memory files and docs say "user" or "consumer" for a dev. Read
   them that way; fix them only when editing the file anyway.

Related: [[widget-is-the-umbrella-word]], the other vocabulary rule.
