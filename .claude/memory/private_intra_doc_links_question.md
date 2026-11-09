---
name: private-intra-doc-links-question
description: "OPEN 2026-11-09, look with Todd: the `docs` recipe's comment says `private_intra_doc_links` is silenced under --document-private-items, but apcsp's identical run raised it as an error"
metadata:
  type: project
---

**Raised 2026-11-09 from apcsp, to look at together when Todd is back in formoxus.**

The `docs` recipe in formoxus's `justfile` runs rustdoc twice and its comment says
only the public run reports a public doc linking to a private item, because
`private_intra_doc_links` is "silenced once private items are documented". That is the
whole reason given for the second, public-only run.

apcsp's new `just docs` runs ONLY the private pass:
`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --document-private-items`,
and it still failed with exactly that lint:

> error: public documentation for `e2e` links to private item `config` (and the same
> for `session`)

The trigger was `crates/e2e/src/lib.rs`'s `//!` header linking [`config`] and
[`session`], two private `mod`s whose items are re-exported. Fixed in apcsp by linking
the re-exported items instead. The open question is only about formoxus's comment.

**To check:**
1. Reproduce in a scratch crate: a `//!` link to a private `mod`, built with and without
   `--document-private-items`. Does the lint fire in both? Is it the same toolchain
   (formoxus pins 1.99.1, see [[toolchain-pin-and-goldens]])?
2. If it fires in both, the comment is wrong about WHY two runs are needed. The public
   run may still be worth keeping (it's what docs.rs builds), but the reason should be
   corrected.
3. If it fires only in some cases (e.g. a crate-level `//!` doc vs an item doc, or a
   link to a module vs an item), write down the actual rule in the comment.
