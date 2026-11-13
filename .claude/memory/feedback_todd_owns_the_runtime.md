---
name: feedback-todd-owns-the-runtime
description: "Todd writes the runtime to own and understand it, so he can answer issues after publishing without relying on Claude: plans give what, why and traps, not finished code, unless he asks"
metadata:
  type: feedback
---

**Lean toward giving Todd the what, the why and the traps, not finished code, unless he
specifically asks me to write something.** He then writes it and I review. How much he
wants shifts with how tired he is, so when he asks for code, give it: having him make the
change himself is still useful, because he sees what is going on locally.

**Why:** Todd, 2026-11-07: "I do want to own and understand the code. I'd like to publish
this, and that will mean getting issues and having to respond to them. I don't want to
have to rely on you." He noticed he was retyping code I had already built in a scratch copy
and proved, which saved me nothing and cost him the working-out.

**How to apply:**

1. A plan step says what has to change, why, where, and what will go wrong. No code blocks
   he would only transcribe. Full sentences still: [[feedback-plans-spell-out-the-why]].
2. Do not build a whole step in a copy before handing it over. Prototype only to settle a
   specific doubt, like whether a read subscribes the form, and report the finding, not the
   code. Claims about behaviour should still be measured, not assumed.
3. "Expand this a bit" means more explanation, not a prototype.
4. Review what he writes; that is where I earn my keep. Tests, plans, memory and docs stay
   mine: [[user-role]].

**Code he asks for: hand it over, don't compile or run it first** (Todd, 2026-11-12:
"In the future, you don't need to compile and run. If I get errors, I can try to fix
them or come back to you if there's a problem."). I had built `TemporaryUploadedFile`
in a scratch crate with tests before giving it to him. Point 2's "measured, not assumed"
still applies to CLAIMS about behaviour (what facet or axum does), not to code he will
compile himself.
