---
name: lint-config
description: "2026-09-23: the workspace grew a [workspace.lints] table. Records what is deliberately OFF and why — chiefly `unused_qualifications`, which Todd asked to be REMINDED to re-sweep periodically, and the module-file lints, whose names both read backwards"
metadata:
  type: project
---

## `unused_qualifications` is OFF, and Todd asked to be reminded

**Raise this every so often** — Todd's words, 2026-09-23: *"Remind me to check
it every so often to see if we have introduced new real issues and then we can
fix them."* A natural moment is after a batch of new widgets or suite tests.

The sweep, and why the grep:

```
cargo clippy --workspace --all-targets -- -W unused_qualifications
```

Every FALSE hit sits on a closure — `move |…|` or `|_|`. Anything else is real.
At the time it was dropped the split was **24 artifacts to 13 real**, and all 13
real ones were cosmetic, in test files (`formoxus::form_for` where `form_for`
was imported, `super::render_to_html` likewise, `dioxus::prelude::Event`). All
13 were fixed before the lint came out.

**Why it is off rather than allowed per-site.** It fires inside `rsx!`'s
expansion and blames the nearest token it can map back to. For
`onsubmit: move |e: FormEvent| {…}` rustc's own suggested fix is to delete
`onsubmit:` — the attribute name. There is no edit to the source that silences
it. Keeping the lint would have meant `#![allow(unused_qualifications)]` at the
top of seven modules including every widget file, which blinds exactly the code
most likely to grow a real one, and a new allow with every new widget.

## The module-file lints are named BACKWARDS

Verified in a clean two-directory crate, because the names mislead:

- `clippy::mod_module_files` → "`mod.rs` files are **not allowed**". **This is
  the one this repo wants**, and it is what is in the table.
- `clippy::self_named_module_files` → "`mod.rs` files are **required**"; it
  would demand `form.rs` become `form/mod.rs`.

Each is named for the layout it DETECTS AND REJECTS, not the one it enforces.
Commit `6cee3b3`'s message names the wrong one; the rename it made was still
right, since the repo uses `form.rs`+`form/` everywhere.

## Also deliberately off

`trivial_casts` — every hit is `Box<FormField<T>> as Box<dyn FormMember>` in
`dispatch!`, where the cast is what picks the trait object.

`must_use_candidate` and `return_self_not_must_use` — 80-odd annotations on a
builder-heavy API, none catching anything the types do not.

`missing_errors_doc` and `missing_panics_doc` — these want prose that belongs
to the deferred doc pass. See [[pre-publish-checklist]].

## Worth keeping an eye on

`clippy::cast_precision_loss` fires on `value_kind`'s `i128 as f64` — the same
bug `an_integer_bound_too_big_for_f64_is_caught_before_it_is_rounded` is
`#[ignore]`d against. When that closes, prefer `#[expect(…, reason = "…")]`
over an `allow` so it points back at the test.
