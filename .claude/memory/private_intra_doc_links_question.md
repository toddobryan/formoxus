---
name: private-intra-doc-links-question
description: "RESOLVED 2026-11-09: the `docs` recipe's comment was WRONG. `private_intra_doc_links` fires with or without --document-private-items (1.99.1 and stable); the public run caught nothing extra, so Todd dropped it (justfile now runs the private pass only)"
metadata:
  type: project
---

## RESOLVED 2026-11-09 (Todd + Claude)

Checked in a scratch crate on 1.99.1 and stable, each with and without
`--document-private-items`: a public doc linking to a private `mod`, a private fn,
or a `pub(crate)` fn fails `-D warnings` in ALL FOUR runs. The private run does not
silence the lint; it adds the note "this link resolves only because you passed
`--document-private-items`, but will break without". No case was found where the
public run caught something the private one missed, so the comment (written by
Claude in `5b0da99`, 2026-11-08) gave a false reason. Todd chose to DROP the public
run; `just docs` is now the private pass alone, ~11 s saved.

Two things neither run catches, found the same day:
1. A public doc linking to a `#[doc(hidden)]` item passes silently in both modes,
   though that page is never generated. formoxus has none today.
2. A hand-written HTML link is never checked. `formoxus-macros`' `path!` doc links
   `ModelPath` as `../formoxus/path/struct.ModelPath.html`, which works only from
   `formoxus_macros/macro.path.html` in a local workspace build: it is dead from
   formoxus's prelude page (where the doc is re-exported) and on docs.rs (separate
   crate docs). Found by scanning the generated HTML for missing local targets.

The original question, kept as history:


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
