# Issue #13: structure is lost on the wire

A working checklist for [#13](https://github.com/toddobryan/formoxus/issues/13).
It began as "an enum's chosen variant is lost on the wire" and, by Todd's
decision on 2026-10-24, also covers **list rows**, which are lost the same
way. Claude keeps this file current when asked.

> **Status, 2026-10-24, evening.** The SUITE IS GREEN: 412 passing, 0
> failing, and every one of step 1's RED tests passed with no edit to the
> test. Done: 1–24, 19a, 20a (15 skipped by decision). `just ci` and the
> full `just e2e` are GREEN. Open: 25 (Todd, apcsp-dioxus) and 26 (close #13). Line numbers are as of `69d94e2` plus today's
> uncommitted work, so they will drift; ask for a refresh rather than
> trusting an old one.

Who does what: Todd writes the library code; Claude writes and updates the
tests, the e2e pair, docs that are only docs, and memory.

---

## The one idea

The server rebuilds every request from `empty_form` (`submission.rs:56`),
where every enum is `Unchosen` and every list has no rows. A leaf on the wire
can only land in a node that already exists, so **anything that decides which
nodes exist has to cross as a value of its own**, at its container's own path:

| Container | Wire entry | Example |
|---|---|---|
| enum (`VariantSet`) | the variant name, `""` when unchosen | `title` = `Dr`, `shape` = `Circle` |
| list (`ListSet`) | its row keys, in order (Q2) | `tags` = `#0,#1` |

Call these **structure values**, to set them apart from **leaf values**.
Leaf values live in the store; structure values live only in the state.

## Decided 2026-10-24

- **A. The state owns structure.** `VariantSelect` reads the choice from the
  state and reports a change as an `Edit`; nothing writes it to the store,
  and nothing should start to. So structure values go on the wire and never
  into the store, and everything that builds a wire payload takes them from
  the state. (Matches `FIELDS_PLAN.md` V1(c), which rejected writing a
  variant into the store for the same reason.)
- **B. Distributing what is already there is a no-op.** The same variant
  name, or the same row keys, leaves the node untouched, so a client
  `validate` never rebuilds anything blank.
- **C. An unknown variant name is remembered in the state, not as a pushed
  error.** `VariantSet::validate` clears `errors` (`variant_set.rs:257`), and
  `OptionNode::validate` clears an absent inner's errors
  (`option_node.rs:50–56`), so a pushed error vanishes, and an
  `Option<Title>` given `Sir` is ACCEPTED as `None`. Remembering the name
  lets `validate` re-derive the error. This is not only for forged requests:
  a tab still running the old wasm after a deploy that renamed a variant
  sends one honestly.
- **D. `absorb` does not rebuild structure** (its doc, `form.rs:552–554`). A
  structure value in a reply is ignored, and kept out of the store.

- **Q1. ONE walk collects both** (Todd: "I really don't like the idea of
  two functions doing what is basically the same thing"). `Node::collect_values`
  (`nodes.rs:41`) emits every entry tagged with its kind, leaf or structure,
  and the callers that need only leaves filter on the tag. Rejected: a
  second walk, `collect_structure`, beside it. It would have left
  `collect_values` and its tests untouched, but at the cost of two functions
  doing the same traversal. The cost taken instead: the store seeding
  filters, and tests that pin exact `collect_values` output over enums and
  lists change (box 22).
- **Q2. A list's rows cross as their keys, in order, at the list's own
  path:** `tags` = `#0,#1`, the list's version of a variant name. It is the
  only spelling that covers every case: a row with NO leaves (an empty inner
  `Vec`, a `Vec<Option<Title>>` holding `None`), and row ORDER, which keys
  alone cannot give, since `AddRow { before }` (`list_set.rs:216`) puts a
  new key in front of older ones. Rejected: inferring rows from the leaf
  paths' `#n` segments, which loses a leafless row.
- **Q3. A malformed list entry** (`#x`, a repeated key) is a field error at
  the list's path, never a panic, like an unknown variant name. No cap on
  the row count: each key costs at least three bytes of request, so rows are
  linear in the body size the server already limits.

- **Q4. The tag is an enum, not a struct with a kind field** (Todd):
  `Node::collect_values(&self, prefix, out: &mut Vec<Value>)` with
  `enum Value { Leaf { path, raw }, Structure { path, raw } }`, plus `path()`
  and `raw()` accessors (or `Value::Leaf { path, raw } | Value::Structure
  { path, raw }` or-patterns) for callers that want both kinds alike.
  Rejected: `Collected { path, raw, kind: ValueKind }` (Claude's first
  proposal), a type-code field. Named fields rather than `Leaf(path, raw)`,
  because two positional `String`s swap silently (Claude's suggestion; the
  name became `RawValue`, fields `path` and `data`, since `Field::Value`
  already names the typed value; `into_pair` serves callers that want both
  kinds alike). The enum also leaves room for
  the two kinds' data to diverge, e.g. `Structure` splitting into a variant
  name and a typed row-key list.
- **Q5. `FormState::collect_values` returns everything, as a
  `ValuesByPath`.** That IS the wire, so `WireForm::from_model` and
  `Submission::into_wire` call it unchanged, and the tag stays out of the
  prelude. (It is still `pub`, as `formoxus::nodes::Value`: `Node` is a `pub`
  trait, and a `pub` trait method cannot take a private type.)
  Leaves-only is a `pub(crate)` filter for the store seeding. Rejected:
  returning the tagged entries, which would make the tag public for the sake
  of three internal callers.

---

## Step 1. RED tests (Claude)

- [x] **1.** `Title { Mr, Miss, Mrs, Ms, Dr, Rev }` in
  `tests/suite/models.rs`.

- [x] **2.** `tests/suite/submissions.rs:277–551`, "Enums: the choice has to
  cross the wire" and "Lists". Unit, struct and tuple variants round-trip;
  the spelling `title` = `Ms` / `shape` = `Circle` (388); missing (410) and
  `""` (421) still "must choose"; an unknown name names itself (437) and is
  never `None` behind an `Option` (458); `Option<Title>` both ways (468,
  477); an enum inside a variant, at `layer.$Painted.fill` (488); each row's
  choice (513); `Vec<String>` (545).

- [x] **3.** `tests/suite/wire.rs`: `Contact` gained `title: Title` (25), so
  the two hand-built round trips (316, 344) fail the way apcsp-dioxus does.
  "A variant chosen in the browser" (382–529): `Harness` tests that pick in
  the real `<select>` and read `to_wire` (432, 443) and `validate` (459), and
  `absorb` with the same choice (480) and another one (504). These are the
  tests that fail under the naive fix, where the store holds a missing or
  stale choice.

## Step 2. Enums: collect and distribute the choice (Todd)

- [x] **4.** `VariantChoice` (`variant_set.rs:26`) gains
  `Unknown(String)`, per C. `is_present` (377) counts it as present, so
  `OptionNode` validates it rather than clearing it. `child_prefix` (56),
  `chosen` (151) and `write_value_into` treat it as having no nodes.

- [x] **5a.** `enum RawValue` (`nodes.rs:225`), per Q4, and `Node::collect_values` (`nodes.rs:41`)
  pushes it. Five impls: `field_node.rs:207` emits `Leaf`;
  `field_set.rs:73`, `option_node.rs:34` pass through; `list_set.rs:144`
  and `variant_set.rs:316` emit `Structure` for themselves (5, 9).

- [x] **5.** Collect the choice, per Q1: a `Structure` entry, the variant name at
  `qualify(prefix, &self.name)`, **`""` when unchosen** (so a
  `Vec<Option<Title>>` row holding `None` still has an entry, Q2), then
  recurse under `child_prefix`. `Unknown` emits its bad name, so a reply
  shows the user what was rejected.

- [x] **6.** `Node::distribute_values` (`nodes.rs:45`) gains `specs:
  &SpecsByPath`, since building a variant's nodes needs them (`set_variant`,
  `variant_set.rs:91`). Five impls: `field_node.rs:211`,
  `field_set.rs:87`, `option_node.rs:42`, `list_set.rs:151`,
  `variant_set.rs:325`; and `FormState::distribute_values` (`state.rs:178`)
  passes its own.

- [x] **7.** `VariantSet::distribute_values` (325), the rule:
  **absent** → leave the choice alone (B; a leaves-only map must never
  unchoose); **the current name** → no-op (B); **`""`** → `Unchosen`;
  **another real name** → choose it through `set_variant` (91), the code
  `Edit::ChooseVariant` uses (285), so nodes are built the way the UI
  builds them; **an unknown name** → `Unknown(name)`. Then recurse into the
  children with the same values.

- [x] **8.** `VariantSet::validate` (256): `Unknown(name)` pushes the
  message `lookup_variant` builds today (128–141: names the bad choice and
  the real ones). Probably split that message out of `lookup_variant` so the
  two cannot drift.

## Step 3. Lists: collect and distribute the rows (Todd)

- [x] **9.** Collect the row keys as a `Structure` entry at the list's own
  path, per Q2 (`#0,#1`; `""` for no rows), then recurse as now
  (`list_set.rs:144`).

- [x] **10.** (Claude wrote it, 2026-10-24, at Todd's request: a new
  `rejected_keys: Option<String>` field (the list's `Unknown`), `blank_row`
  shared with `add_row`, `set_rows`, and `parse_row_keys` with unit tests.
  Also fixed: `collect_errors`/`distribute_errors` never reported the LIST'S
  OWN errors, so a rejected entry came back as an `Err` with no message.)
  `ListSet::distribute_values` (151): **absent** → leave the rows
  alone (B); **present** → make `rows` exactly the listed keys in that
  order, reusing an existing row by key and building a missing one blank the
  way `add_row` does (40–66), but under the LISTED key rather than
  `next_key`; then `next_key` = largest key + 1. Malformed → error at the
  list's path (Q3). Then recurse.

- [x] **11.** (Verified: `optional_containers.rs`'s list tests pass
  unchanged.) `Option<Vec<T>>`: an empty list already collapses to absent
  (`optional_containers.rs:216`), so `""` and absent agree here. Check that
  the new entry does not make an empty optional list read as present.

## Step 4. The wire takes structure from the state (Todd)

- [x] **12.** (DONE 2026-10-24: `.map(RawValue::into_pair)`, and the doc
  now describes both kinds of entry and says the store takes leaves only.) `FormState::collect_values` (`state.rs:142`) per Q5. Under
  (a) the payload builders need no change: `WireForm::from_model`
  (`wire.rs:149`), `Submission::into_wire` and `into_wire_with_errors`
  (`submission.rs:130`, `136`) already call it. Its doc ("Every leaf
  input…", 136–141) changes.

- [x] **13.** `ordered_values` (`state.rs:165`), used by `Form::to_wire`
  (`form.rs:528`) and `Form::validate` (603–609): **the tag picks the
  source**, a `Leaf`'s value from the store as now, a `Structure`'s from
  the entry itself, which is the state. Its doc's "keys from the schema,
  values from the store" gains the exception.

- [x] **14.** Store seeding filters to `Leaf` (`form.rs:218`, `584`, `700`),
  through the `pub(crate)` leaves-only filter Q5(a) adds.

- [x] **15.** `Form::absorb` (`form.rs:559`), per D: skip a path the
  state's own walk tags `Structure` when writing to the store.
  **Found 2026-10-24 after 13: the behaviour already holds without it.**
  `absorbing_a_reply_with_another_title_leaves_the_choice_alone` passes:
  `absorb` writes the reply's `title` into the store, but since box 13 no
  reader takes a structure path from the store, so the key just sits there
  unread, like the stale keys an undone variant choice leaves. A path is
  always a leaf or always structure for a given model, so the key can never
  be mistaken for a leaf. What 15 would still buy is tidiness: no unread key.
  **SKIPPED (Todd):** a key that turns up where one should have been read is
  easier to debug present than filtered away. `absorb`'s doc now says so.

## Step 5. Docs that now lie (Todd, or Claude when asked)

- [x] **16.** `VariantSet`'s doc (`variant_set.rs:33–41`): "The choice
  itself is NOT a leaf — it never appears in a path or a submitted value."
  It is still not a leaf; it IS now a submitted value.

- [x] **17.** `Form::state_signal`'s doc (`form.rs:228–232`): "a variant
  choice lives nowhere but here." Still true of the store, no longer of the
  wire.

- [x] **18.** `crate::wire`'s module doc (`wire.rs:1–40`) and
  `Submission::accept`'s (`submission.rs:44–55`): say that structure
  crosses, and how it is spelled, since a browser-free client has to send it.

## Step 6. Tests after the fix (Claude)

- [x] **19.** Step 1's 16 go green with no edits beyond any spelling Q2
  settles. If one needs more than that, raise it rather than changing the
  test.

- [x] **19a.** (Added and DONE 2026-10-24, from Todd's "how did we miss so
  many bugs?") The PROPERTY, not just examples of it: every model the suite
  round-trips in memory also crosses the wire. `across_the_wire` moved from
  `submissions.rs` to the suite root (`tests/suite.rs`), and all 34 sites that
  assert `form.validate() == Some(model)` now also assert
  `across_the_wire(&model) == Ok(model)`. All 34 pass on today's tree; run
  against `69d94e2` (a scratch worktree), they fail across `enums.rs`,
  `vecs.rs` and `optional_containers.rs`, so they would have caught #13. The
  in-memory check alone "holds by construction" (`roundtrip.rs`'s header),
  which is how the bug stayed green. Convention from here: a new in-memory
  round trip gets the wire line beside it.

- [x] **20.** The list spelling is already pinned, by box 22's updates
  (`vecs.rs` `rows_are_named_by_key`, and
  `inserting_at_the_front_does_not_move_the_rows_below_it`, which shows
  `answers` = `#2,#0,#1`). After box 10, swap
  `leaves_then_apply_is_an_identity_round_trip`'s reload to `empty_form`,
  as its comment asks, and recheck `create_mode_yields_no_rows_yet`. (DONE
  with box 10: swapped, and `create_mode_yields_no_rows_yet` still passes,
  since its map has no `answers` entry. Also added in `submissions.rs`:
  rows rebuilt in the entry's order, `#2,#0`; a malformed entry is a field
  error at the list's path.) Still to add: a row with no leaves (`Vec<Vec<String>>` holding `[[]]`); a
  `Vec<Option<Title>>` holding `None`; order after `AddRow { before: 0 }`;
  rows added and removed in the browser reach `to_wire` (`Harness`); a
  malformed key is a field error (Q3).
  (ALL DONE 2026-10-24. `submissions.rs`: `a_row_with_no_leaves_survives_the_wire`
  (`[[], ["a"]]`, `rows.#0` = `""`), `a_none_row_survives_the_wire`,
  `a_row_inserted_at_the_front_arrives_at_the_front`, and
  `a_missing_row_list_is_an_empty_list`, a CHARACTERIZATION of the last
  silent case that says what must not be built on it. `vecs.rs`:
  `rows_added_and_removed_in_the_browser_reach_the_server`, clicks to
  `to_wire` to `Submission::accept`; checked to fail when `ordered_values`
  reads structure from the store.)

- [x] **20a.** (DONE 2026-10-24, Claude at Todd's request: the fieldset gets
  `id`, `aria-describedby` to a `FieldErrors` list right after the legend,
  and `fx-invalid` (a new invalid class on `FixedKey::ListSet`); comments
  cleaned up; snapshots regenerated, the only change being the new `id`.
  Tests in `vecs.rs`: a list error renders and describes the list; a bad
  cell does not mark it.) `ListSet::render` (`list_set.rs:101`) does not show the list's
  own errors, so a server's verdict at a list path (now delivered by
  `distribute_errors`) and a rejected row-key entry are invisible beside the
  list. A legend-level error list, like `VariantSet`'s beside its select.
  Todd's call whether that is #13 or its own issue.

- [x] **21.** (DONE 2026-10-24 as an e2e test, at Todd's request:
  `e2e/tests/wire.rs` `validating_twice_keeps_the_choice_and_what_was_typed`,
  on `/t/enum-submit`: Circle then Square, each submitted twice; the choice,
  the typed value and the server's model all hold.) B, on the client: switch a variant, type into its field, run
  `validate` twice; the typed value and the choice both survive.

- [x] **22.** (DONE 2026-10-24: all 15, in `enums.rs`, `vecs.rs`,
  `optional_containers.rs`; each gained the list's or enum's own entry, and
  the unchosen ones also assert the `""` value. After box 7, a 16th:
  `enums.rs` `switching_variants_does_not_inherit_a_same_named_field` typed
  into EVERY `collect_values` path, the choice included, and snapshotted the
  choice as its "store"; it now filters to the variant's leaves.) Update the tests that pin exact `collect_values` output over an
  enum or a list, which now gain `Structure` entries. 34 calls in 8 files
  (`enums.rs` 12, `vecs.rs` 7, `optional_containers.rs` 5, `forms.rs` 3,
  `empty_strings.rs`, `newtypes.rs`, `widgets.rs` 2 each, `specs.rs` 1);
  only the ones over enums and lists move. Known: `enums.rs:525–535` and
  `587`, `optional_containers.rs:316`, whose "fieldless variants have no
  leaves" becomes "…contribute only their choice". Each one READ before it
  is changed: an assertion that was really about leaves may want the
  leaves-only filter rather than a longer list.

- [x] **23.** (DONE 2026-10-24: `/t/enum-submit` in `examples/src/test_forms.rs`
  shows `#submitted`, `#accepted` (`Submission::accept` on `to_wire()`, in
  the page) and `#submits`; test `a_variant_chosen_in_the_browser_reaches_the_server`.
  Both e2e tests FAIL with `ordered_values` reading structure from the store,
  checked by mutation.) e2e pair: `/t/enum-submit` in `examples/src/test_forms.rs`,
  whose submit handler runs `to_wire` then `Submission::accept` in the page
  (no server fn exists in the test forms) and shows the result; and its test
  in `e2e/`. Pick a variant, fill its field, submit, see the model. The
  suite cannot prove the select's real `onchange` reaches the wire; only a
  browser can.

## Step 7. Close it out

- [x] **24.** `just ci` green. (2026-10-24, together with the full `just e2e`:
  35 e2e tests, all passing.)

- [ ] **25.** (Todd, in apcsp-dioxus) `create_question_duplicate_is_a_form_error_on_name`
  and `create_question_succeeds_for_their_own_course`
  (`crates/web/src/server.rs`) pass. Not edited from a formoxus session.

- [ ] **26.** (Claude) Close #13 with a comment naming the commit; update
  memory (`next_session.md`, and `facet_form_design_decisions.md`'s "create
  mode yields zero rows" note, which this supersedes for the wire).
