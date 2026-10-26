---
name: trim-by-default
description: "#16, DECIDED 2026-10-26, not built: text fields trim by default at parse time (raw kept as typed), `trim: false` opts out, `widget: password` defaults to no-trim; string newtypes incl. apcsp Markdown trim too; open: what numbers do with surrounding spaces"
metadata:
  type: project
---

**Issue:** [#16](https://github.com/toddobryan/formoxus/issues/16). Todd
raised it 2026-10-26 while working on apcsp-dioxus's MultipleChoice form: today
`"   "` passes `required`. Django's `CharField(strip=True)` is the model; the
widget survey ([[formoxus-widget-survey]]) already listed `strip` as missing.

Decided (Todd):

1. **Trim when parsing, not in the input.** The raw value stays as typed (a
   controlled input showing the trimmed value would eat a typed trailing
   space). Trim where raw text becomes `FieldValue`, so `"   "` → `""`, which
   is already absence ([[facet-form-design-decisions]]: "" IS absence).
   `Submission::accept` rebuilds from raw values, so the server trims the same
   way.
2. **`widget: password` defaults to `trim: false`.** Trimming a password
   inconsistently locks people out; Django's password fields set `strip=False`.
3. **String newtypes trim too, including apcsp's `Markdown`.** I raised that
   leading whitespace can be meaningful in Markdown (an indented code block).
   Todd: valid content for these fields shouldn't start or end with spaces;
   revisit only if a real field needs to.
4. **Spelled `trim: false`** (default `true`), matching `form!`'s
   `key: value` attributes rather than a bare `no_trim` flag.

**Still to check when building:** what number fields do with `" 42 "` today
(parse failure, or the parser already allows spaces?), so behaviour is
consistent. Also: `trim: false` on a type with no text (bool, fieldless enum)
should probably be a compile error, like other type-checked attributes.

**Why:** a whitespace-only answer would otherwise count as filled in, on both
client and server.
**How to apply:** when formoxus work resumes, this is a self-contained
enhancement; check #16 for the current state before starting.
