---
name: next-session
description: "HANDOFF written 2026-09-30 at Todd's request for a fresh session the next day: where the repo is, what is next (issue #6), what is parked, and how Todd likes to work. Read this first, then delete or update it once the session has picked things up"
metadata:
  type: project
---

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
box. **Next box: the runtime side, `WidgetType` (Todd, starting).** Its
`ScalarWidget` panics still duplicate two `form!` messages; deal with them there.

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
