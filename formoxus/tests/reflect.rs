//! Integration tests for the reflection path.
//!
//! Most reflect tests live inside the crate (`src/reflect/tests/`) because they
//! reach crate-private items. This target exists for the ones that CAN'T:
//! anything exercising formoxus's own macros has to be written the way a
//! consumer writes it, from a crate where the path `formoxus::` resolves — which
//! rules out formoxus itself.
//!
//! Submodules live in `tests/reflect/` and need `#[path]`, because cargo only
//! auto-discovers `tests/*.rs` and would otherwise look for `tests/<name>.rs`.
