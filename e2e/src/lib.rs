//! End-to-end browser tests for the example app, driven through `playwright-rs`.
//!
//! These drive a **real browser** against a **running** instance, so every test
//! is `#[ignore]`d: `cargo test --workspace` compiles them — which is what keeps
//! the selectors from rotting silently — and runs none of them. Run them
//! deliberately with `just e2e`, which builds, serves, waits, and tears down.
//!
//! **What this covers that `formoxus/tests/suite/` cannot.** The suite renders to
//! an HTML string, which proves *markup*. It cannot prove that a click lands,
//! that a value survives a round trip through `onsubmit`, or that a browser
//! actually blocks a submit on a `pattern` — and that last one is the assumption
//! behind emitting constraint attributes ungated.
//!
//! Tests run against the small single-purpose forms at `/t/<slug>`, not the
//! gallery. The gallery is for people evaluating formoxus; the test forms are for
//! this. One feature each keeps selectors unambiguous — one field, one button, no
//! scoping.
//!
//! Needs a browser once: `npx playwright@1.60.0 install chromium`.

mod config;
pub mod roles;
mod session;

pub use config::base_url;
pub use session::Session;
