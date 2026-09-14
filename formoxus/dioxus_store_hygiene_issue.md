# Draft issue for DioxusLabs/dioxus

Not part of the build — a draft to review and file upstream, then delete.

---

**Title:** `#[derive(Store)]` emits unqualified `dioxus_stores::…` paths — breaks without `use dioxus::prelude::*` in scope (macro hygiene)

**Version:** `dioxus` 0.7.9 / `dioxus-stores` 0.7.9 / `dioxus-stores-macro` 0.7.9

## Summary

The `Store` derive expands to **unqualified** `dioxus_stores::…` paths, so it silently depends on the name `dioxus_stores` being in the caller's scope. That's true for typical code that does `use dioxus::prelude::*` (the prelude re-exports the crate name via `pub use dioxus_stores::{self, …}`), but it fails anywhere that glob isn't present — most notably in **macro-generated** structs and in crates that re-export the derive through a facade. A derive shouldn't rely on ambient imports.

## Minimal reproduction (no proc-macro needed)

A crate depending only on `dioxus`:

```rust
use dioxus::prelude::Store; // just the derive, NOT the prelude glob

#[derive(Store, Clone, Default)]
struct Foo {
    a: i32,
}
```

```
error[E0433]: cannot find module or crate `dioxus_stores` in this scope
  = note: this error originates in the derive macro `Store`
```

Change the import to `use dioxus::prelude::*;` and it compiles — because the glob brings the `dioxus_stores` *name* into scope, which the derive's expansion happens to reference.

## Root cause

`dioxus-stores-macro` emits bare `dioxus_stores::…` (e.g. `dioxus_stores::Store<…>`, `dioxus_stores::macro_helpers::dioxus_signals::…`) rather than a self-contained path. Since a consumer of the `dioxus` facade doesn't have a crate literally named `dioxus_stores` as a direct dependency, the generated code only resolves by accident of the prelude glob being imported.

## Impact

Anyone generating `Store`-deriving structs from their own proc-macro (or re-exporting the derive from a wrapper crate) hits `E0433 dioxus_stores` in generated code, and has to work around it by injecting a `dioxus_stores` alias into the caller's scope.

## Suggested fix

Either of:

1. **A `#[store(crate = "…")]` escape hatch** (preferred) — mirrors `serde`'s `#[serde(crate = "…")]`, letting downstream macros / facade re-exports redirect the crate path. Cleanest for macro authors.
2. **Emit absolute `::dioxus_stores::…` paths** *and* have the `dioxus` facade guarantee `dioxus_stores` is resolvable for crates that depend only on `dioxus` (e.g. a documented re-export), so it no longer depends on the prelude glob.
