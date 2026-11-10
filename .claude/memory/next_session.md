---
name: next-session
description: "HANDOFF. 2026-11-10: box 3/4 WIP pushed on `paths`, NOT COMPILING (173 errors, the to-do list); open: fold spec paths into ModelPath (A/B/C). 2026-11-09 end: #20 fixed+closed; next PATHS_PLAN box 3 (NodePath, branch `paths`). 2026-11-09 evening: ERRORS_PLAN done, apcsp follows (81e0c21) and is green, #7 closed; next ONE plan for #20+#21, then missing widgets, then #22+#23. 2026-11-09 later: errors merged to main (ca0e731), branch deleted; next apcsp, #7 comment+close (ask), then #22. 2026-11-09 overnight: boxes 7-9 done by Claude, committed per box (not pushed), ci+e2e green; trap 1 left alone (#24); box 10 = Todd merges. 2026-11-09: 6f+6g committed and pushed (05178a1). 2026-11-08 late night: 6e committed, 6f next. 2026-11-08 night: 6c+6d committed, Todd on 6e; #22 filed. 2026-11-08 evening: 6a+6b committed, Todd on 6c. 2026-11-08 later: box 6 expanded into 6a-6g in ERRORS_PLAN (uncommitted), incl. 6f aria-invalid=\"false\" (#7), Todd builds it. 2026-11-08: box 5 (live check) committed; next box 6, the triggers (Todd). 2026-11-07 LATER: box 3 + 4a/4b committed (8e7083a, 96100d2), 4c committed; next 4d (Todd), then 5 (live check, open design question). Earlier 2026-11-07: 3b DONE, next box 4. 2026-11-05 END OF DAY: box 2 DONE (5c09320), 3a done, 3b half done and pushed with 13 tests failing; see the top of ERRORS_PLAN box 3b for the three fixes. Earlier 2026-11-05: box 2 IN PROGRESS on `errors`, pushed NOT COMPILING on purpose (55 errors); box 2 REWRITTEN 2026-11-05 as a work order 2b-2j in ERRORS_PLAN.md, proved in a throwaway copy (864 tests, 58 e2e, clippy, docs). Before that, 2026-11-04: branch `errors`, formoxus/ERRORS_PLAN.md written (uncommitted), errors LEAVE the nodes into one flat map + ErrorsStore; Todd builds the runtime (boxes 2-6), E/F/G decided; then range/checkbox_multiple/select_multiple/file, then docs. #14 committed 2db98c1. Before that: 2026-11-03 #16 a73bec3, #4 ef1c5df (both closed); #14 min_items/max_items BUILT; next ERRORS_PLAN.md (ErrorsStore + live validation), then docs. Before that: #16 trim BUILT (text trims where raw is READ; `no_trim` FLAG, password never trims, numbers always). Plan after: #4, #14 (min_items/max_items?), then ErrorsStore + live validation (Todd wants BOTH before publish; ERRORS_PLAN.md to write), then docs; facet 0.50 waits for stable. Before that: C4 per-field validators BUILT (validator: on any field, Vec<ValidationMessage>, structs now show their own errors), UNCOMMITTED, ci 1146 + e2e 58 green; check params `&str`/`&[T]`/by value; next MVP A3 facet version. Before that: C5 help_text committed 5ec7cdd. Before that: path!(M.set[= v]) + choice labels on set items COMMITTED (1cfd158, beea08a); unit-enum items now label-cased (Node::shown -> Shown, SetMessage::text(case)), UNCOMMITTED, ci 1108 + e2e 58 green. Before that: ORDERED_LISTS box 6 DONE (no code), box 7 DONE incl. path!(…[i]) so a validator names one item (indexed_paths.md), #17 FIXED (variant select name); COMMITTED, ci 1086 + e2e 58 + apcsp green; next apcsp box 8. Before that: `sets` MERGED into main and pushed (42e4de3), all other branches deleted; apcsp renames + list-item layout fix COMMITTED there (4141a4d, not pushed); next ORDERED_LISTS 6-8. Before that: box 12 (merge prep) DONE. Before that: box 13 (4 gallery examples) + the seeded-textarea fix COMMITTED; next box 12 (merge prep), then apcsp after the merge. Before that: box 14 (draft constraints off its input, checked on Add) built, UNCOMMITTED, ci 1054 + e2e 53 green; next box 13 (galleries), then 12. Before that: box 11 (rows -> items) built by Claude and committed, just ci 1051 + e2e 52 green; list labels now Remove item N / Move item N (Todd). Before that: box 10 (e2e /t/tags, 11 tests, suite 52/52) built by Claude and committed; the set's Save became Replace (aria-label: Replace plus the item's text); gallery DEFERRED to box 13 (set + list together); pattern gap CONFIRMED, box 14. Next: box 11 (rows -> items). Before that: box 9 (display + `item:` item views) built by Claude and committed, just ci green 1050; next box 10. Before that: box 8 (Edit through the draft) built by Claude, decisions accepted, committed; just ci green 1036. Before that: box 6 committed d3d4852; box 7 (set draft adder) built by Claude, committed 213f67b, just ci green 1017; next box 8. Earlier 2026-11-03: box 4 committed 7a31548; box 5 expanded + decided (ItemHandler, call with_set first, J open pending 5p timings); Todd on 5a-5e. Earlier 2026-11-03: with_sort_by + order tests A/B DONE. 2026-11-01 LATER: 2d/2e (SetNode, Todd) + 2g (tests/suite/sets.rs) + 2f1–2f3 DONE, uncommitted; next box 4 sorting; 2f4 waits on box 5 with_set. Earlier 2026-11-01: on `sets`, 2c (Items, nodes/items.rs) + 3a/3c DONE, every list <ol>+↑/↓, all green incl. e2e; NEXT Todd 2d (SetNode), edit_within deferred to it. 2026-10-31 LATER: allow_reordering REPLACED by lists vs sets, on branch `sets`, plan formoxus/SETS_PLAN.md; box 2 is now renames (ListSet->ListNode, VariantSet->VariantNode, FixedKey/class renames) + nodes/rows.rs + SetNode; box 3 is every-list-ordered, 3b (macro side) done, 10 vecs tests red until Todd's 3a. Earlier 2026-10-31: ORDERED_LISTS box 5 (adder) committed 314bcd4. 2026-10-31: box 4 (filled rows, Form::edit) ALL DONE, uncommitted, awaiting go. 2026-10-30: ORDERED_LISTS box 2 DONE (ec05880); box 3 DONE ebe4437 (↑/↓ MoveRowButtons, Lucide SVG icons, e2e 37/37, just ci green); box 4 (Todd, add-with-value) next. 2026-10-29: formoxus/ORDERED_LISTS_PLAN.md for apcsp's assignment form; decisions A–G made, box 1 (MoveRow) DONE, box 2 (allow_reordering through form!) next, Todd to pick his steps 2a–2g. 2026-10-25: #15 ($Enum::Variant paths in form!/path!, nested blocks, runtime path check) BUILT, uncommitted, awaiting Todd's review. 2026-10-24: issue #13 fixed (4dcf0fb), #14 queued. Rewritten 2026-10-23 when Todd paused formoxus to go back to apcsp-dioxus: where the repo is, what is deferred and why, the first thing to do when using it, and how Todd likes to work. Read this first; older dated entries below are history"
metadata:
  type: project
---

## ▶ 2026-11-10: boxes 3–4 IN PROGRESS on `paths`, pushed NOT COMPILING on purpose (173 errors)

Todd's WIP, committed so he can carry on at school: `NodePath` in `path.rs` (`new`, `root`,
`as_str`, `join`/`join_item`/`join_variant`, `spec_key`, `owns`, Display, `ITEM_SIGIL` moved
there), and the Node trait, `RawValue`, the store aliases, `ErrorsByPath`, `Edit`,
`FieldHandle` and the build walk switched from `&str`/`String` to `NodePath`. The compiler
errors are the to-do list (mostly E0053, impls not yet following the trait). Spec paths
(`SpecsByPath`, `FormSpec` builders, `spec_key_shape`) were rightly left as strings.

Review notes given to Todd, not yet acted on. Four places give a MODEL path the `NodePath`
type, which is trap 6: `ModelPath::to_node_path` (used by `Form::write`, form.rs ~739);
`resolve_indices(&NodePath)`, which should take the `ModelPath`'s string and return
`Option<NodePath>`; `key_for`/`keyed_or_as_written`; and `FormState::push_field_error`,
which takes `&NodePath` but should take `&ModelPath<T>` (decision F). Smaller ones:
`draft_values` still `HashMap<String, String>`, `absent_owner` returns `Option<String>`,
no `Borrow<str>` yet, `messages_at` builds a `NodePath` just to look one up.

OPEN, awaiting Todd. Todd asked whether spec paths and live paths could be one type. Claude
recommended the other merge: fold spec paths into **`ModelPath`**, not into `NodePath`.
`path!(Quiz.answers[].text)` already yields `"answers[].text"`, the same parser and key
`form!` emits, and both name a place in the TYPE. A live path names one node and holds `#5`.
Putting pattern and instance in one type brings back #21's bug class. Result: two kinds,
`ModelPath<T>` (dev-written, compile-checked) and `NodePath` (runtime). Cost: `FormSpec`
builders take `&ModelPath<T>`, `form!` emits `__new(…)` keys (Claude's macro work), ~86
hand-written builder calls become `&path!(M.x)`, and `SpecsByPath` keeps its `String` keys.
Questions put to Todd:
- **A.** Adopt it, as new boxes after the `NodePath` work on the same branch?
- **B.** A builder given `[i]`/`[= v]`: (i) a clear panic (Claude leans this way, macro-first) or
  (ii) a separate type from `path!`?
- **C.** Still open from before: `owns` vs `is_within`, and whether it is true on the root.

Box 3's inline `NodePath` tests (Claude's) wait for those answers. apcsp still has an
uncommitted memory note to add a `rust-toolchain.toml`.

## ▶ 2026-11-09, end: #20 FIXED and CLOSED (boxes 1–2 committed on `main`, not pushed); next box 3 on branch `paths`

Todd's `Items::is_present` fix + Claude's seven tests, `just ci` green. Box 3 (Todd): branch
`paths`, the `NodePath` type in `path.rs` with nothing using it yet; Claude writes its inline
tests. Review note left with Todd: the fix's doc comment says what, not why.

## ▶ 2026-11-09, latest: PATHS_PLAN decisions A–G ALL taken as recommended; box 1 DONE; box 2 is Todd's

Box 1: seven #20 tests at the end of `tests/suite/submissions.rs`, five red until Todd's
one-line `Items::is_present` fix (box 2, on `main`); verified with the fix applied then
reverted (751/751). Todd asked about trap 4 (`?` in paths): answered in the plan; `?` is
source-only, every string form drops it. Nothing committed (see the plan's file list).

## ▶ 2026-11-09, late: `formoxus/PATHS_PLAN.md` written (uncommitted), for #20 + #21

Boxes 1–2 are #20 on `main` first (Claude's failing tests, then Todd's one-line
`Items::is_present` fix); 3–8 are #21 on a branch `paths` as one radical change. Decisions
A–G await Todd: name `NodePath` (rec), `formoxus::path` not re-exported, owned only,
`join`/`join_item`/`join_variant`, `NodePath::new` and no `From<&str>`,
`FormState::push_field_error` takes `&ModelPath<T>` (its live-string form goes), #20 first.
Same evening: `path!`'s dead hand-written `ModelPath` link deleted from `formoxus-macros`
(the prelude's summary now opens "Emits a witness borrow…", noted in the checklist's
item 6, which is the dead-link scan).

## ▶ 2026-11-09, evening: ERRORS_PLAN DONE; apcsp green; #7 closed

Box 10 ticked. apcsp followed the merge in its `81e0c21` (pushed): the Markdown widget on
`FieldHandle` (`current_attrs`, `on_input`/`on_blur`) and `escape_text`, the `&path!(…)`
call sites, `ModelPath`, plus its own `just ci` (fmt, clippy x3, test, test-http, docs).
Todd: builds and passes all tests. #7: box 9's comment posted, closed (Todd, 20:33Z).
NEXT (Todd, later that evening): ONE plan for #20 + #21 (Claude drafts), then the
missing widgets, then #22 + #23; order in [mvp_scope.md](mvp_scope.md). Same evening:
private_intra_doc_links RESOLVED, the public `docs` run dropped; apcsp got a
memory note to add a `rust-toolchain.toml`.

## ▶ 2026-11-09, later: `errors` MERGED to main (fast-forward, ca0e731), pushed, branch deleted

ERRORS_PLAN is done except apcsp (box 10's recounted list) and posting the #7 comment
(drafted in box 9) and closing #7, which waits on Todd's go-ahead. Next: apcsp, then #22
(ideally with #21 and #23).

**Also, back from apcsp (2026-11-09):** look at
[private_intra_doc_links_question.md](private_intra_doc_links_question.md) with Todd. The
`docs` recipe's comment may be wrong. And `widgets::textarea::escape_text` is now `pub`
(for apcsp's Markdown widget), with NO re-export, per the 11-09 rule.

## 2026-11-09, overnight: boxes 7, 8, 9 done (Claude, alone), COMMITTED; box 10 is Todd's merge

**Morning, 2026-11-09:** committed one per box, `9ec2ef3` (7), `573a4e9` (8), `60a8500`
(9), NOT pushed. Trap 1 DECIDED by Todd: left as it is (a user shown an error would fix
it before submitting; no fix covers several messages), filed as #24, `_yet` test stays.
README "Crossing the wire" rewritten to `to_wire` / `absorb` / `WireForm` /
`reject_field(&path!(…))` (uncommitted).

Todd went to bed saying "7-10 are you ... do what you can without my input". All on
`errors`, NOT committed (commits only when told). `just ci` 1242 and `just e2e` 70/70 green.

1. **Box 7, `live_validation: on | off`:** `form!` key, `FormSpec::with_live_validation`,
   `Formoxus::with_live_validation` (field `live_validation`, default true),
   `FormState::live_validation()` / `Form::live_validation()`, `root_ctx` passes it as
   `is_live`. Beyond the box: a form-level "did you mean" (`FORM_KEYS`,
   `unknown_form_key`; a misspelled key used to read as a field missing `=>`), and a fix
   for EVERY form key given twice pointing at the end of the form ("unexpected end of
   input, …"), now at the key's span. Goldens `form_unknown_form_key_near_miss`,
   `form_key_given_twice`.
2. **Box 8:** `tests/suite/live_checks.rs` (20) and `live_validation.rs` (6),
   `Harness::blur`; `e2e/tests/live.rs` (12) on six `/t/live-*` forms. Suite and e2e
   both mutation-checked (8 and 6 breakages, all caught).
3. **TRAP 1 BITES (decided in the morning: left alone, #24):** a click on Submit just below a field with a
   bad value is LOST. The press blurs, the error renders, Submit (same element) moves down
   29.6 px of its 38.8 px before the release. Pinned as
   `a_click_on_submit_misses_while_an_error_appears_above_it_yet`. README now lists it.
4. **Box 9:** README "Checked as you go", `widgets.rs` "Writing a widget of your own",
   gallery `examples/src/examples/live.rs`, FIELDS_PLAN Step 7 pointer,
   `error_model_design.md` "As built". **#7 NOT closed**: the branch is not on `main`; the
   comment to post is in ERRORS_PLAN box 9.
5. **Found (fixed in the morning):** README's "Crossing the wire" example no longer compiles
   (`push_field_error` takes `&ModelPath<T>`). Box 10's apcsp counts were wrong (10 + 5
   `error_messages_at`, not 6 + 4); recounted there, with `markdown.rs`'s `props.values`.
6. Also: stray doc lines on `Form::use_browser_validation` moved to `values()` and
   `label_case()`; three intra-doc links off the prelude name `crate::Formoxus`.

## ▶ 2026-11-09: 6f and 6g done, uncommitted; next box 7 (Claude), then 8b–8e

6f (Todd): `Validity { Unchecked, Valid, Invalid }` (`field_attrs.rs`, `#[non_exhaustive]`,
`Validity::of(Option<&[ValidationMessage]>)`, re-exported from `formoxus::widgets`);
`with_error_attrs` takes it (`Valid` renders `aria-invalid="false"`, `Unchecked` nothing,
only `Invalid` adds the errors id to `aria-describedby`); `described_by` stays `bool`
(only `Invalid` matters to it, and the container nodes have no `Validity` to give).
`ErrorsStoreExt` (crate-private, `nodes.rs`): `errors_at -> Option<Vec<_>>`, OWNED because
a borrow would outlive the read guard, one key via `get_unchecked(..).try_read().ok()?.cloned()`
(never `peek()`, which subscribes nothing). `WidgetProps::validity()`; `is_invalid()` =
`validity() == Invalid`; `NodeContext::errors_at` and `widgets/values.rs` gone. 6g: `just ci`
and `just e2e` 58/58 green; two snapshots re-blessed (`form_error.html` gains `"false"`
everywhere, `field_errors.html` only on `subscribed`), three inline tests had mapped old
`false` to `Valid` where they meant `Unchecked`. Still mixed: some nodes import `Validity`
via `crate::widgets`, others via `crate::field_attrs` (Todd's A, open).

## ▶ 2026-11-08, late night: 6e committed; 6f next

6e: every built-in widget reads and writes through `props.field_handle()`; text and textarea
`on_input` + `on_blur`, select/checkbox/radio `on_change`, hidden `on_input`. The free
`get_current`/`write_value` are GONE (Todd's call: safer), their bodies private methods on
`FieldHandle`; the form's own silent writes (`absorb`, `reset`, `apply_edit`) use
`FieldHandle::unreported`, and `Form::write` a handle with the real callback and
`is_live: false`. `just e2e` 58/58 with the triggers live, but no e2e test covers live
behaviour yet (8c/8d). dioxus-stores 0.7.10 `HashMap::insert` marks the map's SHAPE dirty
as well as the key, so `set` an existing key, `insert` only a new one.

6f decided (Todd, 2026-11-08): the one-key error read becomes a crate-private extension
trait on `ErrorsStore` (`errors_at`, `validity_at`), replacing `get_errors` and the planned
`get_validity`; `widgets/values.rs` goes. Filed #23, a Django-style `BoundField` (Todd: "we
should have just followed Django and made a BoundField in the first place"): `FieldHandle`
is its value half; with #21 and #22 it reshapes WidgetProps/NodeContext, before publish.

## ▶ 2026-11-08, night: 6c and 6d committed; Todd on 6e

Todd built 6c and 6d. `FieldHandle` (from `WidgetProps::field_handle()`, NOT `field()`: Todd
expects `field()` to return a `Field`) is the only public way to write a value; `write_value`
and `get_current` went `pub(crate)`, `WidgetProps.values` too, `NodeContext::root` too.
`Form::write(&path!(…), raw)` writes from code: raw not typed (a `ModelPath<T>` knows the
model, not the leaf), `Err` when no field renders the path, and it reuses `apply_field_event`
with `is_live: false` so a clean result goes stale and nothing is checked. Todd moved every
path-taking method that only reads its path to `&ModelPath<T>` (clippy
`needless_pass_by_value`); apcsp's 14 call sites are listed in box 10. Two tests read the
store by hand now (`ProbeAdder`, `TagChip`), pending #22 (read-only value store, filed
2026-11-08; Todd wants it right after this plan). Gotcha: an intra-doc link only resolves to
an item VISIBLE from where the link is written, even with `--document-private-items`; a
private fn in `form.rs` cannot be linked from `widgets/`.

## ▶ 2026-11-08, evening: 6a and 6b committed, Todd on 6c

Todd built 6a (`changed_paths: Signal<HashSet<String>>` on `Form`, emptied last in
`with_errors`) and 6b (`FieldEvent { Written, Blurred }`, `apply_field_event` and the free
`check_live`, both beside `apply_edit`; `Form::check_live` gone). Both carry
`#[expect(dead_code)]` until 6c's handle calls them; `live: bool` comes in 6c. Review
lesson worth keeping: rustc's E0716 fix ("use a `let` binding") on `errors.peek().get(path)`
compiles but holds the read guard across an `insert`, a run-time "already borrowed" panic;
classify into a small owned enum (`AtPath`) first. `just docs` now also builds private docs
(ten broken private links fixed the same day).
6c names (Todd): `FieldEventKind { Written, Blurred }` (was `FieldEvent`), `FieldEvent { path,
kind, is_live }` (the message), `OnFieldEvent { callback: Option<Callback<FieldEvent>>,
is_live }` (the handler; `On…` = handler, like `on_edit`), public `FieldHandle`; all in a new
`widgets/field_handle.rs`. Written up as a table + "start to finish" in ERRORS_PLAN 6c.

## ▶ 2026-11-08, later: box 6 expanded, Todd's to build

Claude expanded box 6 of `formoxus/ERRORS_PLAN.md` into 6a–6g, uncommitted, and ticked
box 5. The calls to check with Todd: the typed-into set empties in `with_errors` (Enter
to submit, and `reset` reusing item keys, both break E otherwise); the handle holds
`Option<Callback<(String, Trigger)>>`, with `None` as
`WidgetProps::new`'s default; the trigger is a free function beside `apply_edit`,
replacing `Form::check_live`; the `Callback` comes from `use_callback` in `use_form`,
never from `root_ctx` (a new one each render defeats per-field rendering). Choosers call
both methods in `onchange`. Todd then asked for `aria-invalid="false"` (#7) back: it
is 6f, rendering a `Validity` enum from the store's `Option`, and 6b step 2 writes `None`
over a clean verdict on the first write after a check, knob on or off, so `"false"`
never outlives the value it judged. That made the handle's `None` mean "no form", with a
`bool` for the knob. Tests are 8e. Then Todd: "make the API more ergonomic, so devs
would have to work hard to do it wrong", so 6c now has TWO handles: the form-wide
trigger goes `pub(crate)`, and a public per-field `FieldHandle` (`props.field()`:
`current`, `input`, `change`, `blur`, and `on_input`/`on_change`/`on_blur` handler
builders) is the ONLY way a widget writes. `WidgetProps.values`, `write_value` and
`get_current` go crate-private; a `Form` method taking `Path<T>` replaces outside writes. Handler builders KEPT.
Claude updates the 6g suite tests. Issue #21 filed: a newtype for live paths (`qualify`
-> a method). Then `Path<T>` RENAMED `ModelPath<T>` and `model_path()` → `path_without_variants()`,
committed apart from the plan (`just ci` 1206 green, not pushed);
`path!` and the `path` module kept; apcsp `crates/web/src/server.rs:672` names
`formoxus::Path` and breaks until updated. Traps listed after 6g; the one likely to come up after
publishing is a blur-shown error moving Submit out from under a click.

## ▶ 2026-11-08: box 5 committed, next box 6

On branch `errors`, pushed. 4d needed no code (containers already read the store).
Box 5 is the live check: `Node::absent_owner` (the OUTERMOST absent option owning a
path; Todd chose this over a second `paths` parameter, which would clear under every
absent option), `FormState::check_live -> LiveCheck { errors_at_path, paths_to_clear }`
(both `pub(crate)`), and `Form::check_live`, which writes only the store and carries
`#[expect(dead_code)]` until box 6 calls it. Eight inline tests in `form/state.rs`,
mutation-checked; `Form::check_live`'s store writes are untested until box 6.
`just ci` 1206.

**Next is box 6** (Todd): the triggers on each control, and the `Copy` handle on
`WidgetProps`. Remove the `expect` when it lands, then Claude's 8b–8d.

## 2026-11-07, later: through 4c, committed

On branch `errors`. Box 3 (8e7083a), 4a and 4b (96100d2) and 4c are committed; `just ci`
1198 and `just e2e` 58 green. 4c split `with_owned_attrs` (`required`, in the node) from
`with_error_attrs(field_id, help_text: Option<&str>, is_invalid)` (the aria pair, in the
widget's component through `WidgetProps::current_attrs`). `WidgetProps::attrs` became
`pub(crate) base_attrs` so a custom widget can't spread attributes without their error
state, and `WidgetProps::new` (six args) is the public constructor. `VariantNode` is the
one exception: it applies both itself, as its select's `help_text` is `None`.

**Next is 4d** (Todd): containers read their own key in `render`. Then box 5, the live
check, whose open question is clearing a sibling's errors. apcsp's
`crates/ui/src/markdown.rs` (`:70`, `:93`) reads `props.attrs` and breaks at box 10.

How Todd wants to work now: [[feedback-todd-owns-the-runtime]]. Plans give what, why
and traps, no finished code unless he asks.

## 2026-11-07: 3b done

On branch `errors`. Todd finished 3b: `ordered_errors`, `Form::has_errors`, `to_wire`.
`ordered_errors` and `collect_errors` share a private `errors_from`, a change from the
plan (one walk, not two). The test owed from 11-05 is in `errors_by_path.rs`. **Next
is box 4** (Todd): widgets read the store.

`just e2e` failed twice once, on `required_text`; a page load hung, not 3b. See
[[e2e-harness]].

## ▶ 2026-11-05, end of day: 3b half done, pushed as work in progress

On branch `errors`. Box 2 is finished and committed (5c09320, ci and e2e green, 1194
tests). 3a is finished. **3b is half done: it compiles, clippy passes, 13 suite tests
fail.** The first thing to do is the three fixes listed at the top of 3b in
`formoxus/ERRORS_PLAN.md`; proved in a copy, they make all 715 suite tests pass. Then the
unticked 3b boxes: `ordered_errors`, `to_wire`, `has_errors`.

**Claude owes, once 3b lands:** a test that a live form's errors reach `to_wire` and
`has_errors`. Nothing covers `to_wire` today, so forgetting that box fails no test.
Written and mutation-checked 2026-11-05, in a scratch copy that is now gone:

```rust
#[component]
fn LiveErrors() -> Element {
    let form = use_form(|| empty_form::<Drawing>(FormSpec::default()));
    use_hook(|| form.validate());
    let paths = form.to_wire().errors().paths().join(" ");
    let has_errors = form.has_errors();
    rsx! { pre { id: "wire-errors", "[{paths}] {has_errors}" } }
}
// in errors_by_path.rs; render_to_html, expect contains_substring(">[name shape] true</pre>")
```

**Decided 2026-11-05:** the store's value is `Option<Vec<ValidationMessage>>` (trap 1).
3b and 3c merged into one step. `Form` is now 336 bytes, so helpers take `&Form<T>` for
clippy's `large_types_passed_by_value` (Todd chose that over raising the limit).

**A test trap worth keeping:** `collect_errors` walks live paths, so it HIDES a stale
entry in `field_errors`. A test of pruning has to bring the path back (Circle, Rectangle,
Circle) or read `field_errors` itself. The store has no such filter.

Plan steps Todd carries out get full sentences: [[feedback-plans-spell-out-the-why]].
Saying `reset` was "one line" cost its values loop.

## 2026-11-05, earlier: box 2 in progress, pushed as work in progress

On branch `errors`. **It does not compile, on purpose** (55 errors, mid-refactor).
Todd writes this code; I review each function when he asks.

**What is done and left: the checkboxes in box 2 of `formoxus/ERRORS_PLAN.md`,**
rewritten 2026-11-05 as a work order after Claude built all of it in a throwaway copy.
Trust those over anything older here. The sub-box letters changed: 2e is now
`VariantNode`, 2f `SetNode` (the draft decision), 2g `FormState`. Claude owes 2j, tests
for eight lines the suite does not cover (each was broken on purpose and all 864 passed).

**Things to remember while reviewing what is left:**

1. **Drafts: DECIDED 2026-11-05,** Claude's proposal; see box 2f. `paths` gains a
   parameter, so the ones already written change. `set_node.rs:576` is NOT a scratch
   probe: it writes the real map.
2. **`a && b` loops:** `child.validate(…) && is_valid`, child first, or later
   children never run. Twice a real bug here.
3. **Prefix mistakes compile.** Every bug so far in `Items`, `SetNode` and
   `OptionNode` was a path built wrong, and none was a compile error: a node adds
   its OWN name, so a container passes its own path down as the prefix, and
   `OptionNode` adds nothing (`?` is `form!` syntax only, gone at runtime).
4. **An empty `Vec` in the map is deliberate:** checked and clean, for #7. I called
   it a bug and was wrong. Readers filter. The STORE's value is `Option<Vec<…>>`
   (Todd, 2026-11-05): `None` is cleared or never checked, so "cleared" and "checked
   and clean" stay different.
5. **`paths` MUST walk an absent option's fields** (settled 2026-11-05, proved by
   tests at ce4d154): a form validator (`path!(M.phone?)`), `push_field_error` and a
   server's reply all land there. "An option emptied" is out of the plan. Decided
   with it: 3b's drain also empties store keys missing from the map; box 5 clears
   siblings (how to tell them from errors `validate` cannot re-derive is OPEN at box 5).
6. **Tests I owe, in 8b:** two bad items both report; a count error still reports
   its items' errors; item paths pinned exactly (`tags.#0`, not `tags.#0.#0`).

## ▶ 2026-11-04: road to publish grew four widgets

On `main`, level with `origin`; #14 is committed as `2db98c1`. The order is now
`ErrorsStore` + live validation, then `range`, `checkbox_multiple`,
`select_multiple` and `file`, then docs. See [[mvp-scope]] "ROAD TO PUBLISH".
`file` is for apcsp: students upload assignments.

**`formoxus/ERRORS_PLAN.md` written, on branch `errors`, uncommitted.** Todd takes
the runtime (boxes 2–6); Claude has the `live_validation:` knob, tests, e2e, docs.
E (a blur checks only a field typed into), F (the `live_validation:` knob) and G
(handlers on each control, Todd's call, over Claude's `field_row` wrapper) are decided.

The design moved in conversation: errors LEAVE the nodes, into
`FormState.field_errors` (plain, for the server) and `ErrorsStore` (client), not a
projection as [[error-model-design]] had it. Todd's question "does it matter that
they get out of sync?" is what changed it. Two corrections Claude had to make: a
live check clones the top-level node, not "asks the field through `&self`"
(`OptionNode`'s required depends on siblings), and a store's `remove` does not wake
a reader of that key, so errors clear by writing `None`.

**#19 filed:** a GOV.UK preset (Todd: "a single setting that follows gov.uk
standards"), written as a preset over individual settings. AFTER the MVP: Todd
disagrees with the guidance enough that he would probably never use it, and it is
additive, so waiting costs nothing.

## ▶ 2026-11-03: #14 item counts built (committed 2db98c1)

`min_items:`/`max_items:` (Todd's names) on a list OR set, `with_min_items`/
`with_max_items`. Decided with Todd: checked in `Items::validate` (the
collection's own error, whatever its items say: "Add at least 2 items." / "No
more than 3 items."); Add DISABLED (not hidden) at the max with a `title`
reason, a set's Add stays enabled while REPLACING; Remove disabled at the
min; legend required marker when min ≥ 1; `empty_form` PRE-BUILDS a list's
min blank items (lists only, BLANK walk only, so a new item's nested list
gets its own); a server request listing more keys than the max is refused
UNBUILT (`Items::over_max`). `AddItemProps.max_items` for custom adders.
`form!`: list-only `const` check shared with `adder`, crossed literal bounds
refused at parse. `parse_entry` split (`parse_keyed`) for clippy's line limit.
Suite `item_counts.rs` (14), 3 mutations caught.

## ▶ 2026-11-03: #16 trim committed; the road to publish

**#16:** `FieldNode::distribute_values` trims raw text before the empty check
and the parse (the store keeps what was typed; the server trims the same).
`Field::trims(&self, password)`, default false (bool, choices read as sent);
`StringField.no_trim: bool` → trims unless `no_trim` or a password input;
`NumberField` always (`" 42"` did not parse at all before). **Changed from the
issue's decisions (Todd, 2026-11-03):** a bare FLAG `no_trim`, not `trim:
false`, and a password NEVER trims (no opt-in), so the setting is a plain
`bool`. `form!` merges `set` and `no_trim` as the two bare flags. Populating
from a model does NOT trim (only raw text is read), pinned in `empty_strings`.

**Road to publish (Todd):** #16 → #4 (filter a field's own constraint attrs by
the widget's element; flip `a_textarea_gets_pattern_although…`) → #14 (item
counts; name open, I recommended `min_items`/`max_items`) → `ErrorsStore` +
live validation together (BOTH before publish; write `formoxus/ERRORS_PLAN.md`;
my open proposals: "reward early, punish late" timing, field checks live and
container/form checks on submit) → docs (A1 grammar, A2 pass). facet 0.50:
wait for stable. #18, #7, #8 after.

## ▶ 2026-11-03: C4 per-field validators, committed 3930362

See [[error-model-design]] "Per-field validators" for what was built and how
it departs from the 2026-09-21 design. Suite `field_validators.rs` (19,
mutation-checked in 4 places), golden `form_validator_of_the_wrong_type`,
gallery: the trip's stops need at least two. Checks take clippy's preferred
parameter (`&str`, `&[T]`, numbers by value), via autoref specialization.
Next on the MVP list: A3 (facet version),
then A1/A2 docs.

## ▶ 2026-11-03: `help_text` (C5) built, committed 5ec7cdd

`form! { name => { help_text: "…" } }` / `FormSpec::with_help_text`. Rendered
by `widgets::field_help` as `div.fx-help-text#{id}-help` RIGHT AFTER the
caption (Django 5's field template and GOV.UK's hint order: caption, hint,
control, errors; a checkbox: box, caption, hint), or after the legend for a
radio group, enum, struct, list or set. `aria-describedby` = hint then errors
(`widgets::described_by`), on the control, or on the fieldset for a group.
`WidgetProps.help_text` (custom widgets render it with `field_help`);
`with_owned_attrs` takes `help_text: Option<&str>` (a 4th bool tripped
`fn_params_excessive_bools`). Suite `help_text.rs` (10), mutation-checked; NO
e2e (playwright-rs has no accessible-description assertion; a CDP session's
`Accessibility.getPartialAXTree` could do it, offered to Todd). Gallery: the
tags example has one. Also added: placeholder tests on a textarea and a set
draft. Next on the MVP list: C4, per-field validators.

## ▶ 2026-11-03: set items by value, choice labels

Todd asked whether an error can go on a set item (a choice no longer
legal). Static `choices:` already refuse it on the item. Built for a dev's
own check: `path!(Post.tags[= "rust"])`, see [[indexed-paths]]; suite
`set_item_errors.rs` (8), goldens `path_value_on_a_list`,
`path_value_of_the_wrong_type`. Also fixed: a set item over choices showed
its VALUE ("BOS") where its draft's select showed the label ("Boston");
`Node::shown_text` (default: `collect_values` joined; a leaf gives its
choice's label), used by `item_text`, so text, Edit/Remove names and
messages all read the label. Committed 1cfd158 + beea08a.

**Then the unit-enum gap, fixed (Todd: "if it's not too complicated"):** a
unit-enum item showed its variant name raw ("NewEngland") where the draft's
select label-cased it. The case is resolved at RENDER, but messages were
worded at EDIT, so `Node::shown` now returns `Shown`, pieces with the
variant's marked (`VariantNode::shown`), cased by `Shown::in_case` at
render; `SetMessage` keeps formoxus's words as `Words` (items still as
`Shown`) and its public `text` field became `text(case)`. A dev's
`feedback:` text is kept as said. Uncommitted at writing.

## ▶ 2026-11-03: boxes 6–7 + #17 + indexed `path!` COMMITTED

The box 7 blocker is solved: `path!(Model.list[i])`, see
[[indexed-paths]]. `just ci` 1086, `just e2e` 58, apcsp `check` + `test` green
against it (apcsp never relied on `Path: Copy`). Next: apcsp's box 8 (the
assignment form, `SearchToAdd` as the adder, its duplicate check through
`path!(…[g]…[q])`), and its `select[id$=…]` e2e workaround → `select[name=…]`. Open for then (Todd, 2026-11-03: decide in
apcsp): make `Random`/`Streak`'s `set` pool a real set (`set` in `form!`, or a
`BTreeSet`), so a duplicate within a pool is refused for free and the validator
only checks across groups.

## ▶ 2026-11-03: boxes 6–7 + #17 built

`ORDERED_LISTS_PLAN.md` box 6: nothing to build, a custom widget is already
a display-only list item (`vecs.rs` `a_display_only_item_*`). Box 7:
`tests/suite/assignment_shape.rs` (8), e2e `/t/assignment` (3), gallery
"Lists inside a list". **Blocked in box 7:** a `validator:` cannot put an
error on ONE item: `path!` has `[]` but no index, `Path` has no public
constructor, errors key by item key while a validator sees indices. Server
errors via `absorb` work. Proposal raised with Todd: index segments in
`path!`, resolved index → key by `FormState`. #17: `VariantSelect` sets
`name` to the enum's path (suite + e2e, mutation-checked); apcsp's
`select[id$=…]` workaround in its e2e can now become `select[name=…]`.

## ▶ 2026-11-03: merged; apcsp renames + layout committed (4141a4d)

`sets` fast-forwarded into `main` and pushed (42e4de3); `sets`,
`radio-group` and `widget` deleted locally and on origin (all merged).
In apcsp-dioxus: the class renames in `main.scss` (+ `main.css` regenerated
with the pinned sass, a renames-only diff), `questions.rs` selectors, one
`FieldSet` comment. `just check`, `just test` (184) and `just e2e` (27)
green. Also fixed (Todd's go): apcsp's `.fx-list-item > :first-child
{ flex: 1 }` assumed the field came first, but every list now starts with
`.fx-move-buttons`; the field is now `> :not(.fx-move-buttons,
.fx-button-remove-item)`, and ↑/↓ are compact + secondary like Remove,
stacked. Seen in a browser. Committed in apcsp as 4141a4d, NOT pushed.
Next there: ORDERED_LISTS_PLAN boxes 6–8.

## ▶ 2026-11-03: box 12 DONE; `sets` is ready to merge

Every box of `SETS_PLAN.md` is ticked (boxes 2–5 had only their parents
left open). `main` has not moved since 314bcd4, so the merge is a
fast-forward. `ORDERED_LISTS_PLAN.md` is marked: 2–3 superseded, 1/4/5
done, 3b parked, **6–8 still open, all for apcsp.** Next, after the merge,
in apcsp-dioxus: the renames listed in SETS_PLAN box 12 (`.fx-list-set`,
`ListSet`, `fx-button-add-row`/`-remove-row`, `fx-list-row`, and
`crates/e2e/tests/questions.rs`), then ORDERED_LISTS 6–8. apcsp gets the
seeded-textarea fix for free.

## ▶ 2026-11-03: box 13 + textarea fix COMMITTED

Box 14 committed ed408c0. Box 13: four gallery examples (sets.rs, lists.rs),
list numbers shown by a CSS counter, `.fx-control` border-box. Found and
fixed: a seeded `<textarea>` was blank after fullstack SSR + hydration (SSR
emits a `value` attribute, which a textarea ignores). Fix: also
`dangerous_inner_html` = the escaped value, NOT a text child (pre_render puts
`<!--node-id-->` markers inside, shown literally); `value` stays for client
changes. Details in SETS_PLAN box 13. Next: box 12, then apcsp after merge
(apcsp gets the textarea fix for free).

## ▶ 2026-11-03: box 14 BUILT (committed ed408c0)

Todd's design: the draft's input carries none of its field's constraint
attributes except `maxlength` (`NodeContext::in_draft`, `FieldAttrs::keeping`);
Add/Replace still validates them. No `Draft` newtype: `validate` never saw the
draft. Next: box 13 (gallery examples for a set and a list), then box 12.

## ▶ 2026-11-03, newest: box 11 BUILT and committed

Rows → items everywhere a dev or Formoxus dev sees them; details in
SETS_PLAN box 11. Kept: textarea `rows`, the attribute/widget tables' rows,
`field_row`, the button row. A list's labels are "Remove item N" / "Move
item N up" (Todd: consistent); box 13 must decide whether the gallery shows
the `<ol>`'s numbers, which its CSS hides. apcsp's uses of the old classes listed in box 12.
ORDER (Todd): boxes 13 (galleries) and 14 (draft pattern gap) BEFORE box 12 (merge prep); apcsp-dioxus waits until `sets` is merged into main.

## ▶ 2026-11-03, newest: box 10 BUILT and committed

`/t/tags` + `/t/tags-pattern`, `e2e/tests/tags.rs` (11), full e2e 52/52,
`just ci` green 1050. Todd: hold the gallery until the plan is done, then
gallery examples for BOTH a set and a list (box 13). Box 7's pattern gap is
real (box 14, design first). The set's "Save" clashed with a form's `save`
submit: now "Replace", `aria-label="Replace “rust”"`, named by the edited
ITEM so it stays true as the user types (Todd), visible word first for WCAG
2.5.3. Committed. Also learned: [[mutation-restore-mtime]]. Next: box 11.

## ▶ 2026-11-03, newest: box 9 BUILT and committed

Todd: "Go ahead and do 9." Built: Remove named by its item, `fx-set-adder`
wrapper (`FixedKey::SetAdder`), gallery CSS for sets plus an opt-in
`fx-tags` chips preset. Then, Todd's "we probably want that now": `item:`
in `form!` / `with_item_view`, an `ItemView` component over `SetItemProps`
that replaces an item's TEXT only (Todd), default `ItemText`. `just ci`
green 1050. Delete/Backspace removal skipped (needs a
browser). Next: box 10 (`/t/tags` e2e + gallery example).

## ▶ 2026-11-03, newest: box 8 BUILT and committed

Todd: "Go ahead and write box 8." Built by Claude: `Edit::EditItem { path,
index: Option<usize> }`, items shown as text + errors + Edit + Remove (no
longer editable in place), Save/Cancel/Escape in `DraftAdder`, Edit a
toggle (`aria-pressed`), `SetEvent::Changed { from, to }`. `just ci` green
1036, 14 mutations caught. TEN decisions made on his behalf, listed under
box 8 in `SETS_PLAN.md`; Todd accepted all of them and said commit. Next:
box 9 (display; Remove named by its item).

## ▶ 2026-11-03, newest: box 7 BUILT and committed (213f67b)

Box 6 committed d3d4852. Box 7 planned in `SETS_PLAN.md` (Todd decided: a
DRAFT item rendered by the item type's own widget, not a dedicated text box;
commit = `Edit::AddItem { value: None }`; leftover draft text ignored at
submit; label "Add to {set label}"; no filtering of choices yet). Todd then
said "Go ahead and do this. I'm away from the computer", so Claude built
7a–7d AND 7t, though the plan said Todd builds. `just ci` green, 1017 tests,
12 mutations caught. Todd reviewed it and it is committed, 213f67b. Box 7's "Built" note in the plan lists
what turned up (label via `NodeContext::label_if_unnamed`, draft under
`optional()`, unit-enum draft = `VariantNode` so its structure stays out of
the store, `ItemHandler::position` takes a node, `FormState::is_set` for
focus, every added key +1). Known gap recorded there: a draft breaking a
`pattern`/`min`/`max` blocks a browser-validated submit; Todd: wait for
box 10's e2e to confirm it. Todd OK'd `label_if_unnamed`. Next: box 8 (Edit
fills the draft).

## ▶ 2026-11-03, latest: box 4 committed (7a31548); box 5 EXPANDED, awaiting Todd

Box 5 (a `Vec` marked `set`) is now 5a–5f, 5p, 5t in `SETS_PLAN.md`.
DECIDED: A `ItemOrder` -> `ItemHandler` (orders + dedups, box 6 reuses it);
B `with_sort_by` on an unmarked `Vec` panics "call `treat_as_set` first"
([[feedback-macro-first]]); C J stays open until 5p times large sets
(Claude, after 5d). Next: Todd 5a–5e; Claude 5f (2f4), 5p, 5t.
Same day: D, a duplicate from the wire is dropped quietly (as `BTreeSet`
and serde do), never refused. Todd named 5a's field and builder
`treat_as_set` (`FieldSpec.treat_as_set`, `FormSpec::treat_as_set(path)`),
no `with_` prefix; `form!`'s key stays `set`.
Later: 5a DONE (Todd); 5f/2f4 DONE (Claude: `set` -> `.treat_as_set(path)`
emitted before `.with_sort_by`), 5t item 8 done (8 spec tests). Found:
`rows_of` (build.rs) accepts only lists, so `tags[]` on a set panics at
run time; FIXED in 5b (Todd: `rows_of` -> `items_of`, "not a collection"), test added.
5b DONE (Todd). 5c DONE (Claude, Todd asked: pairwise `without_duplicates`
in build.rs; Peek has no Eq and its Hash panics on unhashable types). Next:
Todd 5d (ItemHandler + dedup_nodes), 5e (write a Vec).
5d DONE (Todd: `retain_first` in build.rs shared by build dedup and both
`dedup_nodes`; `Items::dedup` after the sort in distribute). Stale store
entries from a client-side dedup: LEFT on purpose. Claude waits for 5e to
write the rest of 5t (wire dedup, validate gives a sorted deduped Vec).
5e DONE (Todd: write branches on `partial.shape().def`). 5t DONE (Claude, 9
more set tests, 39 in sets.rs). Box 5 left: only 5p (Claude: time large
sets). All of box 5 uncommitted, `just ci` green.
Box 5 COMMITTED e55e318. 5p DONE: up to 1,000 items everything < 6 ms; at
10,000 the pairwise dedup costs 0.1-0.5 s. J CLOSED by Todd ("plenty fast").
The O(n) default-order fix (adjacent dedup after sort) is written in 5p for
later. Todd started box 6 (duplicates refused at the adder).
Box 6 core WORKING (uncommitted, just ci green, 986): Todd's `add_message` on
SetNode + `AddItemProps::message`; Claude cleaned up: `ItemHandler::contains`
(DefaultOrder peek_value ==, CustomOrder T ==), `Items::contains(order, v)`,
duplicate -> `Ok(None)` + message (decision B), every set edit clears it. 11
tests in sets.rs, mutation-checked. OPEN: message markup (no fx- class, no
role=status), decision I's `on_duplicate` closure + form! key not built.
LATER same day: Todd generalized to every set edit (decision I′ in the plan):
always-present `role=status` region after the adder, `FixedKey::SetMessage`
/ `fx-set-message` (+`fx-invalid` on refusal), Django's wording ("“rust” was
added successfully." / "removed successfully." / "is already there."),
sets only, not auto-dismissed. BUILT (Claude), just ci green 989. Removed
`AddItemProps::message` (the set renders the region). LEFT: the dev hook
`fn(SetEvent<'_, T>) -> Option<String>`; OPEN Q to Todd: plain fn or closure.
Hook BUILT: a closure (Todd), `feedback:` / `with_feedback` + `SetFeedback::new`,
`SetEvent` Copy + non_exhaustive, `Send + Sync` (so `use_signal_sync`, not
`use_signal`: see [[dioxus-signal-not-sync]]). Box 6 DONE, uncommitted, just ci
green 1001. Next: box 7 (the default adder for a set); look at
dioxus-primitives' `tag_group` first. Todd next: 5b (locations given:
node_for_shape's into_list branch, set_node's peek; NO kind field on
SetNode: 5e branches on `partial.shape().def`). All uncommitted, `just ci` green (967).

## ▶ 2026-11-03, later: box 4 DONE (4a–4c4), committed 7a31548, `just ci` green

Todd: 4b (`ascii_`/`unicode_case_insensitive`, in the prelude). Claude:
4c, `form!`'s `sort_by:`, expanding to
`set_items_of(projection).sort_by(order, name)` with a `SetLike` trait
(`on_unimplemented`); why two calls and not the plan's one is in the plan's
4c "As built". 7 goldens, 13 macro tests, 7 suite tests (`sets.rs`, now 27),
5 inline tests of the case-insensitive orders. GOTCHA found: a message
literal written inside `quote_spanned!` on a path takes the path's span,
and clippy reads `r#type` there as a raw string; interpolate messages.
OPEN for 2f4: `set, sort_by:` on a `Vec` panics in `with_sort_by` until it
accepts a field marked `set`. Next: box 5 (Todd), or 2f4.

## ▶ 2026-11-03: `with_sort_by` + tests A/B done; 4b in progress (Todd)

Todd added `FormSpec::with_sort_by(path, SetOrder)` (asserts `is_set`;
must also accept a `Vec` marked `set` once 2f4's `treat_as_set` exists).
`SetOrder::default_for(shape, path)` now panics with
"tags: a set of `Tag` has no default order; give it a sort_by …". Claude
wrote 4 order tests in `sets.rs` (build, AddItem, rebuild from the wire,
a struct item with an `Empty`/`Invalid` field sorting last); read item
order from `collect_values`' structure entry, `#N` keys in screen order.
4b DECIDED: `ascii_case_insensitive` + `unicode_case_insensitive` in
`nodes/set_node.rs` (no `sets` module), generic over `AsRef<str>`, on
`unicase` (new workspace dep), tie-broken by `str::cmp`. Next: 4c (Claude).

## ▶ 2026-11-02: 4a built; next `with_sort_by`, then Claude's tests A/B, then 4b

4a (Todd, with Claude's hints): `SetOrder { item_order: Arc<dyn ItemOrder> }`;
trait `ItemOrder: Debug + Send + Sync` with `sort_peeks(&mut Vec<Peek<'_,
'static>>)` and `sort_nodes`. `DefaultOrder { scalar_type, peek_cmp: fn ptr }`
(total_cmp for floats, `Peek::partial_cmp` otherwise); `CustomOrder<T>`
rebuilds each item's `T` once via `value_of`, which VALIDATES A CLONE first
(`write_value_into` hits `unreachable!` on `Empty` as well as `Invalid`), then
`expect`s, since a validated item failing to build is a formoxus bug (Todd
agreed). `SetOrder::default_for(item_shape)` panics for non-scalars, even on
an EMPTY set; `SetOrder::custom(fn, name)`. All green, 926 tests.

Claude wrote test C (`a_set_of_non_scalars_without_sort_by_fails_at_construction`).
BLOCKED: tests A (newtype set in custom order at build, after AddItem, after a
failed submit via `empty_form` + `distribute_values`) and B (struct item with
a blank field sorts last, no panic) need a `FormSpec::with_sort_by`, which
does not exist (`FormSpec.fields` is `pub(super)`). Set-only, unlike
`collection_field`. OPEN for Todd: (A) who writes `with_sort_by`; (B)
`default_for`'s panic prints `{:?}` of `shape.ty`, a big dump; `{shape}` plus
the path would be better. Then 4b (`case_insensitive`).

#18 filed (walk a model's whole SHAPE up front, registry via `inventory`):
PARKED by Todd until there is more time; don't fold it into sets.

## ▶ 2026-11-01, later: SetNode, its tests, `form!`'s `set`

Todd built `SetNode` (2d) and the `Set`/`SetItem`/`SetItems`/`SetLegend`
`FixedKey`s (2e); a `Def::Set` field now renders a `<ul class="fx-set-items">`
with Remove per item and the adder after it. Its items still render their
leaf widgets, so they are editable, accepted as broken until box 8. Claude
wrote `tests/suite/sets.rs` (2g, 15 tests), made `adder:` accept a set
(2f1), parsed the bare `set` flag (2f2, not yet expanded) and gave it its three `const` checks with `field_kind::is_list_of_leaves` (2f3). DECIDED for
box 8: Edit MARKS the item rather than removing it, so submitting
mid-edit loses nothing, and the draft goes into the adder's text box.
Nothing committed since c78a8e1. Next: box 4 (sorting rules in 4a); 2f4 waits on Todd's `treat_as_set`.

## ▶ 2026-11-01: 2c and box 3 done

Todd built `nodes/items.rs`: `Items` (every field private) holds the rows
and the own errors; `ListNode`'s `Node` methods delegate to it, and the
row names are already items (`add_item`, `ItemValue`, `ITEM_SIGIL`,
`item_segment`, `Edit::AddItem`/`RemoveItem`/`MoveItem`). `ITEM_SIGIL` and
`item_segment` are re-exported `pub(crate)` from `nodes`, because `mod
items` is private. Box 3 done: every list renders `<ol>` + ↑/↓, snapshots
re-blessed. Tests now pick a row's button by `aria-label` with
`Harness::labelled`; one row brings ↑, ↓ and Remove, so "the first new
click listener" no longer means Remove. Next: Todd's 2d (`SetNode`), then
`Items::edit_within` when the set needs the same forwarding loop.

## ▶ 2026-10-31, later: lists vs sets, on branch `sets`

Todd decided a collection is a LIST (`Vec`, order matters: always `<ol>`
with ↑/↓) or a SET (`Def::Set`, or `set` on a `Vec`: no duplicates,
canonical order, display-only members edited through the adder).
`allow_reordering` is gone. Plan: `formoxus/SETS_PLAN.md`, decisions A–G
and settled H–M; memory [[lists-and-sets]].

Same day, Todd: SEPARATE nodes, not a kind switch. `ListSet` -> `ListNode`,
`VariantSet` -> `VariantNode`, a new `SetNode`, the shared row machinery in
`nodes/rows.rs`, and the "set"-named classes/`FixedKey`s renamed
(`fx-list-set` -> `fx-list`, …). That is now box 2, first; the old box 2
(every list ordered) is box 3. `on_duplicate` is the duplicate closure.

Box 1 (branch) done. Box 3b (was 2b), Claude's half, done uncommitted: the `form!`
key, its goldens (the `[]` one converted to `adder`), the specs.rs panics
moved to `with_adder`, vecs.rs's reorderable tests rewritten as every-list
tests, `/t/reorder` without the key. **10 vecs tests are red on purpose
until Todd's 3a** (every list `<ol>` + ↑/↓, `FieldSpec.allow_reordering` and
`with_allow_reordering` deleted). After 3a (box 3c): re-bless any snapshot that
gains an `<ol>`, run `just e2e`.

## ▶ 2026-10-30/31: BOX 2 DONE (ec05880); BOX 3 DONE (ebe4437); BOX 4 DONE, uncommitted

Todd decided 2-i (a BARE FLAG only: `allow_reordering`, never `: true`,
like every other flag; first answered "both" from Claude's wrong claim
about `required`), 2-ii (assert, in `with_allow_reordering` via
`spec_key_error` returning the shape) and 2-iii (plain `bool` on
`FieldSpec`). Split: Todd 2a–2d (runtime), Claude 2e–2g + all tests.
Claude's macro side and 6 parse tests are in; `form!` using the flag
won't compile until Todd's `is_list` and `with_allow_reordering` exist.
Later the same day: 2a–2c DONE (Todd), 2t2 DONE (goldens + specs.rs
panics); decision H: Add moves to the TOP of every list, rows still
append at the bottom; REVERSED the same day: Add stays at the bottom,
reach solved by sticky CSS in box 3, per-row insert parked as box 3b
(Claude argued against over-engineering; Todd agreed). Box 3a (focus a
new row) queued, lower priority. 2d adds `fx-list-rows` and renames
`fx-form-row` → `fx-list-row`.
All of box 2 then landed: 2d (Todd) and 2t3 (Claude, six render tests in
`vecs.rs`). Committed and pushed as ec05880. Then box 3 (Claude): move
buttons, icons on Add/Remove for every list, `/t/reorder` e2e; details in
the plan's box 3. Committed as ebe4437. Next: box 4 (Todd). 3a (focus a
new BLANK row) waits for box 4 by decision; its shape is in the plan.
Box 4 in progress, UNCOMMITTED by decision (commit once box 4 is done):
4a–4c + 4t1 done; also both `optional` slips fixed (ListSet rows,
VariantSet::set_variant; see [[optional-flag-is-payload-only]]). `just ci`
and e2e 37/37 green. Then 4d (Todd: `Node::edit` returns
`Result<Option<String>, _>`, the created row's path), 4e (Todd: on_edit copies
a filled row's leaves into the store), 4e+ (Claude: shared `apply_edit` +
public `Form::edit`, which returns the error and path instead of logging),
4t2–4t6 (Claude; the 4 live-form probes mutation-checked) and 4f (Claude,
2026-10-31: docs on `RowValue`, `AddRow::value`, `Node::edit`). **All of box
4 done, `just ci` green, e2e 37/37; awaiting Todd's go to commit.** Next after
that: 3a (focus a new blank row), then box 5.

## ▶ 2026-10-29: `formoxus/ORDERED_LISTS_PLAN.md`, box 1 DONE; box 2 next, Todd picks his steps

Todd switched back from apcsp-dioxus to build this. apcsp wants its "new
assignment" page (ordered groups of questions, search-to-add) as a formoxus
form; lists need two general features first: **moving a row** and **adding
a row that arrives filled in**. Everything else that form needs already
works (checked: `Vec<enum>` rows, nested lists, numbers, path errors, a
widget reading the whole store).

**Decided the same day, all recorded in the plan (A–G):** reordering is
opt-in (`answers => { allow_reordering }`, the list's OWN path, not
`answers[]`, which is every row); a reorderable list renders `<ol>`/`<li>`;
a filled row comes from a TYPED value on `AddRow`; the adder is a plain
component fn set by `add_row_button:`; ↑/↓ on a row's left, ✕ on its right;
icons are Lucide, INLINED as SVG (the `dioxus-free-icons` crate measured
13.8 s and 65 MB per target for four icons); a display-only row needs no
`<label>`.

The finding that changed B: `Node::edit` never sees the live `ValuesStore`,
so a filled row is built from the value in the tree, `FormState::edit`
returns its path, and `use_form`'s `on_edit` copies its leaves into the
store. Open: how the value is type-erased inside `Edit` (box 4).

**Box 1 done:** Todd wrote `Edit::MoveRow { path, from, to }` and
`ListSet::move_row` (remove + insert, a bad index is a `FormAccessError`);
Claude wrote 9 tests in `tests/suite/vecs.rs` ("Moving rows") and the doc
comments on `from`/`to`. Two bugs caught in review before the tests: `len - 1`
underflowed on an empty list, and the first version swapped instead of moving.

**Next, box 2:** carry `allow_reordering` from `form!` to `ListSet`, split
into steps 2a–2g with file:line pointers. Todd is to pick which steps he
takes (natural split: runtime 2a–2d vs macro 2e–2g) and answer 2-i (bare
flag too?) and 2-ii (hand-built `FormSpec` on a non-list: ignore or
assert?). Then box 3, the move buttons, is Claude's, copied from apcsp's
`crates/ui/src/forms/ordered_picker.rs`.

## ▶ 2026-10-25: issue #15 BUILT, UNCOMMITTED; Todd to review, then commit and close

Todd picked: **A** (name the enum: `data.$QuestionData::TrueFalse.text`), tuple
fields by index (`.0`, `.1`), the nested `prefix => { … }` block, and **C**
bundled in (a string path that names no node now PANICS in `FormSpec::field`).
Then, same day: `?` REQUIRED on every `Option` in a `form!`/`path!` path; top-level
enum models dropped. Design and gotchas in [[variant-paths-in-form-macro]].
Todd reviewed every new compiler output and approved; the seven older Option
goldens got a `?` and were re-blessed. `just ci` green. One old test reversed on purpose:
`specs::an_unmatched_path_is_a_no_op` → `an_unknown_field_is_refused` and friends.

apcsp-dioxus's waiting edit (`crates/api/src/questions.rs`) needs `$TrueFalse` →
`$QuestionFormData::TrueFalse`, a `?` on any `Option` field it names, and a
`,` after that entry's `}` before
`buttons:` (top-level entries are comma-separated; inside a body they are
optional). Then `web`'s `views/questions/forms.rs` layer and its
`MARKDOWN_FIELDS` guard can go.

## ▶ 2026-10-24: issue #13 FIXED (4dcf0fb, pushed, closed); NEXT is #14 or Todd's call

**#13 is done:** an enum's chosen variant and a list's rows were lost on the
way to a server (enums rejected, `Vec`s silently `Ok([])`). The design is
[[structure-on-the-wire]]; the build record, with every decision and box, is
`formoxus/ISSUE_13_PLAN.md` (all 26 boxes done, 15 skipped by decision),
deleted once #13 closed; read it with `git show 4dcf0fb:formoxus/ISSUE_13_PLAN.md`.
`just ci` and the full `just e2e` (35 tests) were green at 4dcf0fb, and
apcsp-dioxus's two `create_question_*` HTTP tests pass against it, so its
question editor is unblocked. Todd wrote most of the library side; Claude
wrote `ListSet::distribute_values`, `ListSet::render`'s error display, the
docs in step 5 and all the tests.

**Queued: issue #14, min/max row counts for a list** (Todd: "probably worth
doing"). Builds on #13's list-level error path; open questions in the issue
(the name: `min_length`/`max_length`, `min_elements`/`max_elements` or
`min_num`/`max_num`; whether a minimum pre-builds blank rows).

**Also open, found during #13:** tuple-variant fields render with NO
`<label>` (`default_label("0", …)` is `None`); Todd: require the dev to label
them, `FIELDS_PLAN.md` V4. (`form!` can now address them,
`shape.$Shape::Rect.0 => { label: … }`, since #15.)

## Coming back after a break (written 2026-10-23)

**Where it stands.** Everything is on `main` (`widget` was merged and
deleted 2026-10-23), pushed, `just ci` green. `formoxus/FIELDS_PLAN.md` is
the plan of record; **Step 5, the widget refactor, is DONE**: widgets are
Django-style types implementing `formoxus::widgets::Widget` (consts `NAME`,
`PARTS`, `EXTENDS`, `ACCEPTS`, `ACCEPTS_OPTIONAL`, `TAKES_CHOICES` plus
`render`), held as `AnyWidget`, rendered from one `WidgetProps`; `form!`
checks every widget, a dev's included, through `const _` asserts on those
consts; classes are keyed `(W::NAME, Tag)`; `BuiltinWidget`, `FieldControl`
and `RenderedInputType` are gone.

**Paused on purpose (Todd: "use formoxus for a while before I commit to
anything").** Two Step 5 boxes are DEFERRED with full surveys and line
numbers kept in the plan:
- the variant picker as an ordinary chooser widget (so `with_widget` could
  put radios on an enum): open questions V1–V3. Nothing is broken; a bare
  enum renders as a `<select>` and round-trips.
- `#[derive(Widget)]`: open question DW1, where `render` goes, since a
  derive cannot share an `impl Widget` with a hand-written `render`.
Feedback from real use should decide both, so expect that rather than
"what's the next plan step". Step 6 (`field:` in `form!`, a type → field
registry) is the next un-deferred step.

**apcsp-dioxus DOES use this repo (corrected 2026-10-23).** It was
repointed at `../formoxus/formoxus` on 2026-09-29 (apcsp `2994d6e`); the note
that stood here was written from a machine that hadn't pulled that. On
2026-10-23 apcsp branch `external-formoxus` caught it up to this repo's
post-09-30 API (`ValidationMessage`/`ValidationError<T>`, the `Widget` trait).
That port surfaced one miss, fixed here the same day: `field_row`,
`field_label`, `required_marker`, `LabelSide` and `WidgetProps::part_class`/
`control_class` were `pub(crate)`, so a custom widget couldn't share the
built-ins' layout. Do not edit apcsp-dioxus from a formoxus session without
Todd asking.

**How Todd works.** He builds step by step from a to-do list with LINE
NUMBERS and hands me whole items or sub-steps; he wants design questions
raised with a recommendation, not decided for him; commit/push only when
told, no PR unless asked ([[no-pr-per-change]]).

## Dated entries (history)

**2026-10-23 (pushed as `555ebf2`, `just ci` GREEN): CHECKPOINT 4 DONE, `BuiltinWidget`
IS GONE.** Items 14–18 landed today: `InputView` routes on a private
`InputBody`, `FieldControl` is flattened into `HtmlElement` and `true_and_on`
deleted (15), and `BuiltinWidget`, `Part` and `RenderedInputType` are deleted
from `formoxus-spec` with their re-exports, tests and the transition test
(16–18, one commit; 18 was a box I added, since no box deleted the types).
`widget_matrix` now reads each widget's `ACCEPTS` through `entry::<W>()`
and no longer lists not-yet widgets (it cannot see `WidgetName::NOT_YET`).
STEP 5 IS DONE. Two boxes DEFERRED (Todd, 2026-10-23), surveys kept in
the plan: the variant picker (a bare enum already renders and
round-trips; V1–V3 open) and `#[derive(Widget)]` (DW1, where `render`
goes, open). Todd wants to USE formoxus for a while before committing to
either, so expect feedback from real use rather than a next plan step.
**`widget` was MERGED into `main` the same day (Todd), and work happens on
`main` from here.** The `widget` branch was then deleted (Todd); its commits live on in `main`'s history. Todd builds step by step from a
to-do list with LINE NUMBERS (he asked for them on 15), and hands me whole
items or sub-steps.

**2026-10-22, NIGHT (pushed, `just ci` GREEN): CHECKPOINT 3 DONE and
ITEM 13 DONE; NEXT IS ITEM 14** (`RenderedInputType`: `InputView` routes on
`InputType` directly), then 15 (flatten `FieldControl` into `HtmlElement`),
16 (re-exports), 17 (mine: `widget_matrix`, `tests/suite/builtin_widgets.rs`,
`formoxus-spec`'s own tests, the transition test in `widget_names.rs`,
README, memory). The plan's item 13 note lists exactly what still reads
`BuiltinWidget`. Item 13 became an ENUM, `formoxus_spec::WidgetName` (Todd:
"a Java enum"), one `macro_rules!` row per widget, variant = type name; the
macro matches on it exhaustively for its messages. `classes_plus:` checks a
part with a `const _` on `formoxus::classes::has_part::<W>`; its error no
longer lists the parts (accepted). Todd is still building, step by step,
and asked me to take whole items when he steps away.

**2026-10-22, END OF SCHOOL DAY (pushed, build RED on purpose):
deleting `BuiltinWidget`, checkpoints 1–2 DONE (`281f5c1`, `ad1449c`);
checkpoint 3 redesigned (Todd: widgets declare `PARTS: &[Tag]`, presets own
classes, no `default_classes()`; see
[[widgets-declare-parts-presets-own-classes]]) and MID item 11.** The
"WHERE THIS STOPPED" note under item 11 in `formoxus/FIELDS_PLAN.md` lists
the 20 remaining lib errors as 11a–11f. Start at 11a: `control_class`
lost `self.attrs.class(…)`, which would drop a field's `class:`. Todd is
building checkpoint 3 himself, walked through step by step; offer to probe
11d (a generic `#[component]`) if he hits it.

**2026-10-21, LATER: STEP 5's boxes through `field_row` DONE and pushed
(`7b37772`). NEXT: delete `BuiltinWidget`, TODD BUILDS IT** ("I'm losing
track of how things work"), from the 17-item list in `FIELDS_PLAN.md`'s
"Delete `BuiltinWidget`" box, which opens with how the four jobs fit
together. Mine: items 4, 8, 17 and goldens. Give hints and reasons when he
asks, not finished code ([[user-role]]).

**2026-10-21: "`build.rs` RESOLVES THE WIDGET ONCE", OPTION A (Todd).**
Callers move off `with_custom_widget` on a leaf rather than adapting
`WidgetType`. My items 1–4 DONE (`cdbcaea`, not pushed); Todd's 5–8 are
in the box in `formoxus/FIELDS_PLAN.md` with file:line refs. Found: no
widget type checks field type at runtime (`widget_matrix` `ok*`).
LATER: box DONE (Todd deleted `ScalarWidget` and `custom_widget`). Golden
flip was the toolchain: pinned 1.99.1 + rust-src ([[toolchain-pin-and-goldens]]).

**2026-10-20, EVENING (committed `c151c41`, NOT pushed): `form!`'s PROPS
BOX ITEMS 1–17 DONE.** `widget:` takes a lowercase built-in name or a path,
props in braces, emitted into `.with_widget`; `EXTENDS` asserts at the key;
`respan` in `form.rs` put four `DefaultField` goldens back on the field.
LATER: `RadioGroup`'s `group_class`/`input_class` props DROPPED (Todd); a
field's `class`/`class_plus` now go on its `<fieldset>`. Box CLOSED.
LATER STILL: `Field::default_widget` box DONE (required method; every
`form!` field renders through `AnyWidget`; `ScalarWidget` only for
`with_custom_widget`). Next: "`build.rs` resolves the widget once".

**2026-10-20, END OF DAY (pushed): THE BRIDGE IS DONE (`d854acb`); NEXT IS
`form!`'s PROPS BOX, and its 17-item to-do list is written into that box in
`formoxus/FIELDS_PLAN.md`.** Todd picks up items 1–12 from home (parse,
emit, check); 13–17 are mine once he reaches checkpoint 1. `FormSpec::with_widget`
exists, and `FieldNode` renders an `AnyWidget` when it has one, else
`ScalarWidget`.

**2026-10-20, AFTERNOON: STEP 5's BUILT-IN TYPES BOX DONE.**
All 18 widget types exist and render what `ScalarWidget` renders
(`tests/suite/builtin_widgets.rs`); nothing uses them yet. Todd wrote
`TextInput`, I wrote the rest (13 text-likes from one `macro_rules!`).
Components are `pub(crate)` `*View`s. Choices travel in `WidgetProps`
(Todd: not a `Select { choices }` prop). `RadioGroup`'s class props are
DEFERRED to the props box and written into it, "don't forget".
`formoxus_spec::Element` is now `HtmlElement`. **Next: `form!` props in the
widget's braces + `EXTENDS` asserts; then the goldens (mine).**

**2026-10-20, LATER: BOX ORDER CHANGED (Todd).** The built-in widget types
come BEFORE `form!`'s props-in-braces box, since the struct expression and the
`EXTENDS` asserts both need a concrete type to name. No special case for
`choices` written inside a widget's braces (unreleased; rustc's "no field"
suffices). Goldens for the props box wait until it lands. Built-in box
decided (see its entry in `FIELDS_PLAN.md`): components get a `View` suffix;
`FieldNode` resolves bounds so `WidgetProps` has no `field_type`; `Classes`
re-key deferred. `FieldControl` flattens into `Element` and `true_and_on`
goes, both in the delete box. **Next: Todd writes the first few built-in
widget types, then hands the rest to me.**

**2026-10-20: STEP 5's `Widget` BOX DONE (pushed).** Todd wrote `trait
Widget` (`NAME`, `EXTENDS`, `render(&self, props: WidgetProps)`,
`default_classes()`), the `pub(crate) DynWidget` and `AnyWidget` in
`formoxus/src/widgets/widget.rs`, from the fleshed-out box in
`FIELDS_PLAN.md`; I wrote the inline tests and the docs (the no-hooks rule on
`render`). `default_widget` returns `AnyWidget`, still `todo!()`. **Next:
`form!` takes props in the widget's braces, plus the `EXTENDS` asserts.**
Todd continues from his school computer.

**2026-10-19, EVENING: STEP 5's `AttrSet` BOX DONE, plus `step`
(two commits, `8183d4f` AttrSet then `step`; not pushed).** Todd wrote `AttrSet` (`EMPTY`, `ANY`, `valid_on(element)`,
`with`/`without`/`contains`, all const) in `formoxus-spec`; I wrote its tests.
It is exported from `formoxus::attrs`. `step` got a table row and its own
`AttrType::Step` (`"any"` or a number); `form!` refuses a fractional one on an
int field and one not above zero (see [[constraint-attributes-gap]]). Runtime
attribute values filed as a Step 8 possibility. **Next: `trait Widget`.**

**2026-10-19: W-B, W-C AND W-A's PROP QUESTION DECIDED** (in
`formoxus/FIELDS_PLAN.md` Step 5). `BuiltinWidget` is retired; class slots key
on a per-widget `const NAME`; lowercase `widget: textarea` stays as the
built-ins' spelling, custom widgets are paths. Widget types use Django's names
extended (`TextInput`, `TelInput`, …). Props are plain expressions, `.into()`.
`.into()` on all but a bare number literal (emitted as written).
The radio widget stays `RadioGroup`. Step 5's decisions are all made. **Next: Step 5's boxes, code only on Todd's say-so per piece.**
Tablet page: republish when Todd says he is heading to the tablet.

**2026-10-18, LATER: STEP 4 DONE (`38ea185`, 4 ahead of origin, not pushed);
STEP 5's W-A DECIDED, written into `formoxus/FIELDS_PLAN.md`.** A widget is a
struct of PROPS like a Dioxus component (`pub` fields, set in the widget's
braces in `form!`, checked by rustc), `const EXTENDS: AttrSet` for pass-through
HTML attributes, `#[derive(Widget)]` with `render` by hand (shape 1), a hidden
`DynWidget` + `AnyWidget` newtype. Parts shrink to CLASS SLOTS; per-part
attribute addressing (D0 3–5) dropped. Step 5 = single-control widgets only;
multi-control (`ContactInfoWidget` set, `DateWidget` split) is new Step 10.
Step 6 gained `Field::format` and "a named field wins over any shape".
**Next: W-B and W-C.** Tablet page not yet republished with this.

**2026-10-18: STEP 4's CONSTRAINT MOVE DONE (`a40278b`, 3 ahead of origin, not
pushed).** Four fields in `fields/`; `form!` emits `with_node(…, field_for(proj,
|__f| { … }))`; `DefaultField` maps model type → field; `check_attrs` gone;
`with_attrs` refuses validated keys. Todd leans on me more, but I write library
code ONLY when he asks for a specific piece (Todd, 2026-10-18: "Don't assume."). **Next in Step 4: wire
`ChoiceField` (typed `choices:`, separate change, decision C).**

**2026-10-17, END: STEP 3 DONE, S-b INTEGRATED.** The walk applies the
whole spec as it builds; `apply_specs`/`distribute_specs` deleted; PR #12
merged, `spike/s-b` and its worktree deleted. **Next: Step 4** (the four
fields; decisions A-1 and B-1 come up there).

**2026-10-17, LATER STILL: S-b SPIKED, IT HOLDS.** Branch `spike/s-b` in
worktree `../formoxus-spike-sb` (uncommitted; build dir `target-spike`).
Results and findings under Step 3 in `formoxus/FIELDS_PLAN.md`. Plan page
for Todd's tablet: https://claude.ai/artifact/Nj88w7AgabvyTygAvVUFtV
(republish from the scratchpad copy, or rebuild it, when the plan changes).

**2026-10-17 (home), LATER: STEP 2 DONE AND COMMITTED.** `FieldNode<F>`
(`field_node.rs`) + the `ScalarField<T>` stand-in; `fields.rs` deleted;
its 52 inline tests ported (20 → `field_attrs.rs`, 32 → `field_node.rs`);
`#[expect(clippy::todo)]` on `default_widget` until Step 5. **Next: Step 3,
decision S (S-a vs S-b; I lean S-b, spike a nested `Vec` row first).**
Todd wants to read the plan from the mobile app.

**2026-10-17 (home): THE REFACTOR HAS A PLAN FILE: `formoxus/FIELDS_PLAN.md`.**
Read it first; it supersedes the step list below and gathers the decisions,
the open questions (F-A..F-C, S, A-1, B-1, W-A..W-C, E-1, review note C)
and what it replaces in `ATTRIBUTES_PLAN.md`'s Step 4. Todd finished Step 1
(uncommitted); the one remaining error is `dispatch!`'s old struct literal,
which Step 2 fixes. Also decided today: compile-time field checks are
EMITTED `const _` (not spec tables); `raw` in `FieldValue::Invalid` kept
for now, queued for deletion (both in [[django-parallel-design]]).

**2026-10-16 (school, ~5pm), END: WIP, BUILD BROKEN ON PURPOSE.** Committed
with `--no-verify` (format hook). Todd is writing; I review. State:
- `formoxus/src/field.rs` (new): `trait Field` with `type Value`,
  `type Context`, `parse` (default `parse_scalar`, moved here), `check`,
  `widget_attrs`, `default_widget(optional) -> Box<dyn Widget>`,
  `async fn check_on_server`; a placeholder `trait Widget {}` (TODO: move).
- `formoxus/src/field_attrs.rs` (new): ONLY the `FieldAttrs` struct so far.
- `fields.rs` is ENTIRELY commented out (`/* … */`, including its tests);
  the old `Field<T>` struct and impls are there for reference while
  `FieldNode<F>` gets written.
- The 10 errors are all one cause: `FieldAttrs`'s methods (`get`, `contains`,
  `without`, `with_default`, `to_attributes`, `merge_with_attrs`, `class`,
  `with_owned_attrs`, `From<[…; N]>`) and `populate`/`Field` are still inside
  the commented-out `fields.rs`; importers point at `crate::fields::…`.
**Review notes to raise with Todd (my first read, not yet discussed):**
1. `async fn check_on_server` has no default, so every field must write
   it; give it one returning no messages.
2. `async fn` in a public trait: the future has no `Send` bound
   (rustc's `async_fn_in_trait` lint), and server handlers need `Send`.
   Likely `fn check_on_server(…) -> impl Future<Output = …> + Send`.
3. `type Context` on `Field` is the big one: Context belongs to the FORM
   (`FormDef::Context`), but a field like `ProgramSnippetField` is written
   once and used in many forms, and one form mixes fields. Must a field's
   `Context` equal its form's? Options: fields generic over the context
   (`impl<C: HasDb> Field<C>`), a `FromRef`-style projection (axum), or
   Context only reaches server checks through `FieldNode` by `&dyn Any`.
4. Unused `value` in default `check` warns; `_value`.
5. Next after the build: `FieldNode<F>` and `impl<F: Field> Node for FieldNode<F>`,
   then `StringField`, `NumberField<T>`, `BoolField`, `ChoiceField<T>`,
   then `build.rs`'s `dispatch!`.

**2026-10-16 (school, end of day): FIELDS AND WIDGETS BOTH BECOME TRAITS.**
Read "Decided, 2026-10-16 afternoon" in [[django-parallel-design]] first:
`Widget` trait + Django-style widget types; `Field` trait + `NumberField<T>`,
`TextField<T>`, typed `ChoiceField<T>`; `field:` in `form!` plus a type →
field registry (subsumes the widget registry); async server-only validation
IN scope. `FormField` → `Field` rename done (it is still the one generic
struct; the trait will take the name, so the struct gets renamed or split).
Todd's `WidgetProps` WIP is in `widgets/widget_props.rs`, not yet used.
Todd is doing the code until he hands off.

**2026-10-16 (school, later): THE REFACTOR HAS STARTED (uncommitted on
`widget`).** Todd renamed `members` → `nodes`, `FormMember` → `Node`,
`RenderCtx` → `NodeContext`, `OptionMember` → `OptionNode`; I swept the
`*_member` builders, comments, README, CLAUDE.md and `ATTRIBUTES_PLAN.md`
(historical plans and memory left as is; renames line in `MEMORY.md`). He
deleted the `Field<T>` sketch and `contact_info` stub ON PURPOSE. Builds
clean. **Next, one radical change ([[feedback-radical-refactors-on-a-branch]]):**
fold `FieldProps` + `WidgetProps` + `FieldAttrs` + `ScalarWidget` props into
ONE `WidgetProps` (`id, path, label, attrs, extra_attrs, values, errors,
classes`); choices move onto the widget value (`Select { choices }`, Django
style); fields write their own constraint attrs (so `FieldType` leaves the
props); widget resolved once in `build.rs` (no `custom_widget: Option`);
`ErrorsStore` IN SCOPE NOW (client projection, one writer, `FormState` stays
the authority); variant picker = a choice widget that still fires
`on_edit`. **Todd is doing ALL of it himself** (committed rename = `a8c3263`)
until he hands off; stay out of library code unless asked. **Decided:**
Django-style widget TYPES (`TextInput`, `HiddenInput`, `Textarea`, …)
implementing a `Widget` trait with at least `render`, replacing the
`WidgetType`/`BuiltinWidget` enum + `Custom` fn pointer. Raised to him:
associated consts make a trait non-dyn (so likely a dyn `Widget` + a const
`WidgetKind` for `form!`'s checks), `Send + Sync` + `clone_box` for
`FormState`, class slots keyed by widget name, one type per input `type`,
and that `formoxus-spec` must keep knowing built-ins for compile-time
checks. Likely absorbs part of Step 4 (D0–D4).

**2026-10-16 (school, day), DESIGN TALK, NO LIBRARY CODE.** Still on
`widget`. The `Field<T>` sketch DOES build now (only an unused `FieldSpec`
import warns); the "does not parse" claim below was stale. Todd answered
A–F; see "Answers, 2026-10-16 day" in [[django-parallel-design]]. Open
thread: whether to merge `FieldProps` + `WidgetProps` + `FieldAttrs` (+
`choices`) into one `BoundField`, keeping `RenderCtx` as the tree-walk
context. **Done:** the dev/user vocabulary is on `main` (`4d9db06`, pushed).
**Still `widget`-only, ON PURPOSE (Todd: hold until the design settles):**
`django_parallel_design.md` and its `MEMORY.md` line. Raise it if `widget`
is merged, rebased or abandoned. An untracked 2-line stub
`.claude/memory/django-parallel-design.md` (hyphens) appeared in the
checkout; origin unknown, left alone; ask Todd before deleting.

**2026-10-16 (home, late), END: DESIGN TALK ONLY, NO CODE.** Branch
`widget` (not `main`), pushed. Todd is spit-balling a closer Django parallel;
nothing is decided. **THE GOAL IS SIMPLIFICATION: fewer, combined concepts
(see "THE GOAL" and the name inventory in that note).** Read
[[django-parallel-design]] first.

**2026-10-15 (school), END: P3 DONE (P3-7, P3-8, P3-9), #10 CLOSED, ALL
PUSHED.** P3-8: rendered cascade tests in `tests/suite/classes.rs`, `bare()`
tests in `snapshot.rs`. P3-9: `/t/bare` + `e2e/tests/bare.rs`. **Next: Step 4
in the plan (emit only valid attributes, the `Widget` trait), opening with
decisions D0–D4; 4d now also carries the parked widget-boilerplate idea.**

**2026-10-15 (school), LATER: P3-7 DONE AND COMMITTED (`2a7d82d`); the
form title is now a `div` + `aria-labelledby` on the `<form>`, not an `<h2>`.
Next: P3-8 (tests, Claude), then P3-9 (wrap-up). The widget-boilerplate
refactor is parked under Step 4d until the `Widget` trait exists.**

**2026-10-15 (school): pulled; `radio-group` branch deleted (local + origin,
fully merged). P3-7-A..D DECIDED as proposed. Next: P3-7a (`FieldAttrs::class`).**

**2026-10-14, END OF SESSION (home) — GREEN, PUSHED: P3-6 DONE; P3-7 BROKEN DOWN, NEXT.**
Todd picks up at SCHOOL on 2026-10-15: `git pull` first.
P3-6: `Rc<Classes>` on `RenderCtx` (built by `Form::root_ctx()`) and on
`FieldProps`; `FieldErrors`/row buttons take a `classes: Rc<Classes>` prop;
`field_label`/`required_marker` take `&Classes`. `render` is now the ONLY
public render: `Form::render_fragment`/`render_title`/`render_fields`/
`render_errors` DELETED (suite's 99 calls → `render(using_fns! {})`), and
`FormState`'s pieces are `pub(crate)`. Widgets use `props.x`, no more
`let FieldProps { … }`. P3-6f test: a custom widget sees the form's resolved
classes. **Next: P3-7 (replace the 37 literals), opening with decisions
P3-7-A..D in the plan; Todd wants to "whiz through" it.** Gotcha: in an
rsx! format string, `"{x.unwrap_or_default()}"` can be evaluated twice; unwrap
into a local first.

**2026-10-14, END — GREEN, PUSHED: P3-4b AND P3-5 DONE (`classes:`/`classes_plus:` in `form!`).**
Walkthrough steps 1–7 all done (parser + expansion in
`formoxus-macros/src/form/classes.rs`, 8 `form_classes_plus_*` goldens, 6
cascade tests in `tests/suite/classes.rs`). `Form::classes()` resolves the
cascade, but NOTHING RENDERS THROUGH IT YET. **Next: P3-6 (Todd, starting):
`Rc<Classes>` on `RenderCtx`; then P3-7 replaces the literals, snapshot-guarded.**
Todd likes to see the diff after each step of a multi-step change.

EARLIER THE SAME DAY: **2026-10-14, LATE — UNCOMMITTED, BUILDS: `classes_plus:` GRAMMAR IN PROGRESS, STEP BY STEP.**
Todd asked to see the diff after EACH step of the walkthrough (stop and wait).
Done since `c07fccf`: P3-4b (`ButtonType`/`Invocation`/`FixedKey`/`SlotKey` in
formoxus-spec; `formoxus::attrs` an explicit list WITHOUT `SlotKey`);
`ClassEdit` = `{ key, base: Option<Change>, invalid: Option<Change> }` built
`ClassEdit::part(w, p).replace_base(…).append_invalid(…)`; `Classes::with_edit`;
`Form::classes()` folds `FormState::class_edits()`. Todd wired `Entry::Classes`/
`ClassesPlus` + `FormSpecMeta`. Grammar DECIDED: values use field-level list
syntax `[a, "b"]`; `+` is PER PIECE (`{ base: [x], invalid: + [y] }`); short
form touches base only; `+ []` refused, `[]` clears; not-yet widget parts
refused. Walkthrough steps: 1 `class_list` shared (DONE), 2 keywords+dispatch
(DONE; `parse_classes_plus` is a placeholder error), **3 `parse_slot_key`
NEXT**, 4 value, 5 block (+ `ClassEntry` reshaped: `append` goes), 6 expansion
(also emit `.with_classes`), 7 tests + goldens + suite.

**2026-10-14 — GREEN, UNCOMMITTED: P3 DECISIONS DONE, P3-1 AND P3-2 DONE, P3-3 FLESHED OUT.**
P3-c..h all decided as proposed (part typo PANICS). P3-1: `widgets/label.rs`
holds the only `field_label`/`required_marker`. P3-2: `tests/suite/snapshot.rs`
pins whole-form markup (clean / field errors / form errors + problems list)
in `tests/snapshots/*.html`; regenerate with `SNAPSHOT=overwrite`, then READ
the diff. It is the safety net for P3-7. P3-1/P3-2 PUSHED (`ecfaca9`).
LATER 2026-10-14 (uncommitted): P3-3 and P3-4 DONE: `classes.rs` has `Slot`,
`FixedKey` (renamed from `FixedSlot` by Todd: it is a key, not a slot),
`Classes` (private `IndexMap<SlotKey, Slot>`), `fx()`/`bare()`/`Default`, the
builders, the lookups; 23 unit tests + `tests/suite/classes.rs`. Todd also
made the problem `<li>` `fx-form-error fx-problem` (snapshot regenerated).
NOTHING renders through `Classes` yet. PUSHED `c07fccf`.
THEN P3-f REVERSED (Todd): a form LAYERS on the app's classes with
`classes_plus: { widget.part: "…", fixed_key: "…", x: + "append" }`,
compile-checked; `classes: EXPR` still replaces whole; both allowed, in that
order. So `ButtonType`, `Invocation`, `FixedKey` move to `formoxus-spec`
(P3-4b, who-does-it open). **Next: P3-4b, P3-5 cascade (reshaped: `FormSpec`
holds `Option<Classes>` + ordered `ClassEdit`s), P3-6 `Rc<Classes>` on
`RenderCtx`, P3-7 call sites.**

**2026-10-13 (home) — GREEN, PUSHED; P2's `BuiltinWidget` BOX AND ITS TESTS DONE.**
`formoxus-spec/src/widgets.rs`: `BuiltinWidget` (enum; `ALL`, `from_form_name`
via `str_eq`, `form_name`, `parts`, `form_control`, `takes_choices`,
`accepts_optional`, `renders`, `default_for`, `is_rendered`), `Part`, and
`Element::tag()` (Todd). Gotcha: a `const fn` CALL is never promoted to
`'static`, so `parts()` wraps each slice in `const { }` (`labelled!`/`grouped!`),
and each text-like `Input` gets its own arm. 13 unit tests, mutation-checked.
LATER SAME DAY, ALSO PUSHED: the MACRO SIDE (Claude, delegated): string tables
gone, `WidgetKind::Named { name, widget }`, messages stay in the macro (Todd:
fine, they are `form!`-vocabulary prose), `field_kind::renders` takes a
`BuiltinWidget` and `WidgetClass` is gone. Goldens unchanged. The macro still
emits `WidgetType::X`; it becomes `WidgetType::Builtin(…)` with the runtime
box. THEN THE RUNTIME SIDE, ALSO 2026-10-13 (uncommitted at time of writing):
`WidgetType = Builtin(BuiltinWidget) | Custom`, `Copy`, `From`, `Debug` prints
`form_name()` (Todd, R1/R2/R4); `ScalarWidget` asserts `is_rendered` then
`renders`, then an exhaustive match picks the component (Claude, R3); R5–R7 and
the moved tests (Claude). `just ci`, e2e 31, `widget_matrix` green.
**P2 IS DONE. Next: P3 (configurable classes); its checklist is not broken down.**

**END OF SCHOOL DAY 2026-10-12 — GREEN, PUSHED; P2 BOXES 1–4 DONE.** Home: `git pull` first.
`RenderedInputType` (formoxus-spec, `to_input_type()`); `Input` routes
hidden/checkbox/text-like (checkbox.rs folded into input.rs, `field_label`
shared); `WidgetType::Checkbox` gone; `form!`'s `checkbox` → `Input(Checkbox)`
with `rule()` matching it by name (stopgap). Green: spec 18, macros 136, lib
98, suite 346, goldens unchanged, e2e 31. **Next box: `BuiltinWidget` (Todd).**

**2026-10-12 (school), MIDDAY — GREEN, PUSHED; P2's FIRST TWO BOXES DONE.**
Todd wrote `formoxus-spec/src/elements.rs` (`Element`, NESTED:
`Div | Fieldset | Label | Legend | Control(FieldControl)`; `FieldControl` =
`Input(_) | Textarea | Select`) and keyed `is_valid_on` on `Element`. Claude
rewrote the 95 `on:` rows, moved callers and tests, and sorted the 73
all-four rows: 70 global (`on: _`), 3 form-only (`disabled`, `form`, `name`,
still `Control(_) | Fieldset`, spec-true). Todd reviewed. Green: spec 17,
macros 136, lib 98, suite 346, clippy, fmt (e2e not rerun). **Next box: the
rendered-input-type enum (Todd).** Todd may next work from HOME: pull first.

**END OF SESSION 2026-10-11 (home) — GREEN, PUSHED; NEXT IS P2.** Todd picks
up on his SCHOOL machine: it needs a `git pull` (this session pushed
`d6b5197`..HEAD), and the new `dioxus-fullstack-core` dependency means a fresh
build. That machine also still holds an old `stash@{0}` (2026-10-08); leave it.
Done since the last handoff: the `formoxus-attrs` → `formoxus-spec` rename;
widget PARTS adopted; P1 (`<label for>`, random form id via
`use_server_cached`, `aria-describedby`, the enum select named by its legend)
built, tested (suite 346, lib 98, e2e 31) and #9 closed. **Next: P2 in
`formoxus/ATTRIBUTES_PLAN.md`, broken into a checklist, opening with two
decisions for Todd: P2-a, DECIDED the same evening (option C: a flat
`Element` for the table and parts, `FieldControl` narrowed to the form
controls, `From` between them, `Fieldset` no longer a control), and P2-b, ALSO
DECIDED (fold decision 5 in now: `Input` routes every `<input>`,
`WidgetType::Checkbox` → `Input(Checkbox)`). Nothing built; Todd starts with
`Element`/`FieldControl`, Claude with sorting the 73 all-four table rows.** Then P3 (class config), then Step 4.

**2026-10-11: P1 (`<label for>`, #9) DONE and committed.** Random form id via
`use_server_cached`, every widget un-nested, `aria-describedby`, the enum
select named by its legend; suite 345, lib 98, e2e 31 green. #9 closed, P1
fully done. NEXT is P2, `BuiltinWidget` in `formoxus-spec` (Todd).

**2026-10-10 (home), LATER:** Todd pulled #9 and #10 ahead of Step 4: the plan's
"Before Step 4" is P1 `<label for>` un-nesting, P2 `BuiltinWidget`, P3 class
config (system + `fx()`/`bare()` only). All decided the same day; the form id
is passed at RENDER (`Option` instance number), never declared in `form!`.
Ids: a RANDOM 8-char base62 form id + path, picked in `use_form` via
`use_server_cached` (spiked and proven across hydration). Nothing built.

**2026-10-10 (home):** Todd ADOPTED widget PARTS (D0), parts named by their
HTML element, every widget has a `label` part; see `attribute_rules_design.md`.
Same day: built-in widget DESCRIPTIONS move into the shared crate, renamed
`formoxus-spec` (first step of 4a, now). Open points A–E are worked as
`form!` → HTML examples in `formoxus/WIDGET_PARTS_EXAMPLES.md`.

**END OF SCHOOL DAY 2026-10-09 — GREEN at `12cb3c3`; STEP 4 BROKEN DOWN, NOT STARTED.**
Done today: typed keys on enum fields (`Kind::Enum`, enum-specific messages,
`is_enum` asks `kind` because facet types `Option<T>` as an enum, see
[[facet-option-is-an-enum]]). Step 4 is now a checklist in
`formoxus/ATTRIBUTES_PLAN.md` (4a–4e) opening with decisions D1–D4.
**Todd is THINKING about a reframe, nothing decided:** treat `RadioGroup` as
the first multi-element widget (cf. Django's MultiWidget). Claude's sketch: a
widget declares named PARTS, each with a control (`group: Fieldset`,
`option: Input(Radio)`, one marked as the value part); validated attributes go
to the value part (dissolves D1), unvalidated ones to the primary part unless
addressed (`radio_group { choices, option: { class: [...] } }`), checks run per
part; custom widgets declare `PARTS` in the trait; formoxus's own attributes
(`aria-invalid` on the group) stay placed by the widget. Kept OUT: value
decomposition (MultiWidget's real job: several names per value), wrapper parts
(label/errors, overlaps #10), per-choice attributes. If he adopts it, 4a becomes
"every field gets its parts" and D4's `AttrSet` becomes "what a `Part` holds".
WRITTEN UP 2026-10-10 at his request: `attribute_rules_design.md`,
"PROPOSED, NOT DECIDED: widgets have PARTS", and D0 in the plan. Not decided;
it has five open points for him, and D0 goes before D1–D4.

**END OF SESSION 2026-10-09 (home) — STEP 3 FULLY DONE, `874e04c`, GREEN.**
Next (Todd): split the enum out of `field_kind`'s `Kind::Other` so typed keys
with `validated: false` are allowed on an enum field (his call: "allow them if
they make sense"); then Claude moves `class_and_class_plus_on_the_variant_select`
to `form!` and adds a golden refusing a constraint key on an enum. After that,
plan Step 4 (emit only valid attributes, `Widget` trait), not started. The
plan's status line at the top of `formoxus/ATTRIBUTES_PLAN.md` says the same.

**UPDATE 2026-10-09 (home) — 3d's WIDGET SIDE DONE (steps 4–10), GREEN.**
Steps 6–10 of `formoxus/ATTRIBUTES_PLAN.md`: `required`/`aria-invalid` are map
entries built by `FieldAttrs::with_owned_attrs` at render; `FieldProps` is
`path`/`label`/`errors`. `RadioGroup`: `required` on each radio, `aria-invalid`
on the fieldset with `role="radiogroup"` (ARIA 1.2 defines it on `radiogroup`,
not `radio`). `FormMember::has_errors` split into `has_errors_within` /
`has_errors_here` ([[errors-here-vs-within]]). Issue #7 got a comment mapping
its plan onto this. Open: `RadioGroup` author attributes (Todd floated a
`form!` grammar; Claude suggested `radio_group { choices, each: {…} }`, field
keys on the fieldset; DEFERRED by Todd 2026-10-09); custom widgets get no author
`class`. Green: 587 tests, e2e 29, fmt, clippy, rustdoc.

**UPDATE 2026-10-08, EVENING (home) — STEPS 4 AND 5 DONE, NEXT IS STEP 6 (Part C).**
Step 5 (style, `"; "` join): Todd; 5 tests, Claude.
Todd built `FieldAttrs::class` and the five widgets' literal `class`, plus
`VariantSet.field_attrs` (enum-field attributes reach the `<select>` now).
Claude: `matches!`/`if let` tidy-up in `fields.rs`, 7 class tests
(mutation-checked), plan ticked. Open: an author `class` reaches neither
`RadioGroup` nor a custom widget (pinned for the radio group). Committed and pushed
together with step 5. Green: 580 tests, fmt, clippy, rustdoc.

**UPDATE 2026-10-08, END OF SCHOOL DAY — BUILD GREEN, NEXT IS STEP 4.**
The widget-side work is now a numbered checklist under 3d in
`formoxus/ATTRIBUTES_PLAN.md` ("Widgets build their own attributes"); Todd
asked for it in a FILE because he hates scrolling up for a plan. Part A
(steps 1–3) is done (`1d1722b`): widgets take the typed `FieldAttrs` and call
`FieldAttrs::merge_with_attrs`. `required`/`aria_invalid` leaving `FieldProps`
was folded in (steps 6–10). **Next is step 4, `FieldAttrs::class(base)`
(Todd).** Also today: a suite test that a custom widget receives formoxus's
attributes (mutation-checked); `form!`'s `custom(…)` still drops `attrs`
(main plan Step 4); the Widget trait must make widgets declare default
classes, fn vs const open (design note). Morning: Todd made a mess, it was
STASHED (`stash@{0}` on the school machine only), not discarded. Green: lib
86, suite 328, macros 136, attrs 17; fmt, clippy, rustdoc clean.

**UPDATE 2026-10-07, END OF SESSION (home) — BUILD GREEN, 3d PARSING DONE.**
The `form!` side of 3d is finished and pushed: class lists (Todd) and style
blocks (Claude, delegated) parse bare-or-quoted names with `_` → `-`, bare
numbers, `!important`, and a set of targeted errors (all listed in
`formoxus/ATTRIBUTES_PLAN.md` 3d's checklist). `AttrType::List` is gone and
the `todo!` is `AttrValue::List(&[#(#items),*])`. Unknown-key messages no
longer list ~90 keys. 7 new goldens. Green: macros 136, lib 86, suite 327,
goldens; clippy and fmt clean (e2e not rerun). **Next is Todd's: attribute
conversion moves into the widgets** (Todd's call 2026-10-07: convert to
`Vec<Attribute>` inside the widget, which knows its base class; a spread
`class` duplicates, PROBED). Until then a style renders space-joined
(`color: red font-size: 20px`): `Style`/`StylePlus` need `"; "`. `RadioGroup`
class placement is ON HOLD (Todd wants examples; maybe `group_class` /
`input_class`). Then Claude: the suite test pinning ONE `class` attribute,
`author_attrs.rs` onto `form!` keys, dropping the `takes_*` wrappers.

**UPDATE 2026-10-06, END OF SCHOOL DAY — BUILD GREEN, 3d IN PROGRESS.**
3b, 3c and 3e of `formoxus/ATTRIBUTES_PLAN.md` are done and committed (see the
plan's DONE notes); every test passes (macros 111, lib 86, suite 327, goldens,
e2e 29). Todd has started 3d: `AttrValue::List(&'static [&'static str])` and
`AttrType::List` exist, and `html_attributes` has a `List` arm. **Todd's call
(2026-10-06): do NOT pull `class`/`class_plus` out of the generic attribute
list; handle them inside the `html_attributes` loop** (this replaces plan
step 12's `FieldProps` field). The one clippy failure left is the `todo!` in
`ParsedAttr::entry_tokens` for list values, which 3d replaces with
`AttrValue::List(&[#(#items),*])`. Then Claude does the rest of 3f (new
goldens; `author_attrs.rs` onto `form!` keys). `label` is an `Expr` again.

**UPDATE 2026-10-05, END OF SESSION — WIP COMMITTED, THE BUILD IS BROKEN ON
PURPOSE.** Todd is mid-way through **step 3b of `formoxus/ATTRIBUTES_PLAN.md`**
(the living checklist, with files and line numbers; read it first). He is
replacing `field_body!` in `formoxus-macros/src/form/field.rs` with a
hand-written `FieldBody { widget, label, attrs: ParsedAttrs }`, so
`LEGAL_KEYS` and the per-key fields (`min`, `max`, …) are gone and their users
(`constraints_tokens`, `type_checks`, `Parse`, the unknown-key message) do not
compile yet: about 23 errors, all in `field.rs`. `cargo fmt` also flags that
file. The WIP commit used `--no-verify` for that reason.
Done before this: steps 1–2 (`69a9225`), the 95-row table (decision 15 in
`attribute_rules_design.md`), `indexmap` added to `formoxus-macros` (workspace
entry now featureless; `formoxus` adds `serde`), `variant_name` /
`from_variant_name` in the table (Todd). Decisions for 3b, all recorded in the
design note (13–15): `ParsedAttrs(IndexMap<AttrId, ParsedAttr>)`,
`AttrId { Std(Attr), NonStd(String) }`, quoted keys pass through, keys are the
snake_case variant name, `class_plus:`/`style_plus:`, `AttrSource::List`
holding `Vec<LitStr>` from a bracketed list (Claude's suggestion, 2026-10-05).
Pending reminders: `AttrValue` list variants (plan 3a/3d); the two
`LEGAL_KEYS` tests are Claude's to move to the table once parsing compiles.

**UPDATE 2026-10-01: issue #6 is BUILT** (see [[formoxus-feature-parity]] 6b): `required: true` on a non-optional bool, rule in `ValueKind::check` reached via `raw_value_to_validate`, compile gates, `Checkbox`-only `required_true` prop for the marker, suite + trybuild + 5 e2e tests; 509 tests + 29 e2e green. Next in [[mvp-scope]] is C6. The rest of this note is the 2026-09-30 handoff.

**Written 2026-09-30, end of day, for the next session.** Todd asked: "write
whatever the next version of you will need to know where we're going."

## Where the repo is

- `main` at **`3f04386`**, pushed, clean apart from memory edits written after
  it (this file, and the reorder in [[mvp-scope]]) — those are UNCOMMITTED and
  ride along with the next code commit, per the no-standalone-memory-commit
  rule. If Todd is on his school machine, they will not be there; offer to push.
- Green at that commit: 487 unit/integration tests, 24 e2e (`just e2e`), `fmt`,
  clippy (pedantic set), rustdoc — all clean, zero warnings.
- **MVP step 1 is DONE.** Every pre-publish breaking change landed this week:
  `Choice` (moved to `widgets/types.rs`), the `fx-` CSS vocabulary
  ([[css-class-vocabulary]]), `ButtonType::Action` + `form!` keyword `action`,
  `structure.rs` split into `variant_select.rs`/`row_buttons.rs`,
  `ValuesStore` (the store) vs `ValuesByPath` (plain `IndexMap` data),
  `FieldProps::aria_invalid` + `field_class()`/`field_class_plus()`,
  `ValidationError<T>` + `ValidationMessage` ([[error-model-design]]), the
  `distribute_errors` claiming walk, `collect_*`/`distribute_*` method pairs,
  and the prelude as the one export list ([[prelude-is-the-one-list]]).

## What is next: issue #6

A checkbox cannot be required to be TICKED — "I agree to the terms" submits
unticked and formoxus accepts it. Recorded design (in
[[formoxus-feature-parity]], item 6b): a value constraint — `ValueKind::Bool`
gains a payload, plus `check` + `attrs` + a compile-time gate. **NOT** forwarding
`required`: formoxus's `required` means PRESENCE, and that sense is load-bearing
(it drives `Select`'s placeholder branch and `RadioGroup`'s refusal of an
`Option`), so redefining it for bools would make one word mean two things.
`Checkbox` deliberately drops `required` today, with a comment saying so, and
renders no ` *` marker. Start by scoping it for Todd, not by writing it.

Then the revised order in [[mvp-scope]]: C6 author attributes (with Todd's
`class:` replace / `class+:` append idea — open questions in
[[css-class-vocabulary]]), #9 `aria-describedby`, C5 `help_text`, C4 per-field
validators, the facet version, `form!` grammar docs, the doc pass.

## Parked, Todd wants to come back to them

All in [[parked-validate-values]]: `FormState::validate_values` (make
distribute→validate hard to get wrong); `collect_values` building a
`ValuesByPath` directly; and **typed path keys instead of strings**. The last
one is BREAKING, so it is the one parked item that cannot drift past the first
publish — raise it when publishing comes up, at the latest.

## Open issues

#4 (emit only valid attribute/element pairs), #6 (next), #7 (`aria-invalid`
third state), #8 (warnings), #9 (`aria-describedby`), #10 (configurable class
names). #11 closed this week.

## How Todd works — read these memories, they are not optional

- [[user-role]]: Todd writes the core library code; I write tests, mechanical
  fixes, plans, memory, and investigate. He edits the SAME files concurrently —
  re-read before patching, and check mtimes if errors look unfamiliar (it
  happened twice on 2026-09-30).
- [[feedback-show-scope-before-writing-code]]: say what would change and where,
  then wait — unless he explicitly delegates ("go ahead", "can you fix",
  "handle that").
- [[feedback-numbered-not-bulleted]]: in replies, NUMBER what he should act on,
  LETTER what he may respond to, no bullets.
- When a fix is mechanical and he delegates, verify with the full set — tests,
  `fmt`, clippy, rustdoc — and mutation-check new tests (break the code, watch
  the test fail, restore byte-for-byte).
