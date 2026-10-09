---
name: next-session
description: "HANDOFF written 2026-09-30 at Todd's request for a fresh session the next day: where the repo is, what is next (issue #6), what is parked, and how Todd likes to work. Read this first, then delete or update it once the session has picked things up"
metadata:
  type: project
---

**UPDATE 2026-10-09 (home) — 3d's WIDGET SIDE DONE (steps 4–10), GREEN.**
Steps 6–10 of `formoxus/ATTRIBUTES_PLAN.md`: `required`/`aria-invalid` are map
entries built by `FieldAttrs::with_owned_attrs` at render; `FieldProps` is
`path`/`label`/`errors`. `RadioGroup`: `required` on each radio, `aria-invalid`
on the fieldset with `role="radiogroup"` (ARIA 1.2 defines it on `radiogroup`,
not `radio`). `FormMember::has_errors` split into `has_errors_within` /
`has_errors_here` ([[errors-here-vs-within]]). Issue #7 got a comment mapping
its plan onto this. Open: `RadioGroup` author attributes (Todd floated a
`form!` grammar; Claude suggested `radio_group { choices, each: {…} }`, field
keys on the fieldset; questions 3–4 unanswered); custom widgets get no author
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
