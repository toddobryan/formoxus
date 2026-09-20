---
name: facet-form-design-decisions
description: "The design decisions behind the facet-based form spike and why each one was made — plus the runtime gotchas found the hard way (through 2026-09-16: push_field_error's edit()-style dispatch, form.rs split, the prelude::Form collision trap, and why Form/FormState can never cross a server fn boundary)"
metadata:
  node_type: memory
  type: project
---

Decisions made while building [[facet_form_spike]]. Each was reached by hitting an
actual problem, so the *reasoning* matters more than the conclusion — re-deriving these
from the code alone would be slow.

## Reflection replaces trait bounds in both directions

`FormField<T>` needs neither `FromStr` nor `Display`, because facet's vtables do both:

| direction | mechanism |
|---|---|
| `T` → input string | `Peek::new(t).to_string()` (display vtable) |
| input string → `T` | `Partial::alloc::<T>()` + `parse_from_str` (parse vtable) |

This supersedes the earlier plan to "put the `Display` bound on the widget" (mirroring
`widgets/input.rs:19` in the real formoxus, which bounds `T: FromStr + Display`). Not
needed — a model type only has to derive `Facet`.

## `""` IS absence, at BOTH boundaries (2026-09-02)

**The rule:** an empty display string means `FieldValue::Empty` means `None`. So
`Some("")` is not representable in a form, and neither is `""` in a required `String`.

**Not our compromise — HTML5's.** Constraint validation treats an empty input as
`valueMissing`: a `required` field rejects it, and an optional one submits nothing
distinguishable from untouched. (Same for `<textarea>`, and for `<select>` when the
chosen option has `value=""`.) No browser form could round-trip `Some("")` either, so
modeling the distinction would be modeling something the DOM cannot carry. This is also
why `raw_value` stays `String` and does **not** become `Option<String>` — every consumer
would immediately flatten `None` and `Some("")` back together.

**The bug this fixed (found 2026-09-02, before it bit):** the rule was enforced at only
*one* boundary. `apply_leaves` collapsed `""` → `Empty`, but `seed` produced `Valid("")`,
so the same model behaved differently depending on which path it came through:

| | seeded | via `leaves()` → `apply()` |
|---|---|---|
| `Option<String> = Some("")` | `Some("")` | `None` |
| `String = ""` (required) | validates fine | required-error |

**`leaves() -> apply()` was therefore not an identity** — the exact invariant the
uncontrolled design rests on. The existing identity test missed it only because it used
`Some("bringing dessert")`. Fix: `seed` collapses when `p.to_string().is_empty()`,
comparing the *display* string so both boundaries agree exactly (it's the same string
`raw_value` emits). Only `String` can reach it — `true`/`0`/`0.0` all render non-empty,
pinned by `zero_valued_scalars_are_not_empty`. A required `""` now fails on **both**
paths, which is the point: consistent-and-loud beats path-dependent-and-silent.

**Why this matters beyond the leaf:** it makes *deriving* `present` for optional
containers principled rather than a fresh compromise, which is what unblocked building
`OptionMember` without a construction question (Todd's observation). Note the limit,
though: the rule generalizes as "derive it", NOT as "all leaves empty ⟹ absent" — a
fieldless enum variant has no leaves to be empty. See "Optional containers".

**Gotcha that caused a real bug:** `raw_value` originally used `format!("{t:?}")`.
`Debug` quotes strings, and `parse_from_str` faithfully parses the quotes *into* the
value — `"Ada"` round-tripped to `"\"Ada\""`. Caught only by a genuine
`leaves() → apply()` identity round trip; the earlier tests hand-fed clean values, and
the SSR test used `contains(...)`, which matches happily inside the quoted version.
**Substring assertions on rendered HTML are weak — identity round trips are what hold
this honest.**

## `FieldSet` has no generic parameter

`FieldSet<T>`'s `T` was purely vestigial — no method body ever used it (`write_into`
just does `begin_field` → loop members → `end`; shape-checking happens only at the
leaves, in `FormField<T>::set`). Dropping it is what makes recursive construction from
a type-erased `Shape` *possible at all*: you can't turn a runtime `&'static Shape` into
a compile-time generic parameter, so a nested `FieldSet<Location>` was unconstructible.
With no generic, nesting is just recursion. Only `Form<T>`'s outermost `T` genuinely
needs to be concrete, because it alone calls `Partial::alloc::<T>()`/`materialize::<T>()`.

## `form_for(value: Option<T>)` unifies create and edit

`None` = create: walks the shape only, every field genuinely `FieldValue::Empty`.
`Some(t)` = edit: seeds from the value.

Critically, `None` **never consults `T::default()`**. Seeding from a default would make
a required `String` start as `Valid("")`, which is *not* `Empty`, so required-validation
would silently pass on an untouched field. This matches real formoxus, where
`FormFieldState::default()` gives every field `Empty` independent of the model's own
`Default`. Bonus: no `Default` bound is imposed on models.

## Required-ness comes from the model's shape

`shape.def.into_option()` answers both questions at once: `Ok(opt)` → optional, and
`opt.t` is the *real* shape (the `X` in `Option<X>`) that drives the widget and the
populated value. The value side unwraps the same level so peek and shape stay aligned —
`option_member` does both together, and getting them out of step is what produced
`expected Option<String>, but got String`.

**`required: bool` on `FormField` is GONE (2026-09-05).** It used to be stamped at
construction and did two jobs — "may this be `Empty`?" in `validate` and "wrap in
`Some`?" in `write_value_into`. Both moved to `OptionMember`, leaving a flag that was
always `true`, so it was removed. Optionality is now *structural*: an optional field is
a member wrapped in an `OptionMember`, not a member carrying a flag. `VariantSet::
optional` (literally `!required`) went the same way.

## Leaf paths are qualified by FIELD name, never struct name

`location.street`, not `Location.street`. Field names are unique within a struct by
construction, so composing them down the tree can't collide. Struct names carry no such
guarantee: `Trip { origin: Location, destination: Location }` would emit `Location.street`
twice and silently drop one in the `HashMap`. Pinned by
`repeated_struct_types_get_distinct_paths`. Vec convention (**implemented 2026-09-02**):
`answer_choices.0`, `answer_choices.0.text`. One rule throughout — "the path you'd walk
to reach this value in the model."

## Signals stay out of the data model entirely

`Form`/`FieldSet`/`FormField`/`FieldValue` are plain data: serializable, `Clone`, and
testable with no Dioxus runtime. Reactivity, if needed, lives only at the widget
boundary. This also killed a planned hand-written `Serialize`/`Deserialize` bridge —
with no `Signal` inside these types, nothing needs reading through.

**The rule survives; the "no signals at all" baseline did not — see "Form is the schema,
a Store holds the values" below (decided 2026-09-05).** What follows is the uncontrolled
design, which still works and is still tested, but is no longer the plan.

`FormData::values()` collects the whole form from the DOM on submit, keyed by each input's
`name` attribute — which is exactly the qualified path `leaves()` emits. So the DOM holds
editing state and `apply_form_values` shuffles it back once. **The `name` attribute is the
entire contract between DOM and model**; if those names drift from what `apply` expects,
everything silently stops matching (hence `uncontrolled_inputs_are_named_by_qualified_path`).

What uncontrolled gives up: live validation, conditional fields, reading state mid-edit —
which is exactly why it stopped being the baseline once the enum select had to be reactive
anyway.

## Enums: choose the variant BEFORE the form exists, then lock it

Todd's decision, and it's load-bearing: **it eliminates the reactivity requirement
entirely.** The only reason an enum needed a signal was that the set of inputs could
change mid-edit. If the variant is chosen first and can't change after, the form's shape
is fixed for its lifetime — back to fully uncontrolled.

1. **No blank form before a variant is picked.** Don't let users enter anything until
   they've decided the shape. Defaulting to the first variant would make "I forgot to
   choose" indistinguishable from "I meant MultipleChoice."
2. **No changing the variant once displayed.** Cancel and start over. Any retention
   policy annoys someone, and preserving typed data across a switch is ill-posed, not
   just hard: a half-filled `TrueFalse` has no coherent `MultipleChoice` representation
   (`answer_choices` invented from nothing, `answer` with nowhere to go).

So variant choice is a **construction parameter, not a form field**. Planned shape:

```rust
// Pure shape query, no value — drives the dropdown:
//   [("data", ["MultipleChoice", "TrueFalse", "FillBlanks"])]
pub fn required_variants<T>() -> Vec<(String, Vec<String>)>
// Build once choices are in hand: {"data": "TrueFalse"}
pub fn form_for_with<T>(value: Option<T>, variants: &HashMap<String, String>) -> Form<T>
```

`form_for(Some(t))` needs none of it — the value already pins every variant
(`variant_name_active()`). The member's `write_into` becomes
`begin_field(name)?.select_variant_named(chosen)?` → fields → `end()`. Variant names for
the dropdown come free from `UserType::Enum`.

## Server-assigned fields: the `ForCreate` pattern, not `deferred mode`

`Partial::build()` correctly refuses to materialize a struct with an uninitialized
field, so an `Event { id, .. }` whose `id` no form collects fails to build. facet's own
error suggests `begin_deferred`/`finish_deferred` — **don't reach for it.** SurrealDB's
`ForCreate` convention already dissolves this: `Form<T>` targets a type where every
field is genuinely collected (`EventForCreate`, no `id`), Surreal assigns the id on
create, and on edit the *caller* re-attaches the id it already had from the fetch.
`write_into`/`Partial` never need to know about it.

## Enum API surface (decided 2026-09-01)

Building the enum case (see [[facet_form_spike]] status). Decisions made this session:

**The construction trio makes the illegal state unrepresentable.** Instead of one
`form_for(Option<T>, &variants)` that could be called with a value AND variants (a
contradiction — the value already pins every variant), split by mode so the type system
forbids it:
- `form_for(value: &T) -> Form<T>` — edit; takes `&T` (not `T`: `Peek` only borrows, and
  the caller keeps the value to re-attach the `ForCreate` id). No variants param exists,
  so "Some + variants" *can't be written*.
- `empty_form::<T>() -> Form<T>` — create, no enum choices.
- `empty_form_with_variants::<T>(&HashMap<String,String>) -> Form<T>` — create with choices.
All three delegate to a private helper taking `Option<&T>` + `&HashMap`. This recovers the
one compile-time guarantee that *is* structural; the rest is irreducibly runtime.

**~~Runtime checks are `panic!`/`debug_assert!`, NOT `Result`.~~ REVERSED 2026-09-02 —
see "The panic/Result reversal" below. The original reasoning, kept because the general
principle still holds and only its application here was wrong:** Todd's worry: returning a
`Result` for something decidable-in-principle-at-compile-time "feels like writing Python."
Resolution: Rust's `Result`-vs-`panic!` split is the *opposite* of Python's one exception
channel — `Result` = expected/recoverable, `panic!`/`assert!` = contract violation / bug.
A caller passing bad variants is a bug, so asserting the invariant is the idiomatic
stand-in for a compile error the type system can't see (like `Vec[i]` panicking, not
returning `Result`). `debug_assert!` even compiles out in release. Two checks remain, both
genuinely runtime (reflection defers them, nothing could catch them at compile time):
(1) supplied variant keys/values must match `required_variants::<T>()`; (2) `empty_form`
on a `T` that has a required enum field must fail ("choose a variant, use `..._with_variants`").

**`required_variants::<T>() -> HashMap<String, VariantOptions>`** (was
`HashMap<String, Vec<String>>`; the value type gained an `optional` flag on 2026-09-02). Public form takes no arg, walks `T::SHAPE`; keyed by each
enum field's **qualified path** → variant names in **declaration order**. Empty map when a
`T` has no enum fields anywhere (this is what makes `empty_form::<T>()` safe for such a T).
Implementation: recursive walk with a `&mut HashMap` **out-param** (not a threaded return —
that's what tangled the borrow checker on the first attempt), `qualify(prefix, name)` for
paths (no leading dot), and `_ => {}` to **skip** non-enum shapes (the struct arm recurses
into every field, so scalars/Option/List must be skipped, not panicked on). Same shape as
`collect_leaves`. Also drives the variant dropdowns and the validation check above.

**Confirmed facet 0.46.5 enum API** (verified against the installed crate + facet's own
`tests/partial/deserialize.rs`, since it's pre-1.0 and moving):
- Shape: `Type::User(UserType::Enum(EnumType))`; `EnumType.variants: &'static [Variant]`;
  `Variant { name: &'static str, data: StructType }` → `variant.data.fields` are the
  variant's `&[Field]` (same `Field` type `members_for` walks).
- Read (edit mode): `Peek::into_enum()? -> PeekEnum`; `PeekEnum::variant_name_active()?`,
  `::active_variant()?`, `::field_by_name(name)?` (parallels `PeekStruct`), and
  `PeekEnum: HasFields` (`.fields()` → `(Field, Peek)`).
- Build (`VariantSet::write_into`): `partial.begin_field(name)?.select_variant_named(v)?`
  → per-field `begin_field/set/end` → `.end()`. (`select_nth_variant`/`select_variant`
  by index/discriminant also exist.)
- Deriving `Facet` on an enum **requires `#[repr(u8)]`** (every enum in facet's tests has it).
- `UserType::Union` = a Rust `union` (untagged, no discriminant), NOT an enum — ignore it,
  same as `Opaque`; the enum arm targets `UserType::Enum` only.

**`VariantSet` (BUILT 2026-09-02, commit `075a556`; reshaped at `cec4669` to hold
`choice: VariantChoice` + `optional: bool` instead of `variant: String`)** = `FieldSet` +
a locked choice. `collect_leaves`/`apply_leaves`/`validate`/`has_errors` identical to
`FieldSet` (the variant name is a construction param, NOT a leaf — not emitted). Only
`write_into` differs: `begin_field(name)?.select_variant_named(&variant)?` → members → `end()`.
`member_for`'s `UserType::Enum` arm builds it, mirroring how the struct arm builds a `FieldSet`.
Two helpers are shared between that arm and the top-level `fields_from_enum`:
`chosen_variant(enum_type, peek_enum, variants, prefix) -> &'static Variant` (edit: the value's
`variant_name_active()`; create: `variants[prefix]`) and `variant_members(variant, peek_enum,
variants, prefix)` (one `member_for` per variant field, seeded from `peek_enum.field_by_name`).

**Gotchas found building it:**
- **Construction `prefix` must accumulate.** It was passed *unchanged* through
  `member_for`/`fields_from_struct`, so it was always `""` — fine for leaf paths (those are
  computed later in `collect_leaves` by threading the member tree), but the create-mode
  `variants[prefix]` lookup for a nested enum field needs the *real* path. Fix:
  `fields_from_struct`/`variant_members` call `member_for` with `&qualify(prefix, field.name)`.
  So there are TWO independent path mechanisms that must agree: construction-time prefix (for
  the variants lookup only) and collect-leaves-time prefix (for leaf `name` attrs).
- **`PeekEnum<'_, 'static>`** — `PeekEnum` has two lifetimes; the `'facet` one must be pinned
  to `'static` (like `Peek<'_, 'static>` everywhere else), or you get "lifetime may not live
  long enough" when a seeded field-peek flows into `member_for`. `PeekEnum` is `#[derive(Copy)]`,
  so `chosen_variant` reading the variant name doesn't consume it — `variant_members` reuses it.
- **Bare top-level enum model is a known gap** (`form_for::<SomeEnum>`): `members_for` dispatches
  it to `fields_from_enum`, which returns the variant's fields *flat* with no `VariantSet`, so
  nothing calls `select_variant_named` on the top-level partial → won't materialize. Enum *fields*
  are fine (member_for wraps them). Not needed for `Question` (enum is always a field there).

## Optional enums and the panic/Result reversal (2026-09-02)

**`Option<Enum>` was broken in both directions**, and neither was covered by a test.
- `Some(v)` died on an internal `unwrap` *inside facet* (`eenum.rs:87`): `begin_field`
  lands on the `Option` slot, not the enum inside it, so `select_variant_named` looked for
  the variant among `None`/`Some`. Fix: `begin_some()` first — and it **pushes a frame**,
  so it needs its own `end()`. (`select_variant_named` does *not* push, which is why the
  plain enum path gets away with a single `end()`.)
- `None` died in our own `chosen_variant`, which exposed that **`form_for` was not actually
  infallible**: an absent optional enum pins no variant, so edit mode fell through to the
  empty choice map. Fixed by giving `chosen_variant` a `seeding` flag — while seeding, a
  `peek_enum` of `None` means the value's `Option` really *was* `None` (→ absent, no choice
  needed); otherwise it means "no value, consult the map." Signature is now
  `-> Option<&'static Variant>`, where `None` is the absent answer.

**Choices are typed, not strings.** `VariantChoice { Absent, Named(String) }`. A `"None"`
sentinel string would be ambiguous for a perfectly ordinary model like
`enum Filter { None, ByDate { .. } }` — "leave this optional field empty" has to be its own
thing, not a reserved name. Options side is `VariantOptions { optional, variants }`.

**Disclosure is iterative, which is why the original walk was wrong.** An enum's variant
must be answered before its fields are visible at all, so answering one can reveal enums
that were unreachable a moment earlier (`Doc` → `Outer::First` → `Inner`).
`missing_variants::<T>(chosen)` therefore descends *into the chosen variant's* fields; the
caller loops until it comes back empty. The earlier non-recursive version could only ever
see enums reachable without making a choice.

**The panic/Result reversal.** The 2026-09-01 decision above was to `panic!` on a missing
variant, on the reasoning that a bad choice is a caller bug, and `Result` for something
decidable-in-principle "feels like writing Python." That general principle is right and
still stands — but it was misapplied here, for two reasons:
1. **The error payload is the UI.** `MissingVariants(HashMap<path, VariantOptions>)` is
   exactly what a variant-picker needs to render. Handling it isn't defensive boilerplate
   bolted on; it *is* the create-a-record flow. A panic throws that away.
2. **Iterative disclosure makes "missing" an expected state, not a bug.** You cannot know
   the full set of questions up front, so the first call *legitimately* comes back
   incomplete. That's the normal path, and `Result` is the right channel for it.

So `empty_form`/`empty_form_with_variants` now return `Result<Form<T>, MissingVariants>`,
pre-checking with `missing_variants` before constructing. The residual panics inside
`chosen_variant` stay, but are now genuinely unreachable from the public API — and the
invariant that keeps them so is: **`missing_variants` empty ⟺ construction succeeds.** If
the pre-flight walk and the construction walk ever disagree about what to descend into,
the `Result` starts lying (reports nothing missing, then panics). That coupling is the
thing to protect; it's why a non-recursive pre-flight was not an option once nested enums
existed.

Two smaller calls, both so failures hand back something useful: an unrecognized variant
name is reported as *still-missing* (giving the caller the real options) rather than
falling through to a panic deep in construction, and `Absent` on an enum that isn't behind
an `Option` is likewise reported rather than silently accepted.

**Frame arithmetic for the three write paths**, kept in one `match` in `VariantSet::write_into`
so it stays auditable side by side:

| case | calls | `end()`s |
|---|---|---|
| `Absent` | `begin_field` → `set_default` | 1 |
| `Named`, plain enum | `begin_field` → `select_variant_named` → members | 1 |
| `Named`, behind `Option` | `begin_field` → `begin_some` → `select_variant_named` → members | 2 |

`set_default()` is what writes the absent value: `Option`'s `Default` is `None` whatever
the inner type is, so it never needs the concrete type reflection can't give us.

**`Absent` renders a disabled `--none--` input** (`ABSENT_DISPLAY`) so leaving a value out
stays visible rather than the field silently vanishing — and it's where a `<select>` would
go if variant choice ever becomes live. Safe because **disabled widgets aren't submitted**,
so that string never returns through `FormData::values()` and can't be mistaken for a value.

That makes `VariantSet::Absent` the first member that **renders without being a leaf**,
splitting the two renderers for real: `Form::render()` walks members and shows it, while
the `leaves()`-driven Dioxus component can't. Resist pushing render metadata into
`leaves()` — it's a *value* view feeding `apply` and signal-minting. The right fix, when
the Dioxus path becomes real, is to walk members there too.

One coupling to watch: `optional: !required` is the only place those two concepts meet.
`required` comes from `Def::Option` and now does double duty — "may be empty" for scalars,
"descend a frame before selecting" for enums. They coincide because both mean "behind an
`Option`," but if a case ever separates them, that line is where it bites.

## `Vec`/`Def::List`: edit mode (2026-09-02, commit `cc9d650`)

**Rows are members named by their index** (`"0"`, `"1"`), held in a `ListSet` that is
structurally identical to `FieldSet`. That single choice is why `collect_leaves`/
`apply_leaves`/`validate`/`has_errors` needed **no list-specific code at all**: the
container qualifies `prefix` with its own name, each row appends its index-name, and
`answer_choices.0.text` falls out of the machinery structs already used. A flattened
alternative (rows as siblings in the parent's member list, named `"answer_choices.0"`)
was rejected: nobody would own the `init_list` grouping, and the dot would have to be
baked into a name by hand.

**The one real refactor — `write_into` splits in two.** A list item has no named field;
the item *is* the position, so a row can't do `begin_field("0")`.

```rust
fn write_value_into(&self, p) -> ...;              // write at the CURRENT position
fn write_into(&self, p) -> ... {                   // default: descend, write, come back
    self.write_value_into(p.begin_field(&self.name())?)?.end()
}
```

All four member kinds opened with `begin_field(&self.name)?` and closed with
`partial.end()`, so each just lost those two lines; no impl overrides `write_into`.
`ListSet::write_value_into` = `init_list()` → per row `begin_list_item()?` →
`row.write_value_into(..)` → `end()?`. **`begin_list_item` pushes a frame** (like
`begin_some`, unlike `select_variant_named`) — same frame arithmetic trap as `Option<Enum>`.
Watch the one asymmetry: a `FieldSet`'s members still call `write_into` (struct fields
*are* named); only list rows call `write_value_into`. Reversing that compiles fine and
fails at runtime.

**Construction split alongside it:** `member_for(field, ..)` is now a wrapper over
`member_for_shape(shape, name, ..)`, because a row has no `Field` and so no `field.name`.
Plus `struct_member`/`enum_member`/`list_member` extracted to parallel `scalar_member`.
`enum_member`'s `seeding` param is the **outer** `peek.is_some()`, not the unwrapped one —
two peek-derived args side by side that look like they should agree and mustn't.

**`prefix` convention, easy to get wrong:** the prefix handed *into* `member_for_shape`
already includes that member's own name (`fields_from_struct` qualifies before calling),
which is why `struct_member` passes it through unchanged. `list_member` must qualify the
index on itself. Get it wrong and leaf paths drift from `variants`-map keys — invisible
until `Vec<Enum>` in create mode.

**Fell out for free, now tested rather than accidental:** `Vec<Vec<T>>` (→ `rows.0.1`) and
`Vec<Enum>` in edit mode (each row's variant pinned independently by the value, no caller
choice needed).

**Two known gaps** (both marked in `vec_tests`):
1. **`Option<Vec<T>>` fails** — `init_list can only be called on List or DynamicValue
   types`, because `begin_field` lands on the `Option` slot. Literally the `Option<Enum>`
   bug again, and it drags in the same seeding ambiguity (an `inner_peek` of `None` means
   both "create mode" and "the `Option` was `None`"). `#[ignore]d` with the diagnosis.
   **See "Optional containers" below — this is one instance of a general gap, not a
   list-specific bug, so don't fix it by bolting an `optional` flag onto `ListSet`.**
2. **Create mode yields zero rows, silently** — the row count isn't in the shape, so it's
   a construction parameter like a variant choice, and it isn't plumbed yet. That's step 4:
   `MissingVariants` → `MissingChoices` carrying lengths too, with the two walks
   (`missing_variants` pre-flight and construction) kept in step per the invariant above.
   `create_mode_yields_no_rows_yet` characterizes today's behavior and **should start
   failing** when lengths land; the list identity round-trip likewise reloads into
   `form_for(&same_value)` instead of `empty_form`, and swapping it back is a good
   acceptance check.

**Add/remove a row needs no reactivity**, which is why locking the count up front is far
less restrictive than the variant lock: it's a discrete action, so collect the DOM values,
rebuild at `length ± 1`, `apply()` them back. Row *n*'s paths don't move when row *n+1*
appears, so nothing is lost.

## Optional containers are broken, and `Option` is a decorator not a member kind (2026-09-02)

Probed after the `Vec` work; the tally across the four member kinds:

| behind `Option` | status |
|---|---|
| scalar | **works** — `FormField` has the concrete `T`, writes `set(Some(t))`/`set(None::<T>)` |
| enum | **works** — `VariantSet` grew `optional` + `begin_some` (see the reversal section) |
| struct | **broken both ways** |
| list | **broken both ways** |

`Option<Struct>` fails *differently* in each direction, and the `None` case is the
dangerous one:
- `Some(v)` → panics, `must select variant before selecting enum fields` (facet models
  `Option` as an enum internally, so `begin_field` on the inner field hits the `Option`).
- `None` → **no panic**: `validate()` just returns `None`, because the inner fields are
  required and `Empty`. So an absent optional struct is currently *unrepresentable* — the
  form demands the inner fields. Silent-and-wrong, which is worse than the panic.

**DONE 2026-09-05, commit `786b899` — all ten RED tests pass.** What follows is what
was built and why; see "What the build taught us" at the end for the two things the
plan got wrong.

**Do NOT fix this by giving `ListSet` (and `FieldSet`) their own `optional` flag** the way
`VariantSet` has one. `Option` is not a fifth member kind alongside scalar/struct/enum/
list — those are mutually exclusive, while `Option` is a **modifier that composes with
each of them**.

**Peel ONE layer and recurse — Todd's design, and it's right.** (Claude first argued an
`option_member` would have to re-dispatch on the inner kind, duplicating the match beside
it. Wrong: recursing back into `member_for_shape` *is* the dispatch, so nothing is
duplicated.)

```rust
fn member_for_shape(shape, name, peek, variants, prefix) -> Box<dyn FormMember> {
    if let Ok(opt) = shape.def.into_option() {
        return option_member(opt.t, name, peek, variants, prefix);  // peel one, recurse
    }
    ... scalar / list / struct / enum ...
}
```

Strictly more general than the old code, where the `Option` unwrap happened **once**, up
front, flattened into a `required: bool` threaded downward — exactly why one layer was the
ceiling. Peeling gives `Option<Vec<Option<T>>>` →
`OptionMember(ListSet(rows of OptionMember(FormField)))` for free, and the write path
composes too, each layer contributing exactly one `begin_some`/`end` frame.

The member is a **decorator**, and it holds exactly ONE field:

```rust
pub struct OptionMember { pub inner: Box<dyn FormMember> }
```

No `present: bool` — storing it would freeze at construction, and `apply_leaves` could
never change it, which breaks "filling in an absent optional struct makes it present."
No `name`/`label`/`errors` either: the inner already owns the name (so the default
`write_into` works unchanged), and the wrapper adds no validation of its own — it
*suppresses* the inner's. It forwards everything and overrides three things:
`write_value_into` (`begin_some()` → inner → `end()`, or `set_default()`), `validate`
(inner only when present, else `clear_errors`), and `is_present` (delegate to inner).
`VariantSet` drops its own `optional` field, retiring the `optional: !required` wart.

**`FormField::required` did double duty** — "may this be `Empty`?" in `validate`, and
"wrap in `Some`?" in `write_value_into`. Once `OptionMember` owns the `Some`, a wrapped
`FormField` is built from the bare `T` and so is `required: true`: it writes correctly but
then errors on empty, for a field that's legitimately optional. So the wrapper intercepts
`validate` as well (absent ⇒ `inner.clear_errors()` rather than `inner.validate()`) — it
is NOT a pure pass-through. **`clear_errors` had to be added to `FormMember`** for this:
merely *skipping* validation leaves stale errors from a previous pass when the field was
present, and `has_errors()` then vetoes a legitimately absent field. With both jobs moved,
`required` was always `true` and was deleted.

**`present` is DERIVED, not asked** — but derived *per member kind*, NOT by scanning
leaves. This was briefly planned as a third construction question (alongside variant and
length); `""` IS absence (see that section) settles that it should be derived, since
deriving at the container is the same rule already enforced at the leaf, one level up.
Asking would also be asking about a distinction the widgets can't convey — without a
presence affordance the user has no way to express `Some(Location{"","",""})` vs `None`.

**But "every leaf underneath is empty" is the WRONG derivation, and it cost a silent
data-loss bug.** A chosen *fieldless* variant contributes no leaves at all, so
`Some(Mode::Fast)` and `None` produce byte-identical leaves. A wrapper that scans leaves
concludes "absent", takes the `set_default()` branch, and the user's choice evaporates —
no panic, no error. Verified: `Some(Mode::Fast)` round-tripped to `None`.

So `is_present` is a **trait method on `FormMember`**, and each kind answers from its own
state: `FormField` → `value != Empty`; `FieldSet` → any member present; `ListSet` →
non-empty and any row present; `VariantSet` → `choice != Absent`; `OptionMember` →
**delegates to its inner**, which is the whole point. Only `VariantSet` can answer for an
enum, and nothing else can see the answer.

Consequence: `Some(Location{"","",""})` and `Some(vec![])` are unrepresentable until some
affordance (a checkbox, or the disabled `--none--` input `VariantChoice::Absent` renders)
carries the distinction. Reversible; that affordance is the fix if it's ever needed.

So **`MissingChoices` stays two questions, not three** — which variant, and how many rows.
Optional containers need no construction question, so `option_member`/`OptionMember` can
land independently, BEFORE step 4.

### What the build taught us (2026-09-05, `786b899`)

Two things the plan above got wrong, both found by running it.

**1. `seeding: bool` became tautological, and it's a naming lesson.** `chosen_variant`
needed to know edit-vs-create to read a `peek_enum` of `None`. That was computed as the
*outer* `peek.is_some()` — which worked only while exactly one `Option` layer was unwrapped
up front. Peeling per-recursion made the call site pass `peek.is_some()` on the very peek it
was meant to disambiguate, so an absent optional enum reported "create mode", consulted an
empty map, and panicked.

Replaced by **`FormMode { Blank, Populated }`**, fixed once in `form_for_impl` and threaded
down through nine signatures, never recomputed. Todd's reason for wanting an enum is the
real lesson: *"seeding" reads as an action in progress, so computing it locally looks
reasonable*. A mode is carried, and a name that says "mode" makes recomputing it look as
wrong as it is. Also: `Populated` with no peek at some node is not a contradiction — it
means "there is a value and here it is absent", which is why `Populated(Peek)` would be the
wrong tightening (it outlaws a legal state).

**2. "All leaves empty ⟹ absent" is WRONG, and it cost a silent data-loss bug.** A chosen
*fieldless* variant contributes no leaves, so `Some(Mode::Fast)` and `None` produce
byte-identical leaves. `OptionMember::is_present` scanned leaves, concluded "absent", took
`set_default()`, and the user's choice evaporated — no panic, no error. Nothing caught it
because every optional-enum test used an enum whose variants all carry fields.

Fix: **`is_present` is a trait method**, each kind answering from its own state —
`FormField` → `value != Empty`; `FieldSet` → any member present; `ListSet` → non-empty and
any row present; `VariantSet` → `choice != Absent`; `OptionMember` → **delegates to inner**.
That is why `VariantChoice::Absent` had to survive retiring `VariantSet::optional`: the two
look like halves of one idea, but `optional` is a mechanical frame flag (now the wrapper's
job) while `Absent` is an *answer* nothing else records.

**Test-design note.** The regression needs a PAIR: `a_chosen_unit_variant_survives_behind_
an_option` fails if `is_present` scans leaves, and `an_absent_optional_unit_variant_stays_
absent` fails if it just returns `true`. Either alone admits a wrong fix. Verified by
reverting the fix and watching exactly two of three fail.

## Form is the schema, a Store holds the values (2026-09-05)

> **Superseded in its naming, not its substance (2026-09-07).** The plain-data
> type is now `FormState<T>` and `Form<T>` is the reactive handle — and it is
> *state*, not schema. See "Form vs FormState" below.


**Todd's call, after a prototype.** Two things with separated jobs:

| | holds | changes | reactivity |
|---|---|---|---|
| `Form<T>` | members, structure, typed values, errors | only on a **structural** edit | `Signal<Form<T>>`, coarse |
| `Store<HashMap<String, String>>` | live edited raw strings, keyed by path | every keystroke | per-path, fine-grained |

**`Form<T>` is UNCHANGED.** Plain data, `FieldValue::Valid(T)` still typed, serializable,
testable with no runtime. Signals still stay at the widget boundary — a `Store` *wraps*
plain data rather than putting signals inside it, so the standing decision holds.

**Why it works:** typing and structural change are different operations on different
clocks. Typing is a store write that dirties one path. Adding a row or switching a variant
is a schema rebuild — rare, discrete, already the collect → rebuild → re-apply move.
Trying to serve both with one mechanism is what made every earlier design awkward.

**What the prototype actually verified** (`scratchpad/storeproto`, 3 tests):
1. `Store<HashMap<String,String>>::get(path)` gives a child store addressed by a **runtime
   string**. The original objection — `#[derive(Store)]` generates *named* accessors,
   impossible for `Vec<Box<dyn FormMember>>` — was about the **derive**, not the store.
   `SelectorScope::hash_child` takes any `impl Hash`.
2. Inputs bind to per-path child stores (`value` + `oninput`).
3. **Writing one path re-renders only that leaf** — the objection that sent us
   uncontrolled ("we'd have to recreate the entire struct every time any input changes").
   Not luck: child props are identical across the write, so prop memoization would
   suppress *every* re-render; only the store subscription can dirty that one leaf.

Also verified: the crate, facet included, compiles for `wasm32-unknown-unknown`.

**The value map survives a schema rebuild** — keyed by path, and paths are stable under
growth (row *n* never moves when *n+1* appears). Add a row: new keys, existing values
untouched. Switch a variant: drop the keys under that prefix.

**Destructive-switch warning becomes trivial:** "does any key under this prefix hold a
non-empty value?" No DOM scan, no staleness. This supersedes an earlier same-day decision
to scan the DOM, which only existed because the in-memory `Form` was stale.

### Roads not taken (keep the reasoning; it was expensive)

- **Structure recovered from submitted names** (`answers.0`/`answers.2` → count indices;
  `data.$variant` → the variant). Genuinely workable — the naming convention *is* lossless
  — but unnecessary once the store owns values and the Form owns structure. It also made
  reading fallible in a new way (failures with no field to attach to) and turned submitted
  names into a trust boundary. Worth remembering if a plain-HTML/no-JS target ever matters.
- **Raw-canonical `FieldValue::Valid(String)`.** Considered as the prerequisite for lensing
  *into* `Form`. Unnecessary — values live *beside* the Form — and **actively worse**:
  `Valid(T)` makes `form_for(&m).validate() == Some(m)` true **by construction**, never
  passing through a string. Raw-canonical would convert that structural guarantee into a
  dependency on `T -> String -> T` fidelity, which `tests/roundtrip.rs` now pins for
  built-in scalars but can never enforce for a user's custom type.

## `Unchosen`: the variant becomes an in-form answer (2026-09-05)

**Todd's call.** With a reactive `<select>`, choosing a variant is something the user does
*in* the form, not a construction parameter supplied beforehand. So `blank_form::<T>()`
starts with nothing chosen and is **infallible**, and the choice is validated like any
other field:

- enum **not** behind an `Option` → the select is required; unchosen is a validation error.
- enum **behind** an `Option` → the select offers a `--none--` entry, which is `Absent`.

**If this lands, a lot retires:** `MissingVariants`, `VariantOptions`, `missing_variants`,
`required_variants`, the iterative disclosure loop, and the **"pre-flight empty ⟺
construction succeeds" invariant that has bitten us twice**. That invariant existed only
because construction had to be told the answers up front.

This reverses the 2026-09-02 "choose the variant BEFORE the form exists, then lock it"
decision, deliberately. Its first half was justified by "it eliminates the reactivity
requirement" — spent, now that the select is reactive. Its second half (don't let users
*change* a displayed variant) survives as a **UX rule**: switching wipes that subtree, with
a warning when there is something to lose.

**BUILT 2026-09-05, commit `3063e28` (71 tests, net -103 lines).**

**RESOLVED: two states. `Unchosen` REPLACES `Absent`.** `VariantChoice` is
`Unchosen | Named(String)`, and optionality lives only in `OptionMember` — exactly parallel
to `FieldValue`'s `Empty | Valid`. The reasoning, by analogy with what already happened to
`FormField`:

- `FormField` lost `required`. `validate()` now errors on `Empty` unconditionally, and
  `OptionMember` *suppresses* that when the subtree is absent. Optionality is structural.
- By the same move: `VariantSet::validate()` errors on `Unchosen` unconditionally,
  `OptionMember` suppresses it, and `is_present()` becomes `choice != Unchosen`. For an
  optional enum, "never touched" and "chose `--none--`" both mean `None` anyway, so nothing
  needs to tell them apart — the same collapse as `""` IS absence at a leaf.

That would make `VariantChoice` exactly parallel to `FieldValue`: one "no answer" state,
with optionality living in the wrapper rather than in the value.

The deciding observation: for an optional enum, "never touched it" and "explicitly chose
`--none--`" produce the *same* model value (`None`), so nothing downstream needs to tell
them apart — the same collapse as `""` IS absence at a leaf.

**The counter-argument was rendering**, and it is real: a required enum wants a disabled
`Choose…` placeholder, while an optional one wants `--none--` shown as a legitimate
selected option. Those differ visually even if they validate identically. Resolution:
`OptionMember` wraps the `VariantSet`, so **it** injects the `--none--` entry, keeping the
distinction where optionality already lives instead of duplicating it in the value.
`ABSENT_DISPLAY` and the disabled-input rendering move there too.

**The rename was NOT independently landable, and wasn't landed that way.** `missing_variants` currently treats `Absent`
as a satisfying answer when optional; `Unchosen` is by definition *never* a satisfying
pre-flight answer, because demanding a choice is what the pre-flight is for. So the two
changes are one commit: `Unchosen` arrives as part of "the variant becomes an in-form
answer", which deletes the pre-flight. The predicted bonus landed: the `variants: &HashMap<..>`
parameter threaded through nine signatures disappeared entirely, `choices.rs` with it, and
`empty_form` became infallible (its `Result` existed only for the unchosen case).

### `choose_variant`, and why containers dispatch by containment

`Form::choose_variant(path, variant)` is the one operation added back — the schema rebuild
a reactive `<select>` triggers, and the template for add/remove-row. Six impls plus a
shared helper:

```rust
pub(crate) fn owns(nested: &str, path: &str) -> bool {
    path == nested || path.strip_prefix(nested).is_some_and(|rest| rest.starts_with('.'))
}
```

**Containers dispatch by path CONTAINMENT, not by trying each child in turn.** With a
`String` error a parent cannot tell "not this child" from "this child, and the variant name
was bad" — so try-each would swallow the real message and report "no such path". Testing
containment first means at most one child is ever asked, and its error propagates verbatim.
(An error *enum* would allow try-each; the containment guard is what buys the good message
without one.)

Per kind: `FormField` always errs, but distinguishes "that's a field, not an enum" from "no
such path" — the first means the caller's path was right and its expectation wasn't.
`OptionMember` passes the prefix through **unchanged**, since `name()` delegates to the
inner, which is what keeps `Option` from contributing a path segment. `VariantSet` handles
three cases: not-mine / mine-but-deeper (how `outer.inner` becomes reachable once `outer`
is answered — the disclosure loop's successor, minus the loop) / mine-exactly, where it
looks the variant up in its stored `enum_type`, rebuilds via `variant_members(chosen, None,
FormMode::Blank, &nested)`, and **replaces rather than merges** (a half-filled `Circle` has
no coherent `Rectangle` reading).

**Storing `enum_type: &'static EnumType` on `VariantSet` is what makes the rebuild
possible** — the enum type used to be consulted at construction and discarded. It will also
hand the `<select>` its option list.

**An unknown variant is an `Err`, never a panic** — and the reason is mechanical, not
philosophical. Both failure modes are caller bugs (the path comes from our own member tree,
the name from an `<option>` we generated; neither crosses the wire, since that was the
*rejected* design). Todd's instinct was to let it reach an `ErrorBoundary`. But
dioxus-core's own source says panics are not caught on wasm — `CapturedPanic` "will not be
created in WASM" — so a panic aborts the client instead of reaching a boundary. And event
handlers return `()`, so `?` doesn't propagate from them either. The `Result` is
**transport**: the widget layer is expected to push it into the boundary via
`ErrorContext::insert_error`, not to recover from it. Note this is a *different* reason
from the 2026-09-02 panic→Result reversal, where the error payload was itself the UI.

Whichever wins, the substance of the earlier "`Absent` must survive" note holds: a chosen
*fieldless* variant contributes no leaves, so presence must come from the choice, never
from scanning leaves.

## `T -> String -> T` must be the identity (2026-09-05)

Pinned by `tests/roundtrip.rs` for every built-in scalar: `i*::MIN`, unsigned `MAX`,
`1.0/3.0`, `0.1 + 0.2`, `MIN_POSITIVE`, `EPSILON`, both infinities, and strings with
quotes/newlines/trailing spaces/unicode. All pass. For floats that is a language guarantee,
not luck — Rust's `Display` emits the shortest decimal that parses back identically. NaN
round-trips but is never `==` itself, so the property to assert is that it comes back NaN.

**Where it bites is narrower than it looks.** Only the DOM path depends on it today
(`leaves()` formats, `apply_leaves` parses). The model path does not: `Valid(T)` is typed
and `write_value_into` does `partial.set(t.clone())`. **A user-supplied scalar cannot be
checked** — `RecordId` and friends must uphold it in their own facet vtables, with nothing
enforcing it. That is a real contract on library users and belongs in formoxus's docs.

## Runtime gotchas

- **`Signal::new` panics outside a Dioxus runtime** ("Must be called from inside a
  Dioxus runtime"). Verified directly. This is why real formoxus's `use_form`/
  `use_field_set` are hooks, not constructors. Moot for the current design since signals
  stayed out — but it's why a signal-creating builder would have to be `use_form_for`.
- **Rules of hooks vs. a runtime-discovered field count:** `use_signal` in a loop is
  illegal, but `use_hook` runs its initializer exactly once inside a live runtime, so a
  *single* hook call can mint N signals and the hook count stays 1.
- **`dioxus::prelude` exports its own `Location`**, which collides with a model type of
  that name under a glob import. Will bite again when this moves into a real Dioxus crate.
- **`FormValue` is `Text(String) | File(Option<FileData>)`** with no accessor method —
  match it. `File` is also how file upload would arrive, which is one of the gaps on
  [[formoxus_feature_parity]]; it'd slot in as a different `FormMember` kind.
- **Dioxus CATCHES panics raised during a component's render** (that is what
  `CapturedPanic` is). So a test callback that panics on an unexpected call is
  useless — the panic never reaches the test. Found via `#[should_panic]`
  reporting "test did not panic as expected". A recording callback is the answer.
- **Event listeners register against `PlatformEventData`, not the event's data
  type.** The `oninput` attribute macro converts inside the handler, so passing a
  `FormData` to `handle_event` fails the downcast with an opaque
  `Any { .. }`. Dispatching a real event in a test needs
  `PlatformEventData::new(Box::new(SerializedFormData::new(..)))` plus
  `set_event_converter(Box::new(SerializedHtmlEventConverter))` — a platform
  normally installs that converter and a bare `VirtualDom` has none. It lives
  behind dioxus-html's `serialize` feature, added to formoxus as a **dev**-dep
  (verified it doesn't leak: web and wasm32 still build). Get the listener's
  `ElementId` from `Mutation::NewEventListener` in `rebuild_to_vec()` rather than
  hard-coding it.
- **`Store<HashMap>::get` tracks the map SHALLOWLY** (it calls `contains_key`), so
  reading a value through it re-renders on any insert anywhere. `get_unchecked`
  doesn't track; pair it with `try_read` so a missing key reads as `""` instead of
  panicking. And **write THROUGH the child store** (`set`) — `insert` calls
  `mark_dirty_shallow` and re-renders every other input, so it's the fallback for
  a never-populated path only.
- **A plain `Store<HashMap>::read()` subscribes DEEPLY** — that is what makes a
  whole-map read cost a re-render per keystroke.
- **`Signal::write` needs `&mut self`.** Copy the `Copy` handle into a mutable
  binding and write through the copy; that is what lets a method keep `&self` and
  the wrapper stay `Copy`.
- **`dioxus::prelude` re-exports `warn!`** from `dioxus_logger::tracing` — no
  `tracing` dependency needed.
- **Pico zeroes `fieldset { padding: 0; border: 0 }`**, which is why a `<legend>`
  alone shows no box. main.scss compiles AFTER pico, so a bare `fieldset` rule
  wins on source order, not specificity — both are (0,0,1).
- **`FormData::get(name)` returns `Vec<FormValue>`** — all values for a name, not the
  first. Relevant to repeating groups: HTML allows repeated names, so a Vec could use
  either indexed or repeated names.

## `Form` vs `FormState`, and why it isn't a schema (2026-09-07, `44226c9`)

`Form<T>` is what a page holds — `Signal<FormState<T>>` + the value store +
`Callback<Edit>`, `Copy`, built by `use_form`. `FormState<T>` is the plain data
underneath: structure, typed values, errors.

**And answers.** That's the whole naming argument. A fieldless enum variant
contributes nothing to `leaves()`, so `Grading::AllowTwoChances` exists *nowhere*
but in the tree. Once the variant became an in-form answer, "schema" stopped
being true — which is why every doc written before then calls it the schema.
`FormState` also matches the derive path, where `#[derive(Form)] struct LoginForm`
generates `LoginFormState` for the same concept. No collision: that one is a trait
in `formoxus::form`, this a struct in `formoxus::reflect`, and nothing imports both.

`FormData` and `FormValue` were the other candidates and are **ruled out** —
`dioxus::prelude` owns both and every reflect file glob-imports it.

**Renaming trap:** `\bForm\b` also matches `#[derive(Form)]` in prose about the
*derive* path. The compiler can't catch that. Re-grep after any rename that
touches doc comments.

## `use_form` takes the state by VALUE, not a closure (2026-09-07, `fd12c49`)

    use_form(empty_form::<Question>())   // create
    use_form(form_for(&question))        // edit

One hook, not the derive path's `use_form`/`use_form_from` pair. Taking a value
lets ONE component serve both modes, because the branch happens while building
the argument — not a hook call, so the rules of hooks don't care. Branching
across two different hooks would work today (same calls, same order) and break
silently the moment one of them changes.

Rejected `use_form(model: Option<&T>)`, which maps straight onto the existing
`form_for_impl(Option<&T>)` and is cheaper, **on the turbofish**: `T` can't be
inferred from `None`, so create mode reads `use_form::<Question>(None)`. Passing
the constructor puts the turbofish where it names the mode.

Two costs, both real:

1. **The argument is rebuilt every render and dropped after the first**
   (`use_signal`'s initializer runs once). How often depends on what the holding
   component *reads* — MEASURED: a component reading the whole value map renders
   **twice** for one keystroke, one that doesn't renders **once**. So a deep
   `Store<HashMap>::read()` means a rebuild per keystroke. `form_demo`'s debug
   table does exactly that and is the bad case.
2. **It does not track the value.** For a model from a `use_server_future`, put
   the form in its own component taking the model as a prop (it then can't mount
   before the data exists) and give it a `key:` so a *different* model remounts
   it. That's already this codebase's pattern — see `QuestionsForCourse`.

## `$Variant` path segments (2026-09-07, `c0dc22c`)

Two variants sharing a field name both claimed `footprint.size`. The value map
survives structural edits *by design*, so switching Circle -> Square handed the
Circle's number straight to the Square: `Some(Plot { footprint: Square { size: 5.0 } })`.
The `FormState` rebuild was always correct — the whole failure was the store
keeping a key the new variant then claimed. Without a name collision a stale key
is provably inert, since `apply` never asks for a path the schema lacks.

Children now live under a descriptor: `footprint.$Circle.size`. `$` can't begin a
Rust identifier. `model_path()` strips them, so "paths mirror the model modulo `$`
segments" is executable. An all-digit name is a `ListSet` row index and gets no
label — same guard, in `default_label`.

**This is NOT the rejected `$variant`-marker idea.** That one made the variant a
submitted *value* to be parsed back; all three recorded objections are about
parsing names *in*. A descriptor segment is output only.

Contained to `VariantSet`'s four path sites because **`write_value_into` doesn't
use paths at all** — it addresses by `begin_field(&name())` through `Partial`. The
build-time `prefix` threaded through `build.rs` is likewise vestigial: no member
stores one.

**Consequence: switching a variant is now non-destructive.** Switch away, the old
keys are inert; switch back, they're intact. The planned destructive-switch
warning is RETIRED, not deferred, and "drop keys under the prefix" evaporated.

## `Edit` is the one structural-edit entry point (2026-09-07, `65fe469`)

`FormMember::edit(&mut self, prefix, &Edit)`, one method, not one per edit kind —
the containment walk is identical regardless of the edit, and `FieldSet`/`ListSet`
already carried identical copies of it. `Edit::path()` is all a container reads,
so containers stay kind-agnostic. `Form::choose_variant(path, Option<&str>)`
survives as a convenience wrapper (21 callers, and it reads better in tests).
`None` clears back to `Unchosen` — there was previously no way to un-answer.

**Deleted `FormMember::choose_variant` as dead code**, and it was hiding a bug:
`FieldSet`'s wrapper passed `qualify(prefix, &self.name)` where every other passed
`prefix`, so the name landed twice. Clippy says nothing about an uncalled method on
a `pub trait`.

## `RenderCtx` has three kinds of field

`render(&self, ctx: &RenderCtx)` — for `FormState` too, so the root and every
member take the same thing.

| field | on descent |
|---|---|
| `prefix` | **grows** (`nested`) |
| `required` | **flipped once**, only by `OptionMember` (`optional`) |
| `values`, `on_edit` | **invariant**, carried unchanged |

`required` is a *presentation hint, not the authority*: HTML5 `required` is
per-input and can't express the all-or-nothing rule an optional struct follows, so
an optional struct's leaves render UNMARKED and `validate()` stays the rule.
Marking them would block a deliberately blank address.

## What the merge let the reflection path borrow

Concrete payoff, three so far: `FieldErrors` (verbatim, possible only because the
merge unified the error modules), `label_case::ToCase` for `default_label`, and the
shared `crate::error` types.

## The variant `<select>` works, and what it cost to prove (2026-09-07)

The whole loop closes: a DOM `change` -> `VariantSelect`'s `onchange` -> `Edit::ChooseVariant`
-> `use_form`'s callback -> `Signal<FormState>` write -> subtree rebuild -> re-render.
Four things came out of building it that are not visible in the finished code.

**Both arms of `VariantSet::render` MUST be one rsx template.** The select is
rendered unconditionally and `members` is simply an empty `Vec` when unchosen —
never a `match` that returns different markup per arm. With two templates,
dioxus replaces the whole node on every change, and switching between two
*fielded* variants (`Circle{size}` -> `Square{size}`) produced an identical VDOM
and **zero mutations** — the diff never reached the members at all. Measured, not
reasoned: the pre-fieldset code is broken this way and the current code is not.
Secondary benefit: the `<select>` survives its own edit, so a browser doesn't
lose keyboard focus mid-interaction.

**SSR-based tests cannot see this class of bug.** Every render assertion goes
through `dioxus_ssr::render`, which serializes the VIRTUAL dom. A browser applies
the *mutation stream*. A correct VDOM with an empty or wrong edit list looks
perfect to every such test and is broken on the page. `Harness::fire` therefore
returns the edit count, and `switching_variants_actually_edits_the_dom` asserts a
switch both changes the render and emits mutations. **When a render bug shows up
in the browser but not in tests, dump `render_immediate_to_vec()` — that is the
gap.**

**Listener registration order is NOT document order.** It is the order dioxus
creates dynamic nodes. A test that picks "the nested select" as `listeners[1]`
fires at the wrong widget. Identify a revealed widget by the fact that it
*appeared* (diff the listener set across the edit) rather than by position.

**A structural `rsx!` change needs a full `dx serve` restart.** Hot reload
patches templates in place; wrapping a subtree in a new element changes the
template's root and dynamic-node layout while the running client still holds the
old template and live element ids. Symptom: the page misbehaves in ways no test
reproduces. Cost us a false alarm — the fieldset was blamed for a break it had
actually fixed.

**Widget shape.** `VariantSelect` reads NOTHING from the value store — a variant
choice isn't a leaf, it lives in `VariantSet::choice` — so it is a pure function
of props plus `on_edit`. It carries no `name`, which is what keeps the choice out
of `FormData::values()` and keeps [[facet-form-design-decisions]]'s "the choice
is never a submitted value" invariant true. The derive path's `SelectWidget` did
NOT translate: it is a lens over `Store<FormField<T>>`, and there is no such lens
here. What did translate is its required/optional asymmetry — required gets a
`disabled hidden` placeholder (no way back to unchosen), optional gets a
selectable `--none--` that emits `""`.

## The widget boundary is `(path, label, required, errors)` + one channel (2026-09-08)

Todd's question, asked while starting the `InputKind` branch and then parked:
the derive path's widgets all take `Store<FormField<T>>` + `FieldProps`; can the
reflection path's take `reflect::fields::FormField<T>` + path + values + required?

**No — `FormField<T>` cannot cross the widget boundary here, and the reason is
the whole point of `InputKind`.** `InputKind` is a RUNTIME witness to a
TYPE-LEVEL fact the compiler has already discarded. In the `InputKind::Checkbox`
arm of `ScalarInput`, `T` really is `bool`, but nothing tells rustc so, and no
bound can be added that would. The derive path's `Store<FormField<T>>` works
precisely BECAUSE the widget is selected by `T` (`impl FieldWidget<bool> for
CheckboxInput`); selecting by a runtime enum gives that up by construction.
Consequence: **every reflect widget is non-generic and stringly-typed at its
boundary.**

Second, independent reason: `Store<FormField<T>>` does TWO jobs on the derive
path — value carrier AND write channel (a lens). Reflection splits them. Live
values are `ValuesByPath` keyed by `path`; the `FormField<T>` inside `FormState`
is plain data behind a `Signal`, so passing it by value yields a snapshot, not a
channel. A widget must NOT read `FormField::value`: it only refreshes on
`apply()`, so it is stale between keystrokes. `values[path]` is the live one.

**What the four existing widgets actually share.** Diff `ScalarInput` against
`VariantSelect`: both take `path, label, required, errors`. Then exactly one
channel, and which one it is says what kind of widget it is —

| channel | edits | widgets |
|---|---|---|
| `values: ValuesByPath` | a **value** | `ScalarInput` |
| `on_edit: Callback<Edit>` | the **shape** | `VariantSelect`, `AddRowButton`, `RemoveRowButton` |

plus widget-specific extras (`input_kind`; `variants`/`selected`; `index`). So
the `FieldProps` analogue is those four fields, with `path` carrying far more
weight than anything in the derive `FieldProps` (identity + store key + the HTML
`name`; on the derive path identity WAS the lens). Three of the four come off
`RenderCtx`, so it reduces to one helper —
`ctx.props_for(&name, self.label(), self.errors.clone())` — and shortens every
member's `render`. `placeholder` has no source yet (no `#[form(…)]` attrs in
reflect) so it stays out. Take it as ONE struct prop, not spread fields: same
memoization either way (`#[component]` compares whole props by `PartialEq`), but
a library gains the ability to add a field without breaking third-party widgets.

**Rejected: pre-lensing the store.** Passing `Store<String>` from
`values.get_unchecked(path)` would look most like the derive path. It throws away
the parent map, which the widget needs to choose `set` (key exists) over `insert`
(never populated) — and never-populated is a NORMAL state, not an error, since a
variant chosen after mount reveals leaves nobody has typed into.

**The one-template rule does NOT bind `ScalarInput`.** `VariantSet::render` must
be a single rsx template because its arm changes AT RUNTIME. `input_kind` is
fixed by the type at build time and can never change for a given path (different
variants get different `$Variant` paths), so one `rsx!` per kind is safe. Worth a
test rather than an assumption.

**OPEN, and it blocks the checkbox arm: is an unchecked box `false` or
unanswered?** As the code stands, an untouched checkbox looks answered and errors
as required. Traced: `collect_leaves` emits `(path, "")` for an `Empty` field, so
the store has the key with `""`; `apply_leaves` reads `""` and sets `Empty`;
`validate` fires "This field is required" — while the box renders unchecked, and
no gesture clears it back to unanswered. For a bare `bool` the answer must be
`false`, so the checkbox arm has to seed the store or map empty -> `false`. This
is also the sharpest statement of why `Option<bool>` needs a select: a checkbox
has no third state, and "" IS absence has already spent the empty one.

## Customizing a form when the model isn't a form (2026-09-09, design only)

Two populations of types need form customization, and they want different
mechanisms:

**A. Structs written to BE forms** (`LoginForm`, `ChangePasswordForm`) — put
`#[facet(...)]` attributes right on them, as the derive path did. facet supports
namespaced extension attributes (`#[facet(orm::primary_key)]`), and `Field` has
both `attributes: &'static [Attr]` and `flags: FieldFlags`.

**B. Domain models that must not know how they're displayed** (`Question`) —
customization has to come from the call site. This is the hard one, and the
resolution is that **paths are the wrong primary axis.**

**Type-directed registry, not path-directed.** Almost everything you want to
customize is a property of the TYPE, not the field: `Markdown` always wants a
markdown editor, `Ref<Source>` always wants a source picker. So:

    form_for(&question)
        .widget::<Markdown>(MarkdownInput)
        .widget::<Ref<Source>>(SourcePicker::with(sources))

Fully type-checked, no strings, no macro. This is exactly the derive path's
`DefaultWidget` ("the default widget for a field value type") freed from the
orphan rule, and it composes into one app-level config reused by every form. It
is also the answer to `InputKind` being a closed enum: builtins stay closed, the
registry is the open extension point.

**Newtypes cover "same type, different widget".** `#[repr(transparent)] struct
Password(String)` gets its own shape for registry purposes. Rule for the build
walk: check the registry for this exact shape; if nothing is registered and
`shape.inner` is `Some`, recurse on the inner shape.

**`#[repr(transparent)]` is REQUIRED and its absence fails silently.** Read from
facet-macros-impl `process_struct.rs`: `Shape::inner` is populated only under
transparent semantics — `#[facet(transparent)]`, or `#[repr(transparent)]` on a
tuple struct with <= 1 field. A bare `struct Password(String)` is an ordinary
tuple struct: `inner` is `None`, `scalar_type()` doesn't fire, and the walk falls
through to `UserType::Struct`, producing a `FieldSet` with one leaf at
`password.0`. It compiles and renders — just wrong. Wants a test.

**`#[facet(sensitive)]` is a BUILTIN flag** with `Field::is_sensitive()` in O(1).
It means "redact in debug output", which is true of a password independently of
forms — so it can drive a password input with no new attribute machinery and no
purity violation on a domain model.

**The residue after all that is small:** per-field labels (mostly covered by
`default_label` humanizing the field name), placeholders/help text, and field
ORDER — the one thing nothing above solves, since a domain model's declaration
order becomes the form's order. Build the registry first, port `Question`, and
see what is actually left; ordering may argue for a form-purpose struct for
`Question` after all, which costs nothing extra.

**Correction to a premise: a macro CAN check paths at compile time.** The
objection was "we have no reflection until runtime, so a path typo is a runtime
error". Not so — the macro doesn't need compile-time reflection, only a
type-checked witness. Both verified 2026-09-09:
- emit a never-called `fn _witness(r: &Question) { let _ = &r.venue.street; }`
  next to the runtime string; a typo is `no field 'stret' on type 'Venue'`.
- `offset_of!(Question, venue.street)` works with nested paths, and facet's
  `Field` carries `offset` — so customizations could match by offset, not string.
  More elegant, but breaks through `Vec` elements and enum variants, where the
  witness still works.
Do NOT build this first; it is the escape hatch for the residue, not the
mechanism.

## Retiring the derive path — the real gap list (2026-09-09)

Todd wants the derive path gone within a day or two. The live surface is far
smaller than the file list suggests: `views/questions/forms.rs` and
`views/questions/edit.rs` are ENTIRELY commented out (lines 1-130 and 1-58). What
actually uses it is `LoginForm` (2 String fields), `ChangePasswordForm` (3 String
fields + a cross-field validator), and `ui::MarkdownInput`, whose only consumers
are those commented-out files. No bool, no Vec, no enums — today's `InputKind`
branch is not on the critical path for either form.

Gaps, smallest first:
1. **Password fields** (4 uses) — free via `#[facet(sensitive)]`. Plumbing note:
   `scalar_member` takes `ScalarType`, not the `Field`, so the flag has to reach
   it from `member_for`.
2. **Title** — `FormState.title` already exists and renders `h2.form-title`;
   nothing sets it. Needs a setter.
3. **Form-level validator** (`validator = check_new_and_confirm_match`) —
   `FormState.errors: Vec<FormError>` exists and `has_errors()` reads it; nothing
   populates it. An optional `fn(&T) -> Vec<FormError>` closes it.
4. **Buttons — probably ZERO work.** The derive path generates
   `LoginFormHandlers { sign_in: ... }` so handlers can be typed. The reflect path
   does not need that: the page has `form` in scope and writes its own `<button>`
   calling `form.validate()`. `form_demo.rs` already does exactly this. Different
   idiom, not a missing feature.
5. **Server-side re-validation — the one real design decision.**
   `ChangePasswordForm` crosses the api/web boundary as a generated
   `ChangePasswordFormState` with Serialize/Deserialize. `FormState<T>` holds
   `Vec<Box<dyn FormMember>>` and cannot serialize. Natural reflect answer: send
   the model `T`, return `Vec<(path, String)>` and apply onto the tree.

**Honest caveat:** the reflect path has NO custom-widget mechanism today —
`InputKind` is a closed enum, so a downstream type can't add a variant, which is
what `FieldWidget<Markdown>` does. Same for `ProvidedWidget` (the `Ref<Source>`
picker needing externally-supplied choices). Deleting costs nothing today since
those consumers are commented out, but it removes the reference for two problems
`Question` hits head-on. Decide the custom-widget story (the registry above)
BEFORE starting `Question`, not after.

## The widget layer, settled 2026-09-09/10

**Three flags travel through the build walk and behave differently on purpose.**
Getting this wrong produced a real bug the same day it was introduced:
- `FormMode` — fixed at the root, threaded down UNCHANGED.
- `RenderCtx::required` — set once by `OptionMember`, INHERITED by every
  descendant. Deliberately lossy: it is only a presentation hint, and an optional
  struct's leaves must not be individually marked required.
- the build walk's `optional` — set by `option_member`, CONSUMED by the member it
  describes. Every container (`struct_member`, `list_member`, `enum_member`) must
  reset it to `false` when building children.

The bug: `optional` was threaded like `FormMode`, so `Option<Venue>` gave every
field of `Venue` `optional: true`, and a plain `bool` inside it rendered the
tri-state select — offering "No Value" for a state the type cannot hold. Pinned
by `a_plain_bool_inside_an_optional_struct_still_renders_a_checkbox` and its
opposite `an_optional_bool_renders_a_tri_state_widget_instead`, so a fix can't be
"stop setting it". Nothing else caught it: `optional_containers` was all strings.

**A leaf reads its own value; only a non-leaf takes it as a prop.** `SelectInput`
reads from `path` + `values` like `TextInput`. That is NOT mere consistency —
computing `selected` for a prop means reading the store in `FormField::render`, a
plain fn with no scope, which subscribes THE CALLER and re-renders the whole form.
`VariantSelect` takes `selected` as a prop precisely because a variant choice is
not a leaf and has no path to read; it costs nothing there because a structural
edit rebuilds the subtree anyway. Corollary: `SelectInput` carries `name`
(`apply_form_values` must see it), `VariantSelect` must NOT.

**Option values are raw strings, not indices.** The derive path's `SelectWidget`
stores an index into `choices` because its `T` may not survive a string round
trip. The reflect path GUARANTEES `T -> String -> T` (the `roundtrip` module), so
the indirection — and its silent `unwrap_or_default()` on a mismatched index — is
pure loss.

**`type="number"` rejected, and `inputmode` with it.** `type="number"`: browsers
return `""` for anything they consider invalid (a half-typed value vanishes),
decimal commas break in many locales, the scroll wheel edits the field.
`inputmode="numeric"/"decimal"` is the safe half, BUT iOS shows no minus sign on
those keypads, so it would only be applicable to unsigned types — and Todd
rejected it on the grounds that some fields getting a keypad and others not is
worse than none doing. `Int`/`Float` therefore render exactly as `Text` today;
the kinds still earn their place by recording what a field IS for validation.
`pattern="[0-9]*"` is also out: it enables HTML5 validation, which is the same
browser-decides-what's-valid problem.

**A checkbox never gets HTML `required`.** There it means "must be ticked", which
is not what a required `bool` asks — unticked is a complete answer. No ` *`
marker either, for the same reason: it would promise a rule nothing enforces.

**Provider timing — the answer to "choices come from a server call".** Three
distinct times, and the design only feels impossible when two are collapsed:
- SHAPE time (build walk) — what the TYPE implies. `InputKind`. `Option<bool>`
  qualifies; a `Ref<Source>` picker's options never can.
- CALL-SITE time (registry) — which widget a type gets.
- RENDER time (provider) — the DATA that widget needs.
The derive path already had this: `Provider<C> = Rc<dyn Fn() -> Pin<Box<dyn
Future<Output = C>>>>`, supplied at `render`, called lazily, `Rc` so a repeating
group can share one across rows. It fits the reflect path unusually well, since a
picker is its own component and can `use_resource(provider)` on itself.

## An unticked checkbox: `Empty` keeps one meaning (2026-09-10, `8631631`)

A required `bool` could not be submitted. `collect_leaves` emits `(path, "")`
for an `Empty` field, `apply_leaves` reads `""` back as `Empty`, and
`FormField::validate` calls `Empty` required — so a correctly filled form with
one box left alone returned `None`.

**The one-line fix was tried and reverted, and the revert is the finding.**
Reading `""` as `"false"` in `apply_leaves` closes the symptom and opens a
worse hole: the field becomes `Valid(false)`, therefore permanently
[present](`FormMember::is_present`), and `FieldSet::is_present` is `any` over
its members — so an `Option<Struct>` whose only discriminating field is a
checkbox builds `Some(..)` for a section the user never opened. Wrong data
beats an annoying form, and it is the harder failure to notice. The spike memory
had *half* seen this: it warned about exactly this hazard for a BUILD-TIME
`Valid(false)` seed, then failed to apply the same reasoning to the apply-time
version, which has the identical flaw one level up. **Lesson, same shape as the
2026-09-05 one: a hazard recorded against one implementation site is usually a
hazard about the VALUE, not the site.**

**What shipped:** `Empty` keeps meaning "nothing supplied" everywhere, and the
two CONSUMERS know about checkboxes instead — `validate` doesn't call an
unticked one required, `write_value_into` synthesises `false` at the last
possible moment. Both read one predicate, `FormField::is_unticked_checkbox`,
so the rule is stated once and the rejected design is recorded on its doc
comment. `is_present` needed no special case at all, which is the whole point.

**Synthesising goes through `parse_scalar::<T>("false")`, not
`partial.set(false)`.** This is the general answer to "I need a `bool` here but
the compiler doesn't know `T` is one", which will recur. It cannot be made to
know: `InputKind::Boolean` is RUNTIME evidence (the build walk read it off the
shape) and runtime evidence never yields a static type equality. A `T: From<bool>`
bound would propagate to every model field in the crate; `std::any::Any` would
work (`T: 'static` is already there) but bolts a second reflection mechanism
alongside facet's. The string is already the universal channel — `T -> String
-> T` is pinned as the identity for every builtin scalar — so `"false"` is a
legitimate way to say `false`, and the parse vtable resolves the real shape.

**`Boolean { optional: true }` is excluded on purpose.** An `Option<bool>` is a
tri-state select whose blank really is absence, so there `Empty` must reach
`None` untouched.

### The rejected alternative worth remembering

Todd proposed making a required `bool` a three-choice widget (unanswered /
checked / unchecked) so the *user* could express the distinction, since a
checkbox renders identically whether untouched or deliberately unticked — the
store's three states (`""`, `"true"`, `"false"` after tick-then-untick) are
mechanically real but semantically meaningless, because the user cannot see
which one they are in. Genuinely strong: it REMOVES a special case, since every
other widget distinguishes untouched from answered and the checkbox is the sole
exception. Suggested refinement was radio buttons rather than a select —
unselected radios are HTML's native "unanswered", so there is no placeholder
option to find words for, which is where True/False reads worst.

Rejected because forcing an answer on every boolean is a real UX tax when the
field is a **flag with a default** rather than a **question** — and the shape
cannot tell those apart. That is a per-field call, i.e. the widget registry.
Todd's deciding argument: presence only breaks when a checkbox is a struct's
sole discriminating field, and `Option<bool>` is niche enough that it should
never be a type's only property. **If the registry ever lands, "this bool is a
question, render radios" is the escape hatch to build.**

Note it would also have overturned `a_plain_bool_inside_an_optional_struct_
still_renders_a_checkbox`, whose stated reason ("offering 'No Value' for a state
the type cannot hold") does not actually survive the reframe: the third option
is not *no value for the field*, it is *not yet answered* — form state, not type
state. The test still earns its place for the flag-leak bug it caught.

## Retiring the derive path, part 1: the widget taxonomy (2026-09-10, DESIGN ONLY)

**Nothing is implemented. This section is a handoff — it ends with an open
question Todd has to answer before code starts.**

### What the live surface actually is (verified, not remembered)

`LoginForm` (2 `String`s, one submit button, a title, and form-level errors the
PAGE pushes after the server call — `form.errors().push(FormError(..))` for
invalid credentials / inactive / no-roles), and `ChangePasswordForm` (3 password
fields, cancel + submit, the `check_new_and_confirm_match` cross-field validator,
and the api/web round trip). `views/questions/forms.rs` and `edit.rs` are
entirely commented out; `ui::MarkdownInput`'s only consumers are those files.

**Two prerequisites already exist** — checked, don't re-derive: `FormState` has
`title: Option<String>` AND `errors: Vec<FormError>` (form.rs:146-152), so the
title and form-error gaps are plumbing, not new structure. And facet 0.46.5 does
have `#[facet(sensitive)]` with an O(1) `Field::is_sensitive()`.

### Todd's reframe #1: `InputKind` is the wrong shape AND the wrong name

Proposed instead of a flat `InputKind`: **`WidgetType`** whose variants are HTML
*elements* — `Input(InputType)`, `Select`, `TextArea`, `Meter`, `Progress`, … —
with `InputType` carrying `Text | Password | Email | Search | Tel | Url | Range |
Date | Time | …`. Mirrors HTML's own element→type structure instead of flattening
it, and extends along the axis that actually varies.

**The consequence this exposes (raised by Claude, not yet decided):** `Int { min,
max }` and `Float` are NOT widget types. Under the new taxonomy both collapse to
`Input(Text)` — which is already how they render — and `min`/`max` have nowhere
to live. Today's enum is doing two jobs at once:

```
WidgetType::Input(InputType::…)   // what element renders
Constraint { min, max, … }         // what values are valid
```

Two fields on `FormField`, not one enum. The doc comment already admitted this
without noticing: `Int`'s bounds are "for the error message, not for HTML
`min`/`max`". So the rename EXPOSES the seam rather than creating it, and the
constraint extraction is the same work whenever it happens.

Also flagged: `Meter`/`Progress` are *output* elements. Including them widens the
type from "how the user edits this" to "how this is displayed" — possibly wanted,
but it should be a decision, not a drift.

Note `WidgetType::Select` REAPPEARS after `InputKind::Select` was deleted the
same day, and that is consistent rather than a reversal: the deletion was about a
*shape* not being able to imply a select's OPTIONS, while `Select`-as-element is a
different claim. `Boolean { optional: true }` (which already renders a select)
becomes expressible as one honestly. Data-driven pickers still belong to the
registry.

### Todd's reframe #2: don't key off `#[facet(sensitive)]`

His words: "We're not going to be using Facet for serialization/deserialization,
so I'd prefer form-centric attributes." The objection holds up on its own terms
too: `sensitive` means "redact in debug output", so keying password rendering off
it means marking a field sensitive for LOGGING silently changes its widget, and
wanting a password input forces debug redaction. Correlated concerns, not the
same one, and coupling them stops either moving independently.

### The two candidate mechanisms (THE OPEN QUESTION)

**A. facet extension attributes.** `#[facet(formoxus::password)]` is real,
supported syntax: `#[facet(ns::key)]` lands in `field.attributes` as `Attr { ns:
Some("ns"), key, data }`, typed data via `get_as::<T>()`.
**The cost, which is the part worth remembering:** `#[facet(ns::key)]` expands to
`::facet::__ext!(ns::key …)` which routes to **`ns::__attr!`** — so formoxus must
export an `__attr!` macro and an `Attr` alias at whatever path is used as the
namespace (facet-macros-impl-0.46.5/src/extension.rs:147-170). A small proc-macro
surface, on a pre-1.0 mechanism that can churn — and shedding macro machinery was
part of why the reflection path exists.

**B. The call site says it.** `form_for(&model).widget("password",
InputType::Password)`. A `String` does not IMPLY password — that is an authoring
decision, not a shape fact, so it belongs at CALL-SITE time, which is exactly
where the registry already puts widget choice ("Provider timing", above). No
facet coupling, no macro surface, and it is the registry arriving early rather
than a new mechanism. The trade: verbosity, and the widget choice no longer sits
beside the field it describes.

### Where to start once A-vs-B is answered

The `WidgetType`/`InputType` split plus the constraint extraction, because Login
cannot be rewritten without password support and everything ELSE Login needs is
plumbing: title (setter for an existing field), form-level errors (an existing
`Vec<FormError>` nothing populates), buttons (zero work — the page owns `form`
and writes its own `<button>`, as `form_demo.rs` already does).

**Then the one genuinely hard piece, still undesigned in detail: the api/web
crossing for `change_password`.** Today `ChangePasswordFormState` is
Serialize/Deserialize, goes to the server, gets `.current_password.add_error(..)`
stamped on it, and is written back wholesale into the store.
`reflect::FormState<T>` holds `Vec<Box<dyn FormMember>>` and cannot serialize.
Sketched answer: send the MODEL, return path-keyed issues, apply them onto the
tree — needs a `FormState::apply_errors(&[(path, msg)])` dispatching by
containment the way `edit()` already does, and a channel for FORM-level errors
too (`check_new_and_confirm_match` yields a `FormError`, not a field error), so
probably a small struct rather than a bare `Vec<(String, String)>`.
**A nice property of sending the model:** parse-level validation is then enforced
by deserialization itself, and only semantic rules (passwords match, strength)
need re-running server-side.

### SUPERSEDED — read [[formoxus-widget-survey]] instead

The section above ends on an open question that was answered the next night, and
the design moved well past it. `#[facet(formoxus::password)]` won for
form-purpose structs (with a separate, still-undesigned mechanism for independent
models), the `__attr!` cost recorded above is too high (`define_attr_grammar!`
generates it), and the constraint axis landed differently: constraints nest in the
VALUE kind, not in `WidgetType`, because a presentational override must not
discard validation. `InputKind` turned out to be misnamed rather than mis-shaped.
Full record, with the surveys and the verified facet findings, in
[[formoxus-widget-survey]].

## The `form2!` decision, and `FormSpec` (2026-09-12, through `9d5482e`)

### Facet extension attributes lost to formoxus's own proc macro

Todd's reframe, after the attribute mechanism had been built and proven: "Maybe
we just forget about using facet extension attributes at all." He was right, and
the three reasons are worth keeping because two of them were invisible until we
had built the losing option.

1. **The orphan rule dissolves.** `form2!` expands in the CONSUMING crate, so
   `prompt => widget MarkdownInput` just works — both `Markdown` and
   `MarkdownInput` are in scope there. That was the entire reason `DefaultWidget`
   was stuck (`ui/markdown.rs` says so outright: "Because both `Markdown` and
   `FieldWidget` are foreign to `ui`, this can't be a `DefaultWidget`") and the
   entire reason a runtime widget registry was being designed. It demotes the
   registry from required subsystem to optional optimisation.
2. **Buttons need TYPE generation.** The derive path generates a
   `LoginFormHandlers` type from button names. An attribute grammar only records
   data — it cannot generate types, and `name = sign_in` is a bare ident, which
   is not one of its payload kinds. So attributes structurally could NOT reach
   derive-path parity; a proc macro can.
3. **It sheds every sharp edge** listed in [[formoxus-widget-survey]]'s
   correction section — the `Option<T>` wrapping, the non-uniform read protocol,
   struct fields limited to five types, no nesting, `Facet` forced onto
   `WidgetType`/`InputType`, tests exiled from the crate, pre-1.0 churn.

**Costs accepted, both named explicitly.** Presentation no longer sits beside the
field — Todd's own earlier argument for attributes — mitigated by putting the
`form2!` block directly under the struct, which is where Django's
`ModelForm.Meta.widgets` lives too. And `formoxus-macros` becomes permanent
rather than deletable when the derive path retires; it is a much smaller macro
than the `Form` derive, and the crate is already compiled.

**Named `form2!` deliberately**, not `form!`, so the derive path's `#[form(…)]`
keeps working during migration. Rename after `LoginForm`/`ChangePasswordForm`
move over, then delete the derive.

**It is an EXPRESSION macro, not an item macro**, and that is load-bearing: the
expansion is a block, a block may contain items, and those items are scoped to
the block — so the compile-checked witness fn cannot collide between two
invocations. The item-position version needed a `const _: () = { … }` wrapper to
get the same property.

### `FormSpec<T>` is the form's declaration; `FormState<T>` is the built tree

Todd asked why both exist, since both carry a title. They are source and compiled
artifact — both mention a title the way a source file and its AST both mention a
function name. Five differences, none cosmetic:

| | `FormSpec<T>` | `FormState<T>` |
|---|---|---|
| shape | flat, keyed by qualified path | a tree mirroring `T::SHAPE` |
| coverage | partial (only overrides) | total (every member) |
| mode | independent — serves both constructors | blank or populated |
| mutability | never changes | changes on every keystroke and edit |
| may name | members that DON'T EXIST YET | only what exists |

That last row is what forbids unification: a spec entry for
`grading.$PenalizeIncorrect.penalty` is meaningful before that variant is chosen,
and a tree cannot hold it.

**But the question earned a real simplification:** `FormState` holds the
`FormSpec` verbatim, and `FormState.title` is GONE (`title()` reads through). No
third "static half" type, one home for the title.

**Why the spec must be RETAINED rather than consumed at construction — two
independent late-arriving cases:** `Edit::AddRow` builds members at runtime, and
a variant's fields don't exist until it is chosen. Both need the spec re-applied,
which also means `customize` must be idempotent.

**Consequence:** `FormSpec` must stay plain data so `FormState` keeps its derived
`Debug`. Handlers and providers (which genuinely capture) therefore do NOT go in
`FormSpec`; they become a separate render-time argument, as the derive path
already does with `form.render(LoginFormHandlers { … })`. A form-wide validator
is exempt — a plain `fn(&T) -> Vec<FormError>` implements `Debug` and `PartialEq`.

**Strict uniformity, no no-arg constructors.** `empty_form(spec)` /
`form_for(&v, spec)`. One way to build a form, so "where does the title go" has
one answer — and `T` flowing out of `FormSpec<T>` removes the turbofish at real
call sites. Cost: 108 test call sites migrated in one commit.

### `Edit` does NOT become a variant of the spec

Todd asked; the answer is no, and the decisive reason is mechanical rather than
aesthetic. **The spec is stored and re-applied idempotently; `Edit` is not
idempotent** — `AddRow` twice adds two rows, `RemoveRow { index }` twice removes
two different rows. Merging them would make the stored spec a log of user actions
that re-adds every row on the next re-application. The merge destroys the exact
property it was merging toward.

Four supporting reasons: shape vs presentation (already decided once — blur
validation is "a sibling to `on_edit`, not part of it, because `Edit` is about
shape"); author and lifecycle (user/runtime/one-at-a-time vs
developer/compile-time/whole-set); error story (`edit` returns
`Result<(), FormAccessError>` and is pinned by
`a_bad_path_is_an_error_not_a_panic`, while a bad spec path is a COMPILE error via
the witness fn); and the walks differ anyway (`edit` finds one member by path,
`customize` visits all and each consults the spec).

What IS shared is the signature convention — `customize(&mut self, prefix, spec)`
mirroring `edit(&mut self, prefix, edit)`, and `edit`'s doc comment about "one
method rather than one per edit kind" applies verbatim. **Do not add a setter per
property** (`set_widget`, `set_label`, …) — that widens the trait every time
`FieldSpec` grows a field.

Mutation must go through the trait at all because `FormState.members` is
`Vec<Box<dyn FormMember>>` and `FormMember: Debug` only — no `Any`, no downcast,
so nothing outside can reach `custom_widget` on the concrete `FormField<T>`.

### Small but sharp

**`IndexMap`, not `HashMap` or `Vec<(String, FieldSpec)>`.** Ordered (stable
`Debug`; keeps a future field-ordering customization possible) AND overwrites on
duplicate insert, which a `Vec` would not — `.with_label(p, …)` after
`.with_custom_widget(p, …)` must reach the same entry. Its `PartialEq` ignores
order, and `insert` keeps an existing key's position. Already in the lock
transitively, so it costs nothing to build.

**`Default` on a `PhantomData<T>` struct must be HAND-WRITTEN.** `#[derive(Default)]`
adds a spurious `T: Default`, which surfaced as "the trait bound `DemoQuestion:
Default` is not satisfied" at a call site that was itself correct. `Clone` and
`Debug` may stay derived — their spurious bounds are already implied by the
struct's own. This trap is now documented three times in this crate.

**`mut self` builders are the functional shape, not mutation** — a linear value
you consume and produce, `FormSpec -> FormSpec`. The map idiom is
`entry(path).or_default()`, returning `&mut V`; `get()` returns `&V` and
`unwrap_or_else(|| &Default::default())` borrows a temporary that dies.

### Status at `9d5482e`

`FormSpec` exists and is threaded through both constructors, but **only `title`
is consumed**. `fields` and `validator` are authored and read by nothing —
`FormMember::customize` does not exist. Next slice: `customize` on all five
member kinds, re-application after `edit`, and tests for the two late-arriving
cases (added rows, revealed variant fields) since those are what would silently
regress.

## `apply_specs`, and `[]` as a row selector (2026-09-13, through `344347b`)

### Renamed

`customize` -> **`apply_specs`** (Todd's rename; earlier notes in this file and in
[[facet-form-spike]] may still say `customize` — the method never shipped under
that name).

### The signature, and why it is shaped that way

```rust
pub type FieldSpecs = IndexMap<String, FieldSpec>;
fn apply_specs(&mut self, prefix: &str, fields: &FieldSpecs);
```

- **Takes the T-free half, not `FormSpec<T>`.** `FormMember` is object-safe and has
  no `T`. `title` and `validator` are consumed by `FormState` and never reach a
  member.
- **No `Result`.** Unlike `edit`, which dispatches to the ONE member owning a
  path, a spec is a SET of statements: every member is visited and asks the map
  about itself. A path matching nothing is not an error.
- **This is why it is easy to write a non-recursing version by mistake.** `edit`'s
  shape is "test, dispatch, return" — copy it and you get an `apply_specs` that
  applies to itself and stops. That happened; the nested-path test caught it.
- **Containers apply to THEMSELVES before recursing.** A container is addressable
  in its own right: `location => { label: "Where" }` is its legend, not a prefix.
- `VariantSet` recurses via `child_prefix` (children sit a segment deeper under the
  variant descriptor); `OptionMember` forwards the prefix untouched.

### Which members may take a widget

| member | `label` | `custom_widget` |
|---|---|---|
| `FormField` | yes | yes |
| `VariantSet` | yes | **yes** — it owns the `<select>`; a radio group is meaningful, not yet rendered |
| `FieldSet` | yes (legend) | PANIC — no single widget exists |
| `ListSet` | yes | PANIC, with a message pointing at `[]` |
| `OptionMember` | forwards | forwards |

The asymmetry is deliberate. Panics fire at construction (house precedent:
`an_unsupported_scalar_fails_loudly_at_construction`) and name the path.

### `[]` — the row selector

```
venues        => { label: "Venues" }    the ListSet's legend
venues[]      => { widget: … }         every row
venues[].city => { label: "City" }      the `city` field of every row
```

**Why it exists:** a row's real path contains a generated key (`venues.#0.city`)
that no author can write and that would pin ONE row if they could. Without `[]` a
row's fields are unnameable.

**Precedent:** leptos_form does configure the element type through the `Vec` —
`VecConfig { item: Config, … }`, cloned per row
(`core/src/form_component/impls/collections.rs`). But it gives the row config its
own NAMED slot. We copied that separation rather than overloading one entry:
overloading would make a single `FieldSpec`'s `label` apply to the container and
its `widget` to the children, two halves travelling opposite directions.

**How it resolves:** `[]` survives into the map key. `ListSet::apply_specs`
substitutes each row's actual segment (`k.replace("venues[]", "venues.#0")`).
Substitution rather than a second lookup path **because it composes** —
`rows[].cells[]` rewrites one bracket per level as the recursion descends, so a
nested `ListSet` handles its own with no special case, and unrelated keys pass
through. Cost: one map clone per row at construction.

`answers[0]` is REFUSED rather than having the index dropped silently.

### Path expressibility — what the compile-checked witness can reach

The witness fn requires a valid Rust access, which bounds what a path can name:

| path | witness | ok? |
|---|---|---|
| `password` | `&s.password` | yes |
| `venue.city`, `venue: Venue` | `&s.venue.city` | yes |
| `venues[].city` | `for v in s.venues.iter() { let _ = &v.city; }` | yes — element type INFERRED, so the macro needn't name it |
| `venue.city`, `venue: Option<Venue>` | `&s.venue.city` doesn't compile | **no** |
| `grading.$PenalizeIncorrect.penalty` | `$Variant` is not a field | **no** |

Variant fields remain the open case — same problem as rows had, different
delimiter. Paths through an `Option` are also still unreachable.

### Re-application: a note reversed within a day

An earlier note said re-application after an edit could be skipped because nothing
ADDRESSABLE arrived late. True when written; `[]` falsified it hours later.
`add_row` builds a row from the shape alone, so without re-applying, a row added
after mount renders with derived labels while its siblings carry the spec's.

`FormState::edit` now re-applies wholesale after any successful edit. The owner is
located with `position()` BEFORE mutating, so `apply_specs` can run without
holding a borrow of `members` (an `iter_mut()` loop would conflict).

Safe because `apply_specs` is idempotent — **and that is exactly why `Edit` is not
a variant of the spec**: an edit replayed twice adds two rows. The property that
makes this fix one line is the property that killed that merge.

Unmeasured cost: `apply_specs` now runs on every structural edit, and `ListSet`
clones the spec map per row. Trivial at current sizes; first thing to look at if a
form ever gets large and edit-heavy.

## `value: None` on an input is not "omit the attribute" (2026-09-13)

Cost an afternoon, twice over, chasing "I can't type in the password field".

Withholding a password's value (Django's `render_value=False`) was implemented
first as `value=""` and then as `value: None`. **Both made the field impossible to
type into**, and the rendered HTML looked correct in both cases — which is why the
render-only test could not see it.

The mechanism: every leaf input here is CONTROLLED, so `value` is reasserted from
the store on each re-render, and a keystroke triggers a re-render. `value=""`
therefore wipes each keystroke. `value: None` omits the attribute from the
rendered HTML, but Dioxus **re-emits an unchanged `None` attribute on every
re-render**, and `None` for `value` is not "leave it alone" — the interpreter runs
`node.value = ""; node.removeAttribute("value")`
(`dioxus-interpreter-js`'s `remove_attribute`). Same wipe, one layer down.
`aria_invalid` gets away with `None` only because no DOM state hangs off it.

Assert on MUTATIONS, not markup, for anything of this kind:
`dom.render_immediate_to_vec().edits` after dispatching an event shows whether
Dioxus touched the node at all. `reflect::tests::widgets::a_keystroke_writes_the_value_attribute`
is the guard.

**The premise was then dropped entirely, at Todd's challenge.** Django's
`render_value=False` and Rails' non-echoing `password_field` are real conventions
but SERVER-RENDERING ones: the value would land in an HTTP response body, and so
in proxy logs, shared caches, browser history and view-source. Nothing here
carries it there — keystroke -> DOM -> a wasm-side store in the user's own
browser, and binding it back writes to the very node they typed into. No citation
was found for the client-side case, and there was no honest one to give. A
server-rendered response with a populated password would bring the concern back;
this architecture produces none, since the SSR pass renders an empty form and
every later re-render is client-side. So a password is now an ORDINARY CONTROLLED
INPUT — `type="password"` masks the glyphs, nothing else differs. The two
practical arguments that settled it: a withheld value cannot be CLEARED
programmatically (no "reset after a successful change"), and it was the only
asymmetric widget in the set.

## Buttons on the reflection path (2026-09-14)

Design conversation, nothing built. Full writeup is checked in at
`crates/formoxus/BUTTONS_PLAN.md`; this is the part that is expensive to
re-derive.

**`render` owns the `<form>` element** — Todd's call, on the grounds that a view
needing more can always render manually. Costs moving 41 `render()` call sites
that assert on today's fragment, so the shape is TWO methods: `render()` emits
`<form>` + fields + buttons, and a second emits today's fragment as the explicit
escape hatch. What it buys: the submit button needs no `onclick`, because native
`type="submit"` routes a click AND Enter-in-a-field through one `onsubmit` — the
branch `ButtonInfo::button_tokens` already takes, and only reachable if the
library emits the element.

**Declaration in `form2!`, handlers at `render`.** Todd's own reason ("a handler
needs component-local things") is the WEAKEST of the three and does not decide
it — moving the `form2!` invocation into the component body solves it. The two
that do decide it:

- **`use_form`'s thunk runs exactly once**, so a closure built inside it freezes
  its captures at first render. `Copy` handles survive; a plain prop does not,
  and `login.rs` already launders `next: Option<String>` through a `use_signal`
  for exactly that reason. Render-time closures are rebuilt per render.
- **Circularity**: `login.rs`'s submit handler calls `form.push_error(...)`, and
  `form` is what the thunk RETURNS, so a closure inside it cannot capture it.
  Breakable by passing `Form<T>` into the handler, but that is choosing a
  signature to rescue a design rather than because it is right.

Plus: `form2!` is invoked outside any component in ~15 test fns in
`tests/reflect.rs`, so closures in `FormSpec` would end the spec's life as a
plain testable value. The principle was already written down in the
`Provider<C>` doc comment — "supplied at the `store.render(...)` call site
rather than baked into the declaration".

**ONE hidden slot struct, built by ONE macro** — replacing the derive path's
`…Handlers` + `…Providers`. Todd's two objections: a name just appears that
could collide and that you must know the convention for, and it is more verbose
than it has to be. Both are about NAMING, not about the struct — and the struct
earns its keep, because a struct literal is Rust's only free "every field, by
name, in any order" check (`missing field 'cancel' in initializer`).

So: merge handlers and providers into one slot set (this DELETES
`render_with_providers` and the `Providers: Default` bound, which exists only to
let the second argument be omitted); mangle and `#[doc(hidden)]` the ident so it
is reachable only as `<T as FormState>::Handlers`; and build it with a macro
taking `name: closure` pairs plus dotted paths for nested provider groups.

Concrete form of objection 1, worth keeping: the generated structs inherit the
container's visibility, so a `pub` form puts TWO `pub` names into the crate's
public API that nobody asked for.

**The mechanism that makes it ONE macro instead of one per form: a local
`trait IntoSlot<T>` with blanket impls per closure shape.** The macro emits
`IntoSlot::into_slot(expr)` and the FIELD'S TYPE selects the conversion, so the
macro needs no knowledge of validated/unchecked/provider and the author never
writes `handler(...)`. `impl From<F> for Handler<M>` cannot work — `Rc<dyn Fn…>`
is foreign on both sides, coherence rejects it; a local trait sidesteps that.
**UNPROVEN**: whether inference resolves `T` from the field type in
struct-literal position needs a compile test. Fallback if not: `form2!` generates
a per-form macro, which works but gives back part of objection 1.

**Rejected: a builder** (`form.render().sign_in(…).cancel(…).finish()`). Takes
bare closures and adds ZERO names — the best possible answer to objection 1 —
but `finish()` gating on "every slot filled" needs typestate (marker params per
slot, or a const-generic bitmask wanting unstable `generic_const_exprs`), and
those params surface in every error message. Struct-plus-macro gets the builder's
ergonomics with the struct's error messages.

**Open: the macro's name.** `wire!` proposed and REJECTED by Todd. Candidates
drawn from the codebase's own vocabulary: `supply!` (the verb in the prose) or
`slots!` (`slot` is already the code's noun — `FieldMeta::providers_slot`).
`supply!` is the placeholder in the writeup. Still Todd's call.

### What actually got built (2026-09-16)

The two sections above are the design *conversation* and stay as written. This
is what survived contact, and it is not all of it — `BUTTONS_PLAN.md` carries
the same correction in the repo.

**The slot struct is impossible on the reflection path.** Three findings, each
verified with a throwaway compile rather than reasoned about:

1. **Nothing to name.** `form2!` is an EXPRESSION macro producing a runtime
   `FormSpec` value, so at the `render` call site — usually a different function
   — there is no per-form type in scope. `<T as FormState>::Handlers { … }` does
   not close the gap: qualified paths in struct-literal position are still
   unstable (`more_qualified_paths`, rust#86935), so a macro has to name a
   concrete struct.
2. **`IntoSlot` cannot replace a field type.** It works — the UNPROVEN note
   above resolved in its favour, and the trait plus its three blanket impls are
   live in `reflect/form.rs`. But it only works where a field type selects the
   impl. A map has none, and one target type cannot carry impls for both
   `Fn(M)` and `Fn()`: coherence cannot prove a type implements only one, so it
   is a hard `E0119`.
3. **Item position would cost `title:`.** Moving `form2!` to item position (so
   it could declare the struct) kills `title:`'s capture of LOCALS — the
   property that motivated `Expr` in the first place, "a value that does not
   exist until the form is built: a fetched resource, a route parameter, a
   signal", pinned by nine tests in `tests/reflect.rs`. An item cannot capture a
   local.

Todd's call, after seeing all three: **runtime checking.** The reflection path
already fails at render for an unwired widget, so this is the trade it makes
everywhere else.

**The macro is `using_fns!`.** It builds a `Fns<T>` name→fn map and reads each
closure's **arity syntactically** — `|m| …` validates first, `|| …` runs
unconditionally. A macro can see what no trait can. A non-closure is REJECTED,
because a path hides its arity and guessing picks the wrong `ButtonFn` variant
with no error until someone clicks.

**`Fns::reconcile` replaces `missing field 'cancel'`**, and checks two things
the struct literal never could: that a closure's arity agrees with the button's
declared `invocation`, and that a form declares at most one submit button (two
share one `onsubmit`, so the second would never fire). Mismatches render as form
errors with the button left `disabled` — never a panic, since on wasm a panic
aborts instead of reaching an `ErrorBoundary`.

**The `ButtonFn` variant decides how a handler is called, not the spec's
`invocation`.** It is the only thing that CAN — a validated handler takes a
model, an unchecked one takes nothing, and declared intent conjures neither at
runtime. So `invocation:` in `form2!` is a checked assertion, not a directive.

**§4 of the build order is dropped.** The derive path is being retired, so
nothing new goes into it; the two `pub` names objection 1 complains about will
leave with it rather than be fixed.

**Migration gotcha, from `login.rs`.** A submit handler now receives the
VALIDATED MODEL, not a `FormEvent`: `prevent_default` and `validate` both moved
inside `Form::render`. A view coming off a hand-written `<form>` must change its
closure's parameter AND delete the `form.validate()` it used to do itself.
`reconcile` cannot catch that — any one-argument closure is a valid `Validated`
handler whatever its argument type, so it is rustc's to catch.

## `push_field_error`, and how a server verdict reaches one field (2026-09-16)

Built to unblock `change_password.rs`'s migration — the piece
[[next_up_two_todos]] named as blocked. `Form::push_field_error(path, message)`
is the per-field counterpart to the already-existing `Form::push_error`
(form-wide). Both are for a verdict only the SERVER has ("that username is
taken", "current password is wrong") — nothing local can produce it, so
`validate()`'s own field checks can't cover it.

**Shape: a new `FormMember` trait method, dispatched exactly like `edit`, NOT
a `field_at(path)` accessor.** Todd's instinct was to reach for something like
`state.field_at(path).errors.push(...)`, and there is no such accessor —
deliberately, and consistently: nothing in this tree does "get me a mutable
reference at a path," it does path *dispatch*. `FormMember::edit` is the exact
precedent (`owns()` containment check, delegate to the one child that owns it,
`no_such_path` otherwise), so `push_field_error(&mut self, prefix, path,
FieldError) -> Result<(), FormAccessError>` on the trait mirrors it member for
member: `FormField` (the leaf) is the one place `path == qualify(prefix,
name)` actually pushes into `self.errors: Vec<FieldError>`; every container
(`FieldSet`, `ListSet`, `OptionMember`) just walks to the one child that owns
the path, same as their `edit` impls. **`VariantSet` is the one exception**:
unlike `edit`, `path == my_path` (the enum's own path, where the `<select>`
lives) IS meaningful for a field error — `VariantSet.errors: Vec<FieldError>`
already renders beside the select (`self.errors.clone()` in `render`), so a
server complaint about the CHOICE itself ("pick a grading scheme") belongs
there rather than being forced down into a child. `ListSet`'s own `errors:
Vec<FormError>` (note: a different type) can NEVER receive a `FieldError` at
its own path — not by special-casing, just because no row ever qualifies to
exactly the list's own path, so it falls through to a child or `Err` for
free.

**`Form::push_field_error` is `&self`, matching `push_error`/`validate` —
`FormState::push_field_error` is `&mut self`.** Caught twice in review because
it's non-obvious which layer needs which: `Form`'s only touches `self.state`
through the `Signal`'s interior mutability (`state.write()`), so `&self` is
enough and is what lets a `Form<T>` parameter stay usable in a handler without
being declared `mut` (`push_error` in `login.rs`'s `submit_login` is the
existing precedent — `form: Form<LoginForm>`, no `mut`, same call shape).
`FormState::push_field_error`, underneath, mutates `self.members` directly
(no Signal to hide behind), so it genuinely needs `&mut self` — same as
`edit`. Getting this backwards either way is a real compile error, not a
style nit, so it's worth stating the rule rather than re-deriving it: **`Form`
methods are `&self` (write through the `Signal`); `FormState` methods that
touch `members` are `&mut self`.**

**Also cleared by the next `validate()`, and for free** — not a separate
rule. `FormField::validate` already clears and rebuilds `self.errors` on every
leaf each call, so a field error pushed here can't outlive the next submit,
exactly mirroring why `push_error`'s form-level errors don't either.

### `form.rs` split into `form/{spec,state}.rs` (2026-09-16)

Same day, same session: `reflect/form.rs` had grown to ~780 lines holding
three distinct things (`Form<T>`, `FormState<T>`, `FormSpec<T>`) plus the
button/slot machinery, and Todd asked for it split for navigability. Mirrors
the `members.rs`/`members/` pattern already in the codebase: `form.rs` stays
the module root (now just `Form<T>` + `Handler`/`UncheckedHandler`/
`Provider`/`IntoSlot` + `use_form`), with `form/state.rs` (`FormState<T>` +
`form_for`/`empty_form`) and `form/spec.rs` (`FormSpec<T>` + `FieldSpec`) as
siblings, re-exported (`pub use spec::{...}; pub use state::{...};`) so every
external path (`crate::reflect::form::{Form, FormState, FormSpec, ...}`,
`reflect.rs`'s own `pub use form::{...}`) is untouched. The only visibility
change needed: `FormSpec`'s fields (`title`, `fields`, `validator`, `buttons`)
went from module-private to `pub(super)`, since `FormState::validate`/
`apply_specs`/`title` read them directly and `state.rs` is now a sibling
module rather than the same file. Zero behavior change — `cargo test
--workspace` stayed green through the split.

### The `prelude::Form` name collision (a live trap, not hypothetical)

`formoxus::prelude::*` re-exports the OLD DERIVE PATH's `Form` — a
zero-generic-parameter trait (`crate::form::Form`, `type State: FormState`) —
alongside the `#[derive(Form)]` macro of the same name (different
namespaces, which is why the prelude's own comment says a single glob import
can bring both). It does NOT re-export `reflect::Form<T>`. A file on the
reflection path that does `use formoxus::prelude::*;` for `FormError`/
`FieldError` gets this `Form` silently in scope too, and `T: Form<SomeModel>`
looks like a perfectly reasonable generic bound on `reflect::Form<T>` right up
until `cargo check` reports "trait `Form` takes 0 generic arguments" pointing
at a trait you never meant to name. **Bit `crates/api/src/auth.rs` while
migrating `perform_password_change` off the derive path** — fixed by naming
imports explicitly (`formoxus::error::FormError`, `formoxus::form2`,
`formoxus::reflect::FormSpec`) instead of the prelude glob. Any reflection-path
file still importing `formoxus::prelude::*` is worth checking for this.

### Why `Form<T>`/`FormState<T>` can never appear in a server fn's signature

Realized while fixing the above: it's not just the name collision.
`Form<T>` wraps a `Signal<FormState<T>>`, and a `Signal` can only be minted by
`use_signal` — a Dioxus hook, which needs a live component scope. A `#[post]`
handler is not a component, so there is no way to hand one a `Form<T>` at
all, whatever the bound says. `FormState<T>` itself doesn't cross either,
for an unrelated reason: it holds `Vec<Box<dyn FormMember>>`, which has no
`Serialize`/`Deserialize` impl (trait objects), so — unlike the OLD DERIVE
PATH, where the whole `…FormState` was plain, serializable data that could
round-trip the wire wholesale with server-attached errors baked in — the
reflection path can only send the plain MODEL across in either direction.

**Consequence for `change_password.rs`:** `perform_password_change` takes and
returns the bare `ChangePasswordForm` model (already the `#[post]` fn's
param type) plus a small serializable verdict on failure, not a `Form`/
`FormState` of any kind, and doesn't call `push_field_error` at all —
`push_field_error` is purely a CLIENT-side API. The shape landed as an
ad hoc enum:

```rust
pub enum ChangePasswordProblem {
    Field(String, String), // path, message -> client calls form.push_field_error
    Form(String),           // message        -> client calls form.push_error
}
```

**Todd's explicit objection, recorded because it'll shape the next form that
needs this:** he doesn't want every form crossing the server boundary to
invent its own `XProblem` enum. Two directions floated, not yet decided:
(1) one general problem type all forms share, or (2) `form2!` generates an
`XErrors` type per form (maybe gated by a keyword) the same way it already
generates the button-handling machinery. Todd's own observation: if (2)
happens, an `XErrors`/`XHandlers`-shaped struct existing per form might be
reason enough to revisit `using_fns!`'s runtime-checked (`Fns::reconcile`)
button dispatch in favor of compile-time checking again — the struct-literal
approach was rejected earlier only because *nothing nameable* existed at the
`render` call site (see "The slot struct is impossible on the reflection
path" above); a generated `XErrors`/equivalent would be exactly such a
nameable type, reopening that question. **Not designed yet — revisit once a
SECOND form crosses the server boundary**, so the "general" shape is inferred
from two real cases instead of guessed from one.

## `collect_errors`: the contract, and the two bugs it shipped with (2026-09-17)

The read half of the wire story — `FormState::collect_errors() -> FormErrors {
form: Vec<FormError>, fields: Vec<(String, Vec<FieldError>)> }`. Landed
structurally in `9992752` (WIP) with impls on all five members, but with **no
callers and no tests**, and two defects that only showed up once tests existed.
Both now fixed, both covered by a regression test that was verified to fail
against the old code.

**The contract, now written on the trait method** (`members.rs`):

1. **Only members WITH errors appear.** `FormField::collect_errors` had been
   pushing `(path, self.errors.clone())` unconditionally, so a clean
   three-field form serialized as `[("current_password", []), …]` and
   `FormErrors.fields` was never empty. That destroys the one thing the shape
   is for: a caller reading `fields.is_empty()` as "the server accepted it."
   Guarded now, and stated as a trait-wide rule rather than one impl's habit —
   it only means anything if every impl honours it.
2. **A container reports its OWN path when it has errors of its own.**
   `collect_errors` is the inverse of **`push_field_error`**, NOT of
   `collect_leaves`. Whatever can be pushed at a path must be collectable from
   it.

**The `VariantSet` bug is the interesting one, and it was a copy-paste from
the wrong sibling.** Its `collect_errors` had been written as a copy of
`collect_leaves` directly above it — including the `let Some(nested) =
self.child_prefix(prefix) else { return }` early return, which is *correct*
for leaves (an unchosen variant genuinely has no leaves) and exactly backwards
for errors. `validate` pushes `"You must choose a variant for this field."`
**precisely when** the choice is `Unchosen` — so the early return dropped the
one error this member reliably produces, in the one state where it matters.
Three writers put errors at `my_path` and all three were unreachable:
`validate` (unchosen), `lookup_variant` (a stale name from a live `<select>`),
and `push_field_error` (a server verdict about the choice itself — the exact
"pick a grading scheme" case the 2026-09-16 `push_field_error` design section
argues for). The fix emits `qualify(prefix, &self.name)` **before** recursing,
so the collected order matches `render` (which puts `VariantSelect` above
`members`), and keeps the early return only as a short-circuit afterwards.

**The path trap bit here too**: `self.errors` live at `my_path`, one segment
SHALLOWER than `child_prefix`'s `$Variant`. Only `nested` was in scope in the
broken version, so the obvious "just push what you've got" fix would have
named a path no widget renders. This is the third method where `edit`'s "two
paths are in play, and they are NOT the same" warning has come due — more
evidence for the deferred `Path`/`Prefix` newtype in [[next_up_two_todos]].

**Known gap, documented not fixed:** `FieldSet` and `ListSet` each hold a
`Vec<FormError>`, which cannot enter a `FieldError`-typed `FieldErrors`, so
container-level form errors are NOT collected. Latent rather than live —
nothing pushes to either today, `validate` only clears them — but a future
`ListSet` min/max-rows check would be the first writer and would force
`FormErrors` to grow a third bucket.

**Tests:** `crates/formoxus/src/reflect/tests/errors.rs`, 14 of them — the
emptiness contract from both sides, qualification through nested `FieldSet`s,
unchosen vs. chosen `VariantSet`, `push_field_error`→`collect_errors` round
trips (leaf and enum-own-path, chosen and unchosen), list rows by row KEY, an
absent optional enum reporting nothing, two `$Variant` segments deep, and the
spec validator landing in `.form` while `.fields` stays empty (the case that
proves `fields.is_empty()` alone is NOT "it passed"). `FormErrors`/`FieldErrors`
are now re-exported from `reflect` for `auth.rs`'s benefit.
