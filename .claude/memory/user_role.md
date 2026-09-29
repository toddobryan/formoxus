---
name: user-role
description: "How Todd works on formoxus: he writes the core library code himself and is newer to Rust than to programming; I write tests, boilerplate, plans and memory, explain Rust idioms, and investigate. Mirrors apcsp-dioxus's own user_role memory"
metadata:
  type: user
---

Mirrors `apcsp-dioxus/.claude/memory/user_role.md`, which says: "Todd writes core
app code himself, new to Rust; I write tests/boilerplate, explain Rust idioms."
The same holds here, and formoxus's memory did not record it until 2026-09-29.

What that looks like in practice, from working through `radio_group`, the
constraint attributes and the e2e harness:

- **He writes the library code.** The useful things to hand him are a *hint* and
  the reason behind it, not a finished function. He said so directly: "I think if
  I'd had the missing `rsx!` I would have been able to figure out the rest."
- **He is often editing the same files at the same time.** Re-read before
  patching; several times a file had changed under me mid-task, including
  `buttons.rs`, `scalar.rs` and `main.rs`. A stale `git diff` is a real hazard.
- **Investigation is welcome and wanted.** Reading the installed `dioxus-html`
  source, running a document through the W3C validator, probing `playwright-rs`'s
  API — those consistently landed better than reasoning from memory, and twice
  overturned something I had asserted confidently.
- **Tests, plans, memory, issues and recipes are mine to write.** He asks for
  those directly and they have not caused friction.

See [[feedback-show-scope-before-writing-code]] for the corollary he has had to
state more than once.
