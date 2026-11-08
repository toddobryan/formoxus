---
name: mutation-restore-mtime
description: "GOTCHA 2026-11-03: a mutation check that restores a file by moving a backup copy back leaves it OLDER than the mutated build, so cargo and dx keep the mutant; bump the mtime on restore"
metadata:
  type: feedback
---

A mutation-check script that does `shutil.copy(path, path + ".bak")`, writes
the mutant, runs the test, then `shutil.move(path + ".bak", path)` restores the
CONTENT but not freshness: the restored file's mtime is the copy's, from before
the mutant was written and built. Cargo (and `dx build`) decide freshness by
mtime against the last build's dep-info, so the restored file looks unchanged
and the MUTATED build is kept.

Found on box 10 (`sets`): after an e2e mutation run, `just e2e` kept failing
`edit_then_save_replaces_the_item` on a focus check that passed before, because
the wasm still had "no focus after Edit" in it. `git diff` showed the source
clean. Touching every source and rebuilding fixed it.

The MUTANT results are still valid (writing one bumps the mtime, so each is a
real rebuild); what goes wrong is every build AFTER the last restore, until
something else in that crate changes.

**Why:** a stale build makes a green or red run report on code that no longer
exists, and `git diff` cannot show it.

**How to apply:** restore with `os.utime(path)` after the move (or write the
original text back instead of moving the copy), and after any mutation session
`touch` the touched files before the run that will be reported. See also
[[e2e-harness]].

**A second way to keep the wrong build (2026-11-07):** two scratch copies sharing one `CARGO_TARGET_DIR` get the same hashes, as cargo hashes workspace paths relative to the root, and `git archive` stamps files with the commit time. So a fresh copy with no edits looks up to date and runs the OTHER copy's build. It made `HEAD` look identical to a prototype. Check for `Compiling formoxus`, or `touch` the sources.
