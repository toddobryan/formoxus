---
name: feedback-todd-owns-the-runtime
description: "Todd writes the runtime to own and understand it, so he can answer issues after publishing without relying on Claude: plans give what, why and traps, not finished code, unless he asks; but docs and dev-facing messages are mine to write"
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

**Tests written ahead of his code: NO implementation, not even a throwaway one to check
the tests** (Todd, 2026-11-14, after I ran the `FileType` `FromStr` tests against a
scratch implementation: "I was more interested in what you thought were good error
messages than in how easy it was to implement. Plus, what's the point of me implementing
it if you already have a good implementation?"). Two harms: (1) it bends the SPEC toward
what is easy to code, when the tests' job is to say what a dev SHOULD see, error wording
above all; (2) it is point 2 again, the working-out done and thrown away. So write the
tests from the plan and from the dev reading the message, and let his implementation be
their first run. If a test is wrong, he finds it, which he likes.

**Docs and dev-facing messages are mine to WRITE, even inside his runtime code**
(Todd, 2026-11-26: "I will usually have you write docs and user-facing messages.
You're much more likely to be consistent than I am, and I will review them at some
point."). That means rustdoc, module docs, and the text of every panic, `const _`
assert, `on_unimplemented` note and compile error a dev reads (the "user-facing" in
his words is a DEV reading formoxus's messages: [[dev-vs-user-vocabulary]]). When a
plan step of his has a message in it, write the message myself, in the house style
(`{prefix}:` first, the type by `{shape}` Display, then the ways out: a `types!` line,
then `field:`), rather than describing it for him to type. The logic around it stays
his. He reviews the wording later, so keep the same phrasing across sibling messages
and list what I wrote in my reply.
