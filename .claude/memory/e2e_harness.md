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
  `NEXTEST_TEST_THREADS=1` (was `RUST_TEST_THREADS=1` under `cargo test`).
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

## 2026-11-03: 52 tests, 11 files: `tags` (11) for SETS_PLAN box 10

On `/t/tags` and `/t/tags-pattern`. The draft is found by role and name
(`textbox "Add to Tags"`), items by `.fx-set-item:has-text("…")`. **Scope the
set's Save to `.fx-set-adder`**: a test form's own submit button is `save`,
so `button("Save")` matches two. `a_draft_breaking_its_pattern_blocks_the_submit_yet`
pins a known gap (SETS_PLAN box 14) with a matching draft as its positive
control. A mutation run that restores files by moving a backup leaves stale
wasm: see [[mutation-restore-mtime]].

## 39 tests, 8 files (35/7 until 2026-10-30; 29/6 until 2026-10-24; 24/5 until 2026-10-01)

**2026-10-30: `reorder` (2), for ORDERED_LISTS box 3**, on `/t/reorder`.
A list ROW's input has no accessible name (the list's legend labels the
list), so rows are found by position: `ol.fx-list-rows > li input`, `nth`.
Icon-only buttons are found by `aria-label` through the new
`roles::button`. Mutation-checked: a ↑ that moves nothing fails it.


**2026-10-24: `wire` (2), for issue #13**, on `/t/enum-submit`. That form also
shows `#accepted`, what `Submission::accept` builds from `form.to_wire()` run
IN THE PAGE (no test form has a server fn), and `#submits`, a counter, so
"validated twice" is assertable. Both tests were mutation-checked: they fail
when `ordered_values` reads structure from the store.

**A `just e2e` filter matches test NAMES, not file names.** `just e2e wire`
ran nothing, vacuously green (`2 filtered out`). Select a file with
`just e2e --test wire`: the recipe puts its args before its own `--`, so
`--test` reaches cargo.

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

## playwright-rs 0.15.1: `to_be_focused` / `is_focused` are broken (2026-10-31)

They pass the locator's whole selector to `document.querySelectorAll`, so a
role lookup or any `nth()` fails as "not a valid selector". Use
`e2e::focus::expect_focused(&locator)`, which polls `Locator::evaluate("el =>
el === document.activeElement")`. Reported 2026-10-31 as padamson/playwright-rust#180, fixed by #181 (Todd's fork,
branch `fix-is-focused-playwright-selectors`), MERGED upstream by 2026-11-01.
Not released yet: crates.io's newest was still 0.20.1 (2026-10-17) that day.
Once a release after 0.20.1 is out, bump to it, delete `e2e/src/focus.rs`, and
go back to `expect(…).to_be_focused()`.

## nextest, from 2026-10-31

`just test`/`just e2e`/CI run `cargo nextest run` (plus `cargo test --doc`,
which nextest skips). nextest runs EVERY binary's tests at once, one per CPU:
at 32 parallel browsers each test timed out on `#app-ready` (5 s default
`expect`), because each fetches and compiles the 102 MB debug wasm (release:
4.2 MB). `.config/nextest.toml` caps `package(e2e)` at 8. Measured: wall time is
~10-11 s at 8, 16 or 32 (with a 60 s wait), so the cap costs nothing. Filters
match TEST names: `just e2e reorder` matches `…reorders_the_model`, not the file.
playwright-rs is 0.20.1 (from 0.15.1); browsers via the bundled driver
(`~/.cache/playwright-rust/<ver>/…/node …/package/cli.js install chromium`) or
`npx playwright@1.64.1` once system Node is ≥ 20.

**Seen flaky once (2026-11-03):** `adder::a_picked_song_arrives_as_a_filled_item`
timed out at 30 s as the LAST of 58 in a full `just e2e`, then passed alone and
in two full reruns. Nothing in the change touched lists. If it recurs, suspect
the run's tail (server load), not the adder.

**Seen again (2026-11-07), and now diagnosed:** `required_text::novalidate_lets_formoxus_report_the_empty_field`
and `the_field_is_still_marked_required_under_novalidate` both failed at 30.76 s in
a full `just e2e`, then passed in two reruns by Todd and six more of mine (17-19 s
each; the failed run took 40 s). A hang, not a result: `expect` fails as
`AssertionTimeout` at 5 s, and these were a plain `Timeout` at 30 s. One of them does
nothing but open the page, and the server had logged `[200]` for it, so `goto` is what
timed out: the HTML arrived and `load` never fired. They are adjacent in the file, so
they probably started together. Which subresource stalls is NOT known (the 102 MB
debug wasm is the suspect). Tell them apart with the error's type: a 30 s `Timeout`
points at the harness, an `AssertionTimeout` at the code. The `dx-e2e.*.log` a run
leaves in `/tmp` logs each page's `[200]` with a time, which is how to place a hang.

## 2026-11-09: 70 tests, 13 files: `live` (12) for ERRORS_PLAN 8d

On `/t/live-text`, `live-click`, `live-server-error`, `live-select`, `live-off`,
`live-custom`. Things learned writing them:

1. **`Locator::select_option` does not focus the select.** It fires the events
   only, so `expect_focused` on the select after it fails; a focus check there proves
   nothing anyway.
2. **`ClickOptions::builder().delay(150.0).build()`** holds the button 150 ms between
   press and release: a human-paced click. Even Playwright's instant click is two round
   trips, so a render triggered by the press (a blur) lands before the release.
3. **A click whose press and release land on different elements fires no `click`**
   on the button. How trap 1 was measured: `bounding_box()` before and after the blur,
   and an expando property (`el.__probe = 1`) to prove the element was not replaced.
4. **To count submits a test form cannot see**, wrap the form in
   `div { onsubmit: … }`: `submit` bubbles through dioxus's delegation. `/t/live-click`
   does it, with a control test proving the counter works.
5. **A submit validates EVERY field**, so `.fx-field-error` after one is not unique;
   scope to a field's list with `[id$="-{path}-errors"] .fx-field-error` (the form id
   prefix is random).
6. **Pass e2e extra flags after the recipe's own:** `just e2e --test live --no-capture`
   works; `--run-ignored …` clashes with the recipe's `ignored-only`.

