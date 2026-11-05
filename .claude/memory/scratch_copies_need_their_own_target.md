---
name: scratch-copies-need-their-own-target
description: A `git archive` copy built with CARGO_TARGET_DIR on the repo's target reads stale builds as fresh, in both directions
metadata:
  type: feedback
---

Never point a throwaway copy of this repo at the repo's `target`. `git archive` gives
files their COMMIT time, and cargo's hash for a workspace path package does not include
the absolute path, so the copy and the repo share fingerprints. Old mtimes then read as
fresh: on 2026-11-05 a copy of 2db98c1 compiled its tests against another copy's lib.

**Why:** a result from the wrong code looks like a real result, and the repo's own next
build can pick up the copy's artifacts.

**How to apply:** give the copy its own target dir, or `touch` its sources and afterwards
`cargo clean -p formoxus -p formoxus-macros -p formoxus-spec -p formoxus-examples -p e2e`
plus `rm -rf target/dx`. To prove which code a binary is, search it for a string only that
code has; `grep -c` printed nothing on these binaries, so use Python's `mmap`.
