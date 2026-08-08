//! Compile-fail tests for `#[derive(Form)]`'s diagnostics.
//!
//! Each `tests/ui/*.rs` is a deliberately-broken input; the matching `.stderr`
//! pins the exact compiler output — **including the span**. That's the point:
//! these guard the `syn::Error::new_spanned` shape checks and the `Debug` trait
//! bounds, so a refactor that quietly degrades an error (wrong span, lost
//! message) fails here instead of slipping through.
//!
//! To regenerate after an intentional change:
//!
//! ```text
//! TRYBUILD=overwrite cargo test -p formoxus --test compile_fail
//! ```
//!
//! then *read the diff* before committing — an accidentally-worsened diagnostic
//! looks exactly like an intentional one to the tool.
//!
//! `missing_debug` and `unknown_attribute` take their text from rustc / darling
//! rather than from us, so they're the ones that will need regenerating on a
//! toolchain or `darling` bump. They're still worth pinning: `missing_debug` is
//! the only guard that the `Debug` bounds actually fire on the user's own struct.

#[test]
fn ui() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
