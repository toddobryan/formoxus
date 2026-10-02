---
name: e2e-harness
description: "The playwright-rs browser tests built 2026-09-27 — `just e2e`, the e2e crate, the /t/<slug> test forms. Records the get_by_label-vs-get_by_role distinction that decides whether `aria-hidden` on the required marker does anything, and that Playwright normalizes whitespace in text assertions"
metadata:
  type: project
---

Built 2026-09-27/28, modelled on `~/code/rust/apcsp-dioxus/crates/e2e` + its
`just e2e`. `E2E_PLAN.md` is gone — it was finished and deleted; `CLAUDE.md`'s
Testing section now carries the summary. **24 tests green**, the whole suite in
about 7s once built.

- `e2e/` — workspace member, NOT a default member, so `--workspace` compiles it
  (selectors cannot rot silently) and a bare `cargo test` skips it. Every test is
  `#[ignore]`d, so even `--workspace` runs none.
- `Session::open(path)` launches, navigates and waits for `#app-ready` in one
  call. One page per test, because each test form has its own URL — apcsp needs a
  separate `goto` only because it has to log in first.
- `examples/src/test_forms.rs` — `/t/<slug>`, one dynamic route with a match.
- `just e2e` — foreground build, background `setsid dx serve`, `trap` killing the
  process GROUP, `curl -sf` poll, log grep for `panicked|Build failed`.
  Deliberately NOT in `just ci`, which has no browser.
- `just e2e-watch [filter]` — the same, in a VISIBLE window. Three things have to
  change together or watching does not work: headed (`E2E_HEADED=1`), slowed
  (`E2E_SLOW_MO`, default 400ms, Playwright's own `slowMo` so it paces ACTIONS),
  and SERIAL, or every test opens its own window at once. Serial comes from
  `RUST_TEST_THREADS=1`, not `-- --test-threads=1`, because `just e2e` puts its
  `{{args}}` BEFORE its own `--` and cargo would read the flag itself.
  Slow-mo multiplies per action, so pass a filter and watch ONE test.

## `get_by_label` vs `get_by_role`: the one that cost time

**`get_by_label` matches the `<label>`'s TEXT. `get_by_role` matches the computed
ACCESSIBLE NAME.** They differ exactly where `aria-hidden` is involved, which is
the entire point of the `aria-hidden` on formoxus's required marker:

- `get_by_label("Name", exact)` **times out** — the label's text is still
  `"Name *"`, because the marker span is inside it and `aria-hidden` does not
  remove text from a label's text content.
- `get_by_role(Textbox, name: "Name", exact: true)` **matches**, because the
  accname algorithm skips `aria-hidden` subtrees.

So an exact label match is the wrong tool, and reaching for it makes
`aria-hidden` look broken when it is working. Use `get_by_role` with a name. It
is also the only way to VERIFY the accessible name from a test —
`the_required_marker_is_absent_from_the_accessible_name` asserts both halves: the
role-and-name match proves the asterisk is not in the name, and a
`get_by_label("Name *")` match proves it is still in the DOM and visible.

This corrects an earlier claim that `aria-hidden` would make exact
`get_by_label` work; it does not, and apcsp's workaround comment about the name
being `"Username *"` was right about `get_by_label` all along.

## Playwright normalizes whitespace in text assertions

`{:#?}` in the `#submitted` block renders multi-line in the DOM, but
`to_contain_text` and the failure message collapse it —
`OneField { name: "Ada", }` on one line. So pretty-printing helps a HUMAN reading
the page; it does NOT make an assertion match a whole line the way I claimed when
recommending it. Substring assertions behave the same either way.

## 29 tests, 6 files (was 24/5 until 2026-10-01)

`required_text` (5), `constraints` (9: pattern incl. anchoring end-to-end,
lengths, bounds), `widgets` (8: checkbox, select, radio-group, textarea),
`buttons` (2: reset restores the SEEDED value, submit records the edit),
`must_agree` (5, added 2026-10-01 for issue #6: browser blocks an unticked
`required: true` box, formoxus rejects it under novalidate both untouched and
ticked-then-unticked, marker out of the accessible name).

Two things worth knowing about writing more:

- **A `type="number"` input has role `spinbutton`, not `textbox`.** The `bounds`
  form is the only one that uses it, and a `textbox` lookup there fails in a way
  that looks like a missing label. `e2e/src/roles.rs` has one helper per role so
  the mistake is not available.
- **Most of these assert `#submitted` is EMPTY** to mean "the browser blocked the
  submit", which is vacuous if the click never landed. What rescues them is that
  each such form ALSO has a positive test using the same button, so the button is
  known to work. Verified by mutation: answering the select in
  `an_unanswered_required_select_is_blocked` makes it fail.

## Small API notes for `playwright-rs` 0.15.1

- `Locator::inner_text()` takes NO argument, unlike most of the other methods
  that take an `Option<...>` of options.
- `GetByRoleOptions` and `LaunchOptions` are both `#[non_exhaustive]`, so build
  them from `default()` and assign fields; a struct literal will not compile.
- Negation is `expect(x).not().to_have_text(…)`, not a `not_to_*` method.
- `maxlength` is enforced as you TYPE, so `fill` bypasses it — it sets the value
  directly. Use `press_sequentially` to test it.
