---
name: test-only-impls-need-a-feature
description: A trait impl the integration suite needs but devs must never get goes behind cfg(any(test, feature = "__test-util")), switched on by formoxus's dev-dependency on itself
metadata:
  type: project
---

Since 2026-11-10 (PATHS_PLAN box 8), `NodePath` has `PartialEq<str>` and `PartialEq<&str>`
so the suite's ~350 path literals compare as `eq("venues.#3.city")`. Todd wanted them in
tests only: a live path compared to a bare string is what `NodePath` exists to stop.

Neither obvious place works:

- `#[cfg(test)]` alone is off when the library is built for its INTEGRATION tests
  (`tests/suite.rs` links a normally built `formoxus`), so only the inline tests see it.
- The suite cannot write the impls itself: it is a separate crate, and `PartialEq` and
  `NodePath` are both foreign to it (orphan rule).

What works, tried in a throwaway crate first: `#[cfg(any(test, feature = "__test-util"))]`
plus `#[doc(hidden)]` on the impls in `path.rs`; `__test-util = []` under `[features]`
in `formoxus/Cargo.toml`; and `formoxus = { path = ".", features = ["__test-util"] }`
under its `[dev-dependencies]`. `cargo tree` showed the examples crate's normal build
does not get the feature. `cargo publish` should drop the self-dependency, since it has
no version; not yet checked with a dry run (pre-publish).

**How to apply:** put any later test-only convenience (a constructor, a `From`, a
matcher-friendly impl) behind the same feature, not `cfg(test)`. Link: [[next-session]],
[[pre-publish-checklist]].
