//! Compile-fail tests for `form!`'s diagnostics.
//!
//! Each `tests/ui/*.rs` is a deliberately-broken input; the matching `.stderr`
//! pins the exact compiler output — **including the span**. That's the point:
//! `form!` checks every field path it names against the model's real shape by
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

/// Diagnostics `form!` SHOULD give but does not yet, written ahead of the check
/// that will produce them. Each one compiles today, so this fails, which is
/// the point of running it:
///
/// ```text
/// cargo test -p formoxus --test compile_fail -- --ignored
/// ```
///
/// When a check lands, move its case to `tests/ui/` and generate the `.stderr`
/// with `TRYBUILD=overwrite`. They have no `.stderr` here on purpose: the
/// message and span are not decided until the check exists.
#[test]
#[ignore = "pending: the checks these need are not built yet"]
fn ui_pending() {
    trybuild::TestCases::new().compile_fail("tests/ui-pending/*.rs");
}
