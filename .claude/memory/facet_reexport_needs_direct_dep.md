---
name: facet-reexport-needs-direct-dep
description: "2026-09-25: formoxus re-exports facet (`pub use facet`) for MACRO-generated code, but a model crate still needs its own `facet` dependency — #[derive(Facet)] hard-codes ::facet, and #[facet(crate = …)] does not cover builtin attrs like `transparent`"
metadata:
  type: project
---

Todd approved `pub use facet;` in formoxus on 2026-09-25, hoping users would
need only one dependency. **They still need two.** Verified with a scratch
crate depending on formoxus alone:

- plain `#[derive(Facet)]` → `E0433 cannot find facet in the crate root`.
- `#[facet(crate = ::formoxus::facet)]` → structs, enums, `#[facet(default)]`
  fields, and `form!` all build.
- …plus `rename`, `transparent` or `deny_unknown_fields` → fails again (`sensitive`, `skip`, `default` are fine; a renamed-dep repro with no formoxus in it behaves identically). facet's builtin grammar declares
  `crate_path ::facet::builtin;` (facet/src/lib.rs:121), and the `__attr!` arms it generates name
  `::facet::Attr::…` directly (which attrs hit those arms was found by testing,
  not by reading the storage rules). Same in 0.46.5 and 0.50.0-rc.7 (the latest release as of
  that date). Upstream bug: 98 hard-coded `::facet::` in make_parse_attr.rs; FIXED UPSTREAM-PENDING: facet-rs/facet#2662 (opened 2026-09-25, Todd's fork branch `crate-path-at-call-site`; a 0.46 backport is on `crate-path-at-call-site-0.46`, offered as optional since facet has never released a backport). Local clone: ~/code/rust/facet.

**Why the re-export still stands:** step 6a's const witness must name `Facet`
from the consumer's crate, and `::formoxus::facet::Facet` survives a Cargo
rename of facet where `::facet::Facet` does not.

**How to apply:** do not document formoxus as "one dependency". If facet ever
fixes the builtin crate_path, `crate = ::formoxus::facet` would make it true.
