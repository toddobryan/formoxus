//! Compile-fail tests for `form2!`'s diagnostics.
//!
//! Each `tests/ui/*.rs` is a deliberately-broken input; the matching `.stderr`
//! pins the exact compiler output — **including the span**. That's the point:
//! `form2!` checks every field path it names against the model's real shape by
//! emitting a witness expression per path, so a typo fails at compile time
//! rather than as a runtime `no_such_path`. These tests guard that the failure
//! stays *legible* — pointing at the offending path, in the caller's source.
//!
//! To regenerate after an intentional change:
//!
//! ```text
//! TRYBUILD=overwrite cargo test -p formoxus --test compile_fail
//! ```
//!
//! then *read the diff* before committing — an accidentally-worsened diagnostic
//! looks exactly like an intentional one to the tool. The goldens quote rustc's
//! own wording for a bad field access, so they're the ones that will need
//! regenerating on a toolchain bump.

#[test]
fn ui() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
