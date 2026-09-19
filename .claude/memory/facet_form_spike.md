---
name: facet-form-spike
description: "The facet-based runtime-reflection alternative to formoxus's derive macros — where it lives (crates/formoxus/src/reflect), what's proven, and what's next, as of apcsp-dioxus 344347b on branch facet (2026-09-13)"
metadata:
  node_type: memory
  type: project
---

**Where (as of 2026-09-10): `crates/formoxus/src/reflect/`, on the apcsp-dioxus
branch `facet`** (`344347b`, 153 reflect tests + 25 macro tests / 47 workspace targets). MERGED — it is no longer a standalone project. The
old repo `~/code/rust/facet-form-spike` / <https://github.com/toddobryan/facet-form-spike>
still exists at `3063e28` and has NOT been deleted — kept locally and on GitHub until
we're sure it isn't needed. **But it is CLOSED: Todd stopped work there on 2026-09-06.
All further reflection work goes to the `facet` branch of apcsp-dioxus, never to the
spike repo.** Deleting it is Todd's call to make later. See "Endgame" below for how the merge went, and
[[facet_form_design_decisions]] for the reasoning behind each design choice, which
is the part that's expensive to re-derive.

**Why it exists.** The macro-based formoxus hit a wall on `Question`/`QuestionData`
(see [[formoxus_roadmap]]): `#[form(field_set)] data: T` requires `T::State::Model == T`,
but `#[field_set(model = QuestionData)]` requires `T::State::Model == QuestionData`.
Mutually exclusive for the same `T` used the same way — not fixable by tweaking bounds.
Todd's reframe: instead of hand-writing a Form struct and converting it to the Model,
start from the Model and derive the form from its *shape*. `facet` (0.46.5) provides
that shape at runtime via `Peek` (read) and `Partial` (build).

**Dependency weight:** small and modular — 2-3 direct deps, ~1-5MB/90K SLoC total,
and its derive uses `unsynn` rather than the `syn`/`quote` stack, so it doesn't stack
onto the proc-macro cost formoxus-macros already pays. Pre-1.0 (0.46 stable), API
still moving — that churn is the real risk, not size.

## What's proven (71 passing tests, none ignored)

The whole path works end to end for **scalars, structs, enums, and (edit-mode) `Vec`s**:

- `empty_form::<T>()` walks `T::SHAPE` into empty members (infallible); `form_for(&t)`
  walks the same shape but populates each field through `Peek`.
- Required-vs-optional falls out of the model itself via `Def::Option`.
- `leaves()` → `Vec<(qualified_path, raw_value)>`; `apply(&HashMap)` takes them back.
- `validate()` → each member's `write_into` threads a `Partial` → `build().materialize::<T>()`.
- `T -> String -> T` is the identity for every built-in scalar (`tests/roundtrip.rs`).
- Enum variants are answered in-form via `choose_variant`, not supplied at construction.
- Three widget approaches now tried. The **chosen** one is `Form<T>` as the schema beside
  a `Store<HashMap<path, String>>` for live values — prototyped, with per-path
  re-rendering verified. The fully **uncontrolled** form (no signals, `FormData::values()`
  on submit) still works and is still tested, but is no longer the plan; the
  one-`Signal`-per-leaf `use_hook` version was the first sketch.

Models need only `derive(Facet)`. No `FromStr`, no `Display`, no `Default`, no
hand-written `From`/`Into` pair — which is exactly the boilerplate that made the
macro approach painful.

## What is NOT proven

**Enums are PROVEN (2026-09-02, through commit `2112440`, 37 tests)** — the first evidence
facet clears the case that broke the macro approach. Covered: enum fields, enums behind
`Option` (both `Some(v)` and `None`), and nesting. See [[facet_form_design_decisions]]
"Enum API surface" for the facet mechanics and gotchas, which still hold. **The API
described there is superseded** — `required_variants`, `VariantChoice::Absent` and the
disclosure loop were all deleted at `3063e28`; see "in-form answer" below.

**`Vec`/`Def::List` is PROVEN for EDIT MODE (2026-09-02, commit `cc9d650`)** — rows are
members named by index, `write_into` split into `write_value_into` + a default that does
`begin_field`/`end`, and `ListSet` alongside `FieldSet`/`VariantSet`. `Vec<Vec<T>>` and
`Vec<Enum>` fell out for free. See [[facet_form_design_decisions]] "`Vec`/`Def::List`" for
the mechanics and the two traps.

**Optional containers are PROVEN (2026-09-05, commit `786b899`, 68 tests)** — `Option`
composes with every member kind, because `member_for_shape` peels ONE layer and recurses
(the recursion IS the dispatch) and wraps the result in an `OptionMember` decorator.
`Option<Vec<Option<T>>>` works, which the old single up-front unwrap could never reach.
`FormField::required` and `VariantSet::optional` are both GONE — optionality is structural
now, not a flag — while `VariantChoice::Absent` survives. `seeding: bool` became
`FormMode { Blank, Populated }`. See [[facet_form_design_decisions]] "Optional containers",
especially "What the build taught us": the plan was wrong about deriving presence from
leaves, and it cost a silent data-loss bug.

The RED set (written 2026-09-03 at `358de03`, before the fix) is now the regression net,
in `src/tests/optional_containers.rs`. Two of its tests were deliberately green from the
start and still must be: an absent optional struct still offers its inner inputs (hide
them and there is nowhere to type, since presence is derived from what the user types),
and a partly filled one is still an error (absent means EVERY leaf empty, so a wrapper
that treats "some leaf empty" as absent would silently drop what was typed).

**The variant is now an IN-FORM ANSWER (2026-09-06, commit `3063e28`, 71 tests).**
`VariantChoice` is `Unchosen | Named`; optionality lives only in `OptionMember`. Net -103
lines: `choices.rs` is gone entirely (`MissingVariants`, `VariantOptions`,
`missing_variants`, `required_variants`), along with the iterative disclosure loop,
`empty_form_with_variants`, the `variants` map threaded through nine signatures, and the
"pre-flight empty ⟺ construction succeeds" invariant. `empty_form` is infallible. Added
back: `Form::choose_variant(path, variant)`, the schema rebuild a reactive `<select>`
triggers and the template for add/remove-row. See [[facet_form_design_decisions]]
"`Unchosen`" for the dispatch-by-containment reasoning and the wasm/ErrorBoundary finding.

**Split into modules** (`336ee68`, `ae8ab85`) mirroring formoxus's own seams, which is
exactly why the merge was cheap. Now at `crates/formoxus/src/reflect/`: `build.rs` (the
SHAPE walk — the one module with no formoxus counterpart), `fields.rs`, `form.rs`,
`members.rs` + `members/{field_set,variant_set,list_set,option_member}.rs`, with the tests
under `reflect/tests/` (in-crate, since several reach crate-private items). There is no
`reflect/error.rs` any more — see "Endgame" for why.

**ARCHITECTURE DECIDED 2026-09-05 (`6c92601`):** `Form<T>` stays exactly as it is — the
plain-data **schema**, with typed `Valid(T)` — and live editing state lives beside it in a
`Store<HashMap<path, String>>`, which is what `leaves()` already produces. Typing is a
per-path store write; adding a row or switching a variant is a rare schema rebuild that
the path-keyed value map survives. See [[facet_form_design_decisions]] "Form is the schema"
for what the prototype verified and for three rejected designs.
`crates/formoxus/REFLECT_PLAN.md` (was `VEC_PLAN.md`) has the build order.

STILL open, in the order they should probably be done:

1. **Create mode for lists** — the row count isn't in the shape, so `empty_form` silently
   yields zero rows; it should default to **1**. No pre-flight question is involved any
   more (that machinery is gone); this is a schema property, plus an add/remove-row
   operation modelled on `choose_variant`.
   `create_mode_yields_no_rows_yet` characterizes today's behavior and should start failing.
   `an_empty_optional_list_collapses_to_absent` may need flipping too — `Some(vec![])`
   currently reads as absent, which a deliberate zero-row length choice would overturn.
2. **Bare top-level enum model** (`form_for::<SomeEnum>`) — `fields_from_enum` returns the
   variant's fields flat with no `VariantSet`, so nothing calls `select_variant_named` on
   the top-level partial. Re-verified 2026-09-02: still fails, with
   `"must select variant before selecting enum fields"`. Not needed for `Question` (the
   enum is always a field there), so parked rather than fixed. A top-level bare `Vec`
   model would likely fail the same way, and can be parked for the same reason.
3. **The `Question` stress test itself** — the actual motivating case. `QuestionData` is an
   enum whose variants carry `Vec` fields; enum and Vec-edit are both proven now, so this
   is mostly waiting on #1.

`QuestionData`'s three struct-like variants are exactly the shape that made
`#[form(field_set)] data: T` and `#[field_set(model = QuestionData)]` mutually exclusive
under the macro, and reflection handles them with no per-variant hand-written type at all.
Edit-mode `Vec` landing means the `Question` comparison is now runnable in principle;
create mode (open item 1) is what an end-to-end "new question" flow still waits on.

**Work split (Todd's call):** Todd writes the core; Claude writes tests + reviews +
compiles/runs. Same loop as formoxus. (Twice now Todd has lost the thread mid-session and
asked Claude to write the construction directly — an exception, not the norm. Both times
the trigger was the same: too many similarly-named functions in one file to hold at once.)

## Endgame: MERGED 2026-09-06 (apcsp-dioxus branch `facet`, `6cbc131`)

Todd's 2026-09-02 call was "once `Vec` works, merge wholesale into `crates/formoxus`
and delete `facet-form-spike`." Done, except the delete. **The recorded reason for
waiting turned out to be stale**: this file said hold until create-mode lands because
it churns `MissingVariants` -> `MissingChoices`, but `3063e28` had already deleted that
whole family and made `empty_form` infallible. What's left of create-mode is internal
to `build.rs` plus an add/remove-row method — no construction-API churn, so there was
nothing to do twice. **Lesson: when a memory records "wait for X because of Y", re-check
that Y still exists before acting on it.**

The better argument for merging when we did: create-mode row count is a *design*
question (is a blank `Vec` one row? where do +/- buttons go?) that looking at a
rendered page answers better than reasoning from a `String` in a test assertion does.

**Shape of the merge** (two commits, deliberately separable):
1. `289f7b7` — subtree merge (`git merge -s ours --allow-unrelated-histories` +
   `git read-tree --prefix=`) so the spike's ~19 commits of design reasoning come
   along. Verified pure motion with `diff -r` before committing.
2. `6cbc131` — restructure + wire: `src/*` up to `reflect/*`, `lib.rs` -> `reflect.rs`,
   spike `Cargo.toml`/`Cargo.lock`/`.gitignore` dropped, `VEC_PLAN.md` ->
   `crates/formoxus/REFLECT_PLAN.md`.

**GOTCHA from the subtree merge:** `git blame -C` reaches the original spike commits
fine, but `git log <path>` stops at the wiring commit — the `-s ours` merge defeats
default history simplification. Use `git log --full-history -M -C -- <new> <old>` to
see across it.

**The two paths coexist and share exactly ONE thing: `crate::error`.** Both crates had
identical `FieldError`/`FormError` (String newtypes), and two same-named error types in
one crate is a trap, so the reflection path uses formoxus's; `FormAccessError` moved
there too, picking up the `Serialize`/`Deserialize` its neighbours carry. Everything
else stays separate on purpose — **`reflect::fields::FormField` is NOT
`formoxus::fields::FormField`** (the derive path's is generic over a `FromStr` value and
lives in a `Store`; the reflection one carries its own name/label and converts through
facet vtables). The module docs on `reflect.rs` say so. Whether they stay side by side
or one replaces the other is STILL undecided.

State after the merge: **83 tests pass (71 reflect + 12 formoxus), no new warnings.**
`facet` compiles for `wasm32-unknown-unknown` in this workspace, not just standalone.
The `question_proto` and `from_struct` compile errors are **pre-existing on `main`** —
`question_proto` IS the macro wall this path exists to get past, so don't read them as
merge damage.

## The render layer, built 2026-09-06/07

`FormMember::render(&self, ctx: &RenderCtx) -> Element` — real Dioxus, not
strings. One **component per leaf** (`reflect/widgets.rs::ScalarInput`), which is
load-bearing rather than decorative: a component is the unit of reactivity, so a
store read inside one subscribes *that* scope. Reading in `FormMember::render` — a
plain fn with no scope — would subscribe the whole form and re-render everything
per keystroke. The derive path's widgets spawn components for exactly this reason.

Landed since the merge, each with its own section in
[[facet_form_design_decisions]]: `RenderCtx` threading; `required` as a
presentation hint; `default_label` (`can_shuffle` -> "Can Shuffle", `None` for an
all-digit list-row index); fieldsets with legends, styled by DEFAULT with
`fieldset.bare` opting out; `$Variant` path segments; `Edit` as the single
structural-edit entry point; and the `Form`/`FormState` split with `use_form`.

**There is a page now: `/form-demo`** (`crates/web/src/views/form_demo.rs`), above
the auth guards, no login. Two forms from one `DemoQuestion` shape — create and
edit side by side — each with its value store as a table and a Check button
running apply -> validate. Deliberately NOT login: `LoginForm` is two `String`s so
it exercises none of this, and it would need widget selection (its password would
render `type="text"`), buttons, and form-level error rendering first. Login is the
right target *later*, as the smallest real page to force the button design against.

## The reactive `<select>` landed 2026-09-07 (108 tests)

`VariantSelect` in `reflect/widgets.rs`; `VariantSet::render` wraps the select and
its members in one `fieldset` with the label and required-star on the `legend`.
Five interaction tests in `reflect/tests/enums.rs` drive real DOM events through a
`Harness` in `reflect/tests.rs`. See [[facet-form-design-decisions]] for the four
non-obvious findings — one-template diffing, the VDOM-vs-mutations test gap,
listener order, and hot reload.

## Next

0. ~~AN UNTOUCHED CHECKBOX FAILS VALIDATION~~ — **DONE 2026-09-10, commit
   `8631631`, 130 tests.** The fix this file previously prescribed (map `""` ->
   `"false"` in `apply_leaves`) was TRIED AND REVERTED — see
   [[facet-form-design-decisions]] "An unticked checkbox". It closes the symptom
   and opens a worse hole: `Valid(false)` makes the field permanently present,
   and `FieldSet::is_present` is `any` over its members, so an `Option<Struct>`
   whose only discriminating field is a checkbox builds `Some(..)` for a section
   nobody opened. The note here half-saw it — it warned about exactly this for a
   BUILD-TIME seed and then failed to apply the same reasoning to the
   apply-time version, which has the identical flaw one level up.
   What shipped instead: `Empty` keeps one meaning everywhere and the two
   CONSUMERS know about checkboxes, via `FormField::is_unticked_checkbox`.

1. **Retire the derive path — IN PROGRESS.** Read
   [[facet-form-design-decisions]] "The `form2!` decision, and `FormSpec`" then
   "`apply_specs`, and `[]` as a row selector" FIRST.

   **DONE (`344347b`):** `FormSpec<T>` threaded through both constructors;
   `FormMember::apply_specs` on all five member kinds, wired into the constructor
   AND re-applied after every `FormState::edit`; `[]` row selectors end to end;
   Todd's `form2!` PARSER complete (title/validator/field entries with
   `{ control:, label: }` bodies, bracket paths, per-key errors). 153 lib tests,
   25 macro tests.

   **DONE 2026-09-13 (the chain to Login, all of it).** `expand()` emits a block
   expression holding a never-called witness fn plus the builder chain; `form2!` is
   re-exported behind `derive`; `HtmlInput` renders every `<input type=…>` and a
   password does not echo its value. `LoginForm` is on the reflection path in
   `crates/web/src/views/auth/login.rs` and the whole workspace builds under both
   feature sets. 162 lib tests, 22 in `tests/reflect.rs`, 56 macro tests, nothing
   ignored.

   Four things from that work worth not re-deriving:

   (a) **Tokens that can fail typechecking need a BORROWED span.** The witness's
   `.iter()` came from plain `quote!`, so it carried `Span::call_site()` and `[]`
   on a scalar underlined the entire `form2!` invocation while suggesting
   `empty_form::<Article>(chars)`. `quote_spanned!` with the segment's own span put
   the caret back on the author's `field[]`. Author tokens are fine by
   construction; anything the macro synthesises is not. Compile-fail goldens in
   `crates/formoxus/tests/ui/form2_*.rs` pin all of it, including one that pins the
   known limitation that a path through an `Option` cannot be expressed.

   (b) **The control vocabulary is flat and lowercase, and that is the stable
   surface.** `control: password`, not `control: Input(Password)` — the two-level
   shape is how formoxus groups `<input>` for dispatch, not a concept an author
   has, and HTML spells it `type="password"`. A `controls!` table generates both
   the lookup and the name list, so "did you mean" cannot drift from what
   resolves. `InputType::html_type()` is the pivot BACK to HTML (`tel`,
   `datetime-local`), so HTML's names sit on both sides of the enum. Rejected on
   implementing: gating names on whether `ScalarInput` can render them yet — the
   macro crate cannot see those match arms, so the list would be a hand-kept copy
   of a match in another crate, and its first false positive was `password`.

   (c) **`Integer`/`Float` collapsed to one `Number`.** Every `InputType` is now a
   literal HTML type attribute, which let the dispatch arm drop its guard
   entirely. A numeric field still DEFAULTS to `type="text"` (a half-typed value
   vanishes under `type="number"`); `number` is an override someone must ask for.
   `integer`/`float` are NOT aliases — two spellings for one attribute is the
   drift the table exists to prevent.

   (d) **`#[should_panic]` cannot see a panic raised inside a component render.**
   Dioxus catches it; the message prints and the test reports "did not panic as
   expected". The panic tests in `tests/specs.rs` work only because `apply_specs`
   runs OUTSIDE the render. Anything asserting a render-time panic has to test the
   guard directly.

   **`FormState::validate` NOW CALLS `spec.validator` — fixed 2026-09-13.** It had
   been stored by `with_validator` and read nowhere, so a cross-field check was
   accepted, type-checked and silently ignored. Three decisions in the fix:

   It runs **LAST, on the built model**, and cannot run sooner — a cross-field
   check is a statement about the whole value, which does not exist until every
   field has parsed. So a form whose fields are individually fine is built, then
   judged, and a rejected model is discarded; the wasted build happens only on the
   path that was going to fail. Corollary, pinned by a call-counting test: **a
   field error stops the validator running at all**, so a validator may assume
   well-formed input.

   The fn pointer is COPIED out (`Option<fn(..)>` is `Copy`), so no borrow of
   `self.spec` is held while `self.errors` is extended. And there is ONE exit for
   both failure kinds, so a form-wide error cannot be recorded and then returned as
   success.

   Alongside it, `Form::push_error(FormError)` — for a verdict only the server has
   ("invalid credentials"), which no field validator can produce. It writes through
   a COPY of the `Signal` (`self.state` is a plain `Signal`, NOT a `Store`, so
   there is no `.errors()` lens like the derive path's), which is what keeps `&self`
   and so keeps `Form` `Copy` inside an event handler. **Cleared by the next
   `validate`** — the intended lifetime, and why it never blocks validation.
   `FormState::render` now draws them as `ul.form-errors` / `li.form-error`, after
   the members, matching where the derive path puts them.

   Still the hardest piece after that: how server-side re-validation crosses the
   api/web boundary (`FormState<T>` holds `Vec<Box<dyn FormMember>>` and cannot
   serialize; send the model and return path-keyed issues). The cross-field
   validator stays formoxus's own either way — facet's `validator` attribute
   rejects container-level use.
2. **The widget registry — DEMOTED, probably not needed.** It existed to route
   around the orphan rule and to supply provider thunks. `form2!` solves both, by
   expanding in the CONSUMING crate where the type and its widget are both in
   scope and where a `Provider<C>` can be captured. So it is now an optional
   optimisation ("this type, everywhere") rather than a prerequisite for
   `Question`. Design in [[facet-form-design-decisions]] if it is ever wanted.
3. **`InputKind::Select` is dead** — nothing constructs it and the dispatch arm is
   still `todo!()`. `Boolean { optional: true }` covers the only shape-derived
   select there is, and data-driven pickers belong to the registry, not to
   `InputKind`. Probably delete it.
4. **`Question`** — needs `Facet` on `Markdown`, `Ref<Source>`, `RecordId`,
   `SelectType`, `AnswerChoice` first, plus 2.
5. **Mid-list insert / drag reorder** — rows are keyed, so both are pure additions
   rather than renames. Needs a control between rows (a UI question), and DnD must
   permute `rows` rather than reorder visually: tree order IS submission order, so
   a CSS-only reorder would submit stale.
6. **The discarded argument is GONE — `use_form` takes a thunk (2026-09-13).**
   This item used to say "measure it". It was cheaper to delete the work than to
   time it. `use_form` took a `FormState<T>` by value, so its whole argument — the
   `FormSpec` chain, a full facet shape walk, a fresh `Vec<Box<dyn FormMember>>` —
   was rebuilt on every render of the calling component and dropped by
   `use_signal` every time but the first. It now takes
   `impl FnOnce() -> FormState<T>` and passes it straight to `use_signal`, so the
   body got SHORTER and the construction happens exactly once. Cost: one `||` at
   42 call sites, none of which needed `move` — plain capture inference handled
   every case, including `form_demo`'s if/else where the spec is consumed in both
   arms.

   Two things on the same clock are still unmeasured, and still worth timing on
   BOTH targets (native is only a lower bound; the walk is dyn dispatch and
   allocation, where WASM's penalty is worst): `apply_specs` re-runs over the whole
   tree after every `FormState::edit`, and `ListSet::apply_specs` clones the spec
   map once per row. Both fire per structural edit (add/remove row, choose
   variant), which is user-initiated and rare.

   The frequency reasoning that made this look urgent, kept because it is what
   bounds the remaining two: leaf widgets subscribe PER PATH via
   `values.get_unchecked(path).try_read()`, so a keystroke re-renders one leaf and
   never the form component. The exception is a component that reads the value map
   itself — `crates/web/src/views/form_demo.rs` does, for its debug table, and says
   so in a comment. Do not generalise from that page; an earlier version of this
   note did, and called a `format!` per keystroke typical when it was not.
7. **Style the reflect path's class hooks — LOW PRIORITY, and not a bug.**
   `form-field`, `field-label`, `required`, `form-errors`, `form-error`,
   `form-title` are emitted by the reflection widgets and styled NOWHERE in
   `crates/web/scss/main.scss`. Todd's framing, 2026-09-13: the classes were added
   speculatively, "hoping they might be useful at some point" — they are hooks, not
   a styling commitment, so unstyled is the intended state until a page wants
   something. Do NOT treat their absence from the stylesheet as a defect.

   The one with a visible consequence today is `form-error`: a form-level message
   ("invalid credentials") renders as unstyled body text. Field errors do NOT have
   this problem — `FieldErrors` renders a bare `<small>` precisely so Pico's own
   `input[aria-invalid="true"] + small` rule colours it for free, which a `<ul>`
   gets nothing from.

8. **Blur validation** — deferred, but the design question is settled: the error
   should NOT live in widget-local state. `FormField::validate()` already
   populates `self.errors`, which already flows down as `props.errors`, so a
   "validate this path" callback (sibling to `on_edit`, not part of it — `Edit` is
   about shape) keeps one source of truth.

9. **Constraints as extra control arguments — DEFERRED ON PURPOSE 2026-09-13.**
   `control: integer { min: 1, max: 10 }`, `control: text { max_length: 40,
   pattern: "…" }`. Todd chose explicitly to stay on the basics instead. Two
   things to know when it comes back:

   (a) **`ValueKind::Text`'s three constraint fields can never be non-`None`
   today.** `FormField::value_kind` hardcodes `min_length: None, max_length: None,
   pattern: None` — a facet `ScalarType::String` carries no length bounds, so
   nothing populates them and nothing can. `ValueKind::Int`'s `min`/`max` ARE
   real (the type's own range, via the `int!` macro). So this syntax is the ONLY
   route by which text constraints ever become live, and the plumbing below it —
   `ValueKind` -> `ScalarInput` -> the rendered attributes -> validation — already
   exists and is already threaded.

   (b) **The syntax must not put the constraint on the CONTROL**, even though
   that is where the grammar makes it convenient. See
   [[facet-form-design-decisions]]: constraints nest in the VALUE kind precisely
   so a presentational override cannot discard validation. `control: text {
   max_length: 40 }` reads as a control argument but has to LAND in the field's
   `ValueKind`, or swapping `text` for `textarea` silently drops the limit. That
   is the design trap, and it is why this is more than parser work.

   The open question neither of those settles: when a spec's bound disagrees with
   the shape-derived one (a `u8` field given `max: 300`), does the spec win, does
   the shape win, or is it a compile error? A compile error is only possible for
   the integer case, where both numbers are known to the macro — text has no
   shape-derived bound to conflict with.

10. **BUTTONS — designed 2026-09-14, not built.** The reflection path renders
    fields and nothing else: the view hand-writes the `<form>`, the buttons and
    the `div.formoxus-buttons` wrapper (see `login.rs`), and `form2!` has no
    button syntax. Three decisions are settled — `render` OWNS the `<form>`
    element (two methods, so the 41 existing `render()` call sites re-point at a
    fields-only one); buttons are DECLARED in `form2!` and handlers SUPPLIED at
    `render`; and the derive path's `…Handlers` + `…Providers` collapse into ONE
    hidden slot struct built by ONE macro. Writeup checked in at
    `crates/formoxus/BUTTONS_PLAN.md` with a build order; the reasoning that is
    expensive to re-derive is in [[facet-form-design-decisions]] "Buttons on the
    reflection path". Two things still open: whether `IntoSlot` inference
    actually resolves (needs a compile test) and the macro's NAME (`wire!`
    rejected).

    Note the precedent this sits on: formoxus ALREADY owns buttons whose effect
    is internal — `.add-row`/`.remove-row` render with no handler from the view,
    because their effect goes through `on_edit` as an `Edit`. The open question
    was only ever about buttons with an EXTERNAL effect.


**Done 2026-09-09/10 — the `InputKind` branch is complete.** `ScalarInput` is now a
dispatcher over `InputKind { Text, Boolean { optional }, Select, Int { min, max },
Float }`, delegating to `TextInput`, `NumericInput`, `BooleanInput` and
`SelectInput`. A plain `bool` renders a checkbox, an `Option<bool>` the tri-state
select. 128 tests.

**Unrelated but adjacent:** this also settles, in the opposite direction from how it was
framed, the 2026-08-31 question of whether to *extract* formoxus into its own repo to host
the rewrite. The work came home instead. Extracting formoxus for its own sake (the
"generalize to other teachers/courses" goal) remains a separate, later question.
