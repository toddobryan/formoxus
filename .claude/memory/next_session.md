---
name: next-session
description: "HANDOFF written 2026-09-30 at Todd's request for a fresh session the next day: where the repo is, what is next (issue #6), what is parked, and how Todd likes to work. Read this first, then delete or update it once the session has picked things up"
metadata:
  type: project
---

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
