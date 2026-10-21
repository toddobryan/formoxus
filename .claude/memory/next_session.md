---
name: next-session
description: "HANDOFF written 2026-09-30 at Todd's request for a fresh session the next day: where the repo is, what is next (issue #6), what is parked, and how Todd likes to work. Read this first, then delete or update it once the session has picked things up"
metadata:
  type: project
---

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
