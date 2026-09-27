# End-to-end browser tests for the examples gallery

Written 2026-09-27. Mirrors the working setup in `~/code/rust/apcsp-dioxus`
(`crates/e2e` + the `just e2e` recipe), which is the reference — read it before
starting. Each step ends with a check that says it's done.

## Why, and what it buys over the suite

`formoxus/tests/suite/` renders to an HTML string, which proves *markup*. It
cannot prove *behaviour*: that a click lands, that a value round-trips through
`onsubmit`, that the browser actually blocks a submit on a `pattern`. `main.rs`'s
own doc comment already says this — "a `#[gtest]` that renders to HTML proves
markup rather than behaviour."

## What is simpler here than in apcsp-dioxus

Most of that recipe's complexity is not needed:

- **No server, no database, no seeding.** No `ENVIRONMENT=test`, no `.env.test`,
  no `SURREAL_URL=mem://`, no `fixtures`. The gallery is a client-side app.
- **No auth**, so no `login`, `submit_login_form` or `choose_role` on the
  session.
- **No routing.** `examples.rs` is deliberately a flat list, not a router — so
  every test is one `goto("/")` and then works within the page. Drop the `path`
  argument from `goto` or keep it for a future router; keeping it costs nothing.

What carries over unchanged: `playwright-rs`, the `#[ignore]` convention, the
`Session` shape, and the build-then-serve-then-poll structure of the recipe.

## The one piece that is NOT optional

**The `#app-ready` marker.** apcsp's `Session::goto` waits for a hidden
`#app-ready` element rendered by a post-mount effect, and its comment says why:
without it, "a submit that triggers a native form GET instead of the Dioxus
`onsubmit` handler." The gallery has submit buttons and the same hydration race,
so it needs the same marker. Gate it on a feature so it never reaches a real
build.

## Steps

**1. The `e2e` crate.**
- `e2e/Cargo.toml`: `publish = false`, deps `anyhow`, `playwright-rs = "0.15.1"`,
  `tokio = { version = "1", features = ["full"] }`.
- Add `"e2e"` to `members` in the root `Cargo.toml` but **NOT** to
  `default-members` — the same treatment `examples` gets, and for the same
  reason: it stays compiled so it cannot rot, while a bare `cargo test` in the
  edit loop skips it. Say so in a comment beside the existing one.
- Done when: `cargo check -p e2e` passes and a bare `cargo test` does not build
  it.

**2. The hydration marker.**
- Add an `e2e-testing` feature to `examples/Cargo.toml`.
- In `examples/src/main.rs`'s `App`, under `#[cfg(feature = "e2e-testing")]`,
  render a hidden `div { id: "app-ready" }` from a post-mount effect — copy
  apcsp's mechanism rather than inventing one.
- Done when: `dx serve -p formoxus-examples --features e2e-testing` serves a page
  containing `id="app-ready"`, and a build WITHOUT the feature does not.

**3. `Session`.** `e2e/src/{lib,config,session}.rs`, cut down from apcsp's:
- `config`: just `base_url()` from `E2E_BASE_URL`, default
  `http://127.0.0.1:8080`. No username or password.
- `session`: `open()`, `goto(path)` waiting on `#app-ready`, `close()`. Nothing
  else yet — add helpers when a second test wants the same three lines.
- Done when: `cargo check -p e2e` passes.

**4. The `just e2e` recipe.** Port apcsp's, dropping the env and seeding parts.
Keep every piece of its error handling — each one is there because something
went wrong:
- build in the **foreground first**, so a compile error fails fast with the real
  message (`dx serve` is a watcher and stays up on error);
- `--force-sequential`, which halves peak memory on a fullstack build;
- serve in the background under `setsid`, and `trap` killing the whole **process
  group** (dx spawns the server binary as a child);
- poll with `curl -sf` (`-f` so a 5xx is never mistaken for ready), bail if the
  server exited, and `grep` the log for `panicked|Build failed` to catch a
  process that stays up while broken;
- finish with `cargo test -p e2e {{args}} -- --ignored`, so `just e2e one_test`
  filters.
- Done when: `just e2e` builds, serves, runs zero tests green, and leaves no
  stray `dx` process (`pgrep -f 'dx serve'` empty afterwards).

**5. The tests.** `e2e/tests/`, one file per example section. Start with what was
just built and cannot be proven by SSR:
- **`radio_group.rs`** — click a radio, submit, the chosen value appears in the
  submitted output; nothing is checked before the first click.
- **`select.rs`** — choose a state, submit, it round-trips.
- **`buttons.rs`** — reset restores the seeded values; submit shows the model.
- **`constraints.rs`** — the one that needs care, see below.
- Done when: `just e2e` is green.

**6. Tidy.** Note `just e2e` in `CLAUDE.md`'s Testing section, which currently
lists three homes and would then have a fourth. Delete this file.
- Done when: `just ci` still passes (it must NOT run `e2e` — that needs a
  browser) and `just e2e` passes separately.

## The trap in step 5

**`examples/src/examples/select.rs` sets `browser_validation: off`**, on purpose,
so the gallery can show formoxus's own error rendering rather than the browser's
unstylable tooltip. So a test asserting "a bad zip blocks submit" will FAIL
there, and it will look like the constraint attributes are broken when they are
not.

Pick one deliberately:
- assert the opposite for that example — a bad zip submits, and formoxus's own
  `.field-error` appears. That tests the more interesting half anyway, and it is
  the half the suite cannot reach;
- or add a third example with `browser_validation: on` and test browser blocking
  there. Also worth having as a gallery entry, since the two behaviours side by
  side are the clearest way to show what the setting does.

## Every example, both ways: `novalidate` on and off

Todd, 2026-09-27: run each form with and without `novalidate` so both paths are
known to work. This belongs here rather than in `tests/suite/`, and the reason is
the point of the whole crate: in SSR the two differ by ONE attribute on the
`<form>`, which `tests/suite/browser_validation.rs` already pins. What differs is
BEHAVIOUR, and only a browser has it:

| | `browser_validation: on` (no `novalidate`) | `off` (`novalidate` present) |
|---|---|---|
| submit with a bad value | browser blocks, native tooltip, `onsubmit` never fires | submits; formoxus renders `.field-error` |
| submit with a required field empty | browser blocks | submits; "This field is required." |
| `pattern` mismatch | browser blocks | formoxus's own message |

So the same example has to be driven twice and asserted DIFFERENTLY — this is not
one test run twice, it is two expectations. That is also the cleanest proof that
`novalidate` really does neutralize the constraint attributes, which is the
assumption behind emitting them ungated.

**How to reach both from one gallery.** Options, in preference order:

1. **A toggle in the gallery** — a checkbox that flips the setting for the
   examples below it. Needs the config cascade's context tier
   (`provide_defaults(Formoxus::new().use_browser_validation(…))`), which is
   already built, so it is a few lines. Best option: it makes the difference
   visible to a human reader too, which is a gallery entry worth having in its
   own right.
2. **Two examples, one each.** Simple, no new mechanism, but duplicates a form
   and the two drift.
3. **`E2E_BASE_URL`-style env var** read by the gallery. Rejected — it makes the
   page depend on how it was launched, which is exactly what a `Session` should
   not have to know.

Note that `examples/src/examples/select.rs` currently sets
`browser_validation: off` at the form level, and a per-form setting BEATS the
context default — so a gallery-wide toggle will not move it. Either drop the
per-form setting from that example, or keep it as the deliberate "this one is
always off" case and test it that way. Decide before writing the toggle, or the
test will look broken.

## Worth knowing before writing selectors

apcsp's `session.rs` carries this comment:

> formoxus doesn't emit `placeholder` yet — matching on the field label instead.
> Not exact: the label also carries the `" *"` required-marker span, so the
> accessible name is "Username *". Switch back to `get_by_placeholder` once
> formoxus supports it.

So a real consumer is already working around the missing pass-through attributes
— the feature in `CONSTRAINT_ATTRS_PLAN.md`. Two consequences: `get_by_label`
has to include the `" *"` for a required field, and this is a concrete argument
for finishing author-supplied attributes ([[shrink-fieldprops-idea]] has the
design).
