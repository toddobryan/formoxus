---
name: facet-050-compatibility
description: "Verified 2026-09-19 by building the whole workspace against it: facet 0.50.0-rc.7 is a DROP-IN for formoxus — one version string, zero code changes, all 359 tests pass. The 0.47-0.49 gap is not a reflection-core rewrite. Do not move onto the rc; this exists so the day 0.50 goes stable is a one-line bump and not an investigation"
metadata:
  type: project
---

## What was measured, and how

`facet` publishes nothing stable after **0.46.5** — the next 8 releases are all
`0.50.0-rc.0` .. `rc.7`. **0.47, 0.48 and 0.49 do not exist.** That gap reads
like a major rework, which matters more here than for a typical dependent:
the whole `reflect` path is built on facet's reflection *internals* (vtables in
place of `FromStr`/`Display`, `&'static Shape` as an identity key,
`begin_nth_field(0)` for newtype peeling, `Def`/`Type` matching in `build.rs`),
much of it discovered by probing rather than from docs — see
[[facet-newtypes-and-custom-widgets]].

**The inference was wrong.** Method, so it can be repeated in one command:
`git archive HEAD` into a scratch dir, `sed` the one workspace version string
to `0.50.0-rc.7`, then `cargo check --workspace --all-targets` and
`cargo test --workspace`.

Result: **check clean, 359 passed / 0 failed / 5 ignored** — identical to
0.46.5, with **no source changes at all**.

## Why nothing broke — the surface diff, not just the green run

A passing suite alone wouldn't prove much (it could mean the tests miss the
changed corner), so the API was diffed directly out of both crate tarballs:

| depended on | 0.46.5 -> 0.50.0-rc.7 |
|---|---|
| `ScalarType` | 25 variants, identical |
| `Type` / `UserType` / `Def` | 5 / 4 / 12, identical |
| `StructKind` | 4, identical |
| `ReflectError` | 26 variants, identical |
| facet-reflect public fns (`Partial`, `Peek`) | 359 -> 359, **zero added, zero removed** |
| facet-core public fns | 304 -> 312, **purely additive** |

Nothing was *removed* from either crate. `facet-reflect` — the half this
library leans on hardest — is untouched.

**`Shape`'s `PartialEq`/`Hash` are still keyed on `id`, a byte-identical impl**
(it just moved from shape.rs:284 to :328). That is the verified premise
[[widget-registry-idea]] rests on for `HashMap<&'static Shape, ControlType>`,
and it survives 0.50.

## What 0.50 actually is

Growth at the edges, not a rebuild of the core: a new `facet-path` crate, a
taxonomy bridge (`schema_of`/`schemas_of` in a new `taxon_bridge.rs`),
`ReprAffinity` on the shape builder, and **namespaced attributes** —
`#[facet(orm::primary_key)]`, with `Attr::ns()` / `Attr::key()`.

`define_attr_grammar!` still exists in 0.50. That was the mechanism
[[formoxus-control-survey]] records as built, proven, then abandoned. Namespaced
attributes do NOT reopen that decision — what actually won it for `form2!` was
the orphan rule dissolving because the macro expands in the CONSUMING crate
([[facet-form-design-decisions]]), and namespacing is orthogonal to that. NOT
checked, if anyone ever revisits: whether the grammar's own limits moved (the
survey pinned "i64 newtypes yes, i64 struct fields NO").

## The call

**Stay on 0.46.5.** It is the latest stable, and 0.50 offers formoxus nothing
it wants — taking an rc would buy risk for no feature. The point of this file
is that the risk is now *measured* rather than guessed: when 0.50 goes stable,
it is a one-line bump and a test run, not a migration.

Also pinned here because it is easy to misread: **359 is this repo's green
baseline.** [[next-up-two-todos]]'s 476 was the whole apcsp-dioxus workspace
measured before extraction, not formoxus alone.
