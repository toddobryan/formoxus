---
name: toolchain-pin-and-goldens
description: "2026-10-21: rust-toolchain.toml pins 1.99.1 WITH rust-src; goldens flip between machines on a different stable OR a missing rust-src (core snippets vanish). CI pins must match the file"
metadata:
  type: project
---

**`rust-toolchain.toml` pins the toolchain (1.99.1, Todd, 2026-10-21)** with
`rustfmt`, `clippy`, `rust-src` and the `wasm32-unknown-unknown` target.

Two machine differences each made the trybuild goldens in `formoxus/tests/ui/`
flip, and both looked like a regression in `form!`'s diagnostics:

1. **Different `stable`s.** `form_constraint_on_an_optional_enum` was blessed
   on 1.96 (no "required by a bound in `field_for`" note, `8ea7b49`) and on
   1.98 at home (note present, `c151c41`), each undoing the other.
2. **`rust-src` missing.** Without it rustc omits the `core/src/option.rs`
   snippet under "tuple variant defined here", so
   `form_fractional_bound_on_an_integer` and
   `form_integer_bound_too_big_for_f64` lose three lines. A freshly
   auto-installed toolchain has no `rust-src`, hence the pin lists it.

**How to apply:** when a golden mismatches, first check `rustc --version` and
`rustup component list --installed | grep rust-src` before re-blessing. Bump
the pin deliberately: `rust-toolchain.toml` AND the three `"1.99.1"`s in
`.github/workflows/ci.yml`, with any regenerated goldens, in one commit. The
file overrides a CI job's installed toolchain, so `msrv` and `latest` call
`cargo +1.90` / `cargo +stable`.
