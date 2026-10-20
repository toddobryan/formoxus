# Fields and widgets as traits: the build order

A working checklist for the refactor on the `widget` branch: fields and
widgets become traits, with Django-style concrete types behind each.
**The reasoning is in `.claude/memory/django_parallel_design.md`**
("Decided, 2026-10-16 afternoon", "Shape of the field trait", and the
2026-10-17 notes); this file is the order to do things in and the decisions
still open. Claude keeps it current when asked.

> **Status, 2026-10-17.** Steps 1–3 are DONE. `FieldNode<F>` is in
> `field_node.rs` over the `ScalarField<T>` stand-in, `fields.rs` is deleted,
> and the shape walk applies the spec as it builds (S-b), so `apply_specs` is
> gone. **Step 4: the four fields exist and `dispatch!` builds them; next is
> moving the constraints and `choices` onto them, with `form!` emitting a
> constructor.** Line numbers drift; ask
> for a refresh rather than trusting an old one.

**The goal is fewer concepts** (Todd, 2026-10-16): "We have an awful lot of
similarly named concepts that do similar things." The Django parallel is the
means. Each step below should leave the inventory of names (end of the design
note) shorter, or say why not.

Who does what: Todd writes the library code; Claude reviews, writes and
updates tests, goldens and the e2e forms, and does mechanical fixes when asked.

---

## Where it ends up

| Today | After |
|---|---|
| struct `Field<T>` (was `FormField<T>`): value, errors AND config | trait `Field` (config and behaviour, what a dev implements) + struct `FieldNode<F: Field>` (tree state, one `impl Node`) |
| `T` decides parsing; `FieldAttrs` walk decides checks | `Field::parse`, `Field::check`, `Field::widget_attrs` per field type |
| `choices: Option<Vec<Choice>>` on every field | only on `ChoiceField<T>`, which hands them to its widget |
| `WidgetType::{Builtin(BuiltinWidget), Custom { fn }}`, `custom_widget: Option<WidgetType>` resolved on every call | trait `Widget`, implemented by `TextInput`, `Textarea`, `Select`, … and a dev's own; resolved once in `build.rs` |
| `FieldProps` + `WidgetProps` + `FieldAttrs` + `ScalarWidget`'s props | one `WidgetProps` (Todd's WIP, `widgets/widget_props.rs`) |
| errors as a prop computed from `state.read()` | `ErrorsStore`, a client projection with one writer |
| `form!` emits a `FormSpec<T>` expression | `form!` emits `XForm: FormDef` (`type Model`, `type Context`) |
| server checks only by hand, through `Submission::reject_field` | `Field::check_on_server`, run only by `Submission` |

---

## Step 1: build green after moving `FieldAttrs` out (DONE 2026-10-17)

- [x] `FieldAttrs` and its methods in `field_attrs.rs`; `int_bound`/`float_bound`
  with them.
- [x] The six widgets import `crate::field_attrs::FieldAttrs`.
- [x] One `parse_scalar`, in `field.rs`; the suite imports
  `formoxus::field::parse_scalar`. The commented-out callers in `fields.rs`
  become `self.field.parse(…)` in Step 2.
- The bound tests (`int_bounds_are_inclusive` and the rest) test the range
  CHECK, not `FieldAttrs`, so they went to `field_node.rs` in Step 2 and move
  to `NumberField<T>` in Step 4.

## Step 2: `FieldNode<F>`, with no behaviour change

**The idea:** re-plumb before redesigning. `FieldNode<F>` replaces the old
struct, but the node keeps `attrs`, `custom_widget` and `choices` exactly as
they are, and one stand-in field reproduces today's behaviour. The suite stays
green through the whole step, so any failure in Step 4 is a real behaviour
change, not plumbing.

### Decisions (all settled 2026-10-17)

- **F-A. `default_widget` before `Widget` exists:** each impl `todo!()`s
  under `#[expect(clippy::todo)]` until Step 5.
- **F-B. `type Context`:** `()` on the stand-in; the real question (review
  note C) waits for Step 8.
- **F-C. `check_on_server`:** defaults to no messages, and returns
  `impl Future<Output = Vec<ValidationMessage>> + Send` instead of being an
  `async fn`. An implementor may still write `async fn` in its `impl`.

### Boxes

- [x] **`FieldNode<F: Field>`**: the old struct, renamed. `value:
  FieldValue<F::Value>` and a new `field: F`; `name, label, optional, attrs,
  custom_widget, choices, wrapper, errors` unchanged.
- [x] **`impl<F: Field> Node for FieldNode<F>`**, changing only where `T` was:
  - `distribute_values`: `parse_scalar::<T>(raw)` → `self.field.parse(raw)`.
  - `write_value_into`: the unticked checkbox's `parse_scalar::<T>("false")`
    → `self.field.parse("false")`; `set` takes `F::Value`.
  - `validate`: keep the attribute walk (`check(&raw)`), then on `Valid(v)`
    also `self.field.check(v)`.
  - `field_type()` and `raw_value()` read `F::Value`'s shape.
  - `render`, `edit`, `distribute_specs`, the error methods: header only.
- [x] **Stand-in `ScalarField<T>(PhantomData<T>)`**: `Value = T`, default
  `parse`, no checks. Deleted in Step 4.
- [x] **`dispatch!`** builds `FieldNode::<ScalarField<$ty>> { …, field:
  ScalarField(PhantomData) }`.
- [x] **The commented-out tests in `fields.rs`** come back (Claude,
  2026-10-17): the 20 on `FieldAttrs` inline in `field_attrs.rs`, the 32 on
  checks and `validate` inline in `field_node.rs`, against
  `FieldNode<ScalarField<T>>`. The attribute walk is `check_attrs` there.
- [x] **Delete `fields.rs`** (2026-10-17).
- [x] **Acceptance:** `just ci` green with no test changed except type names
  (2026-10-17; clippy, test, docs and msrv pass, and `fmt` needs only the
  pre-commit hook). F-A settled for now: `#[expect(clippy::todo)]` on each
  `default_widget` until Step 5.

## Step 3: decide how `form!`'s settings reach a field (DONE 2026-10-17)

**This is the crux of Steps 4 and 6, and the biggest open question.** Today
`build.rs` builds every leaf with defaults, and afterwards `apply_specs` walks
the tree and MUTATES each node from its `FieldSpec` (`label`, `attrs`,
`custom_widget`, `choices`; `distribute_specs` at `field_node.rs:473`). That
works because every leaf is one type. Once a leaf is `FieldNode<F>`:

1. `min: 3` belongs on `NumberField<i32>`, but generic `FieldNode<F>` cannot
   reach into an unknown `F` to set it.
2. `field: ProgramSnippetField` changes the node's TYPE, which no mutation can
   do; the node has to be built differently in the first place.

### Decision S (Todd)

- **S-a. The spec carries a built field, erased.** `form!` builds the typed
  value (`NumberField::<i32>::default().min(3)`, its type found by inference
  through the projection closure `shape_of(|m: &Model| &m.age)`) and stores it
  as `Box<dyn Any + Send + Sync>` (plus a clone fn). `distribute_specs`
  downcasts to `F` and replaces `self.field`. Handles (1); for (2) the spec
  must also carry a node constructor, since the downcast fails when `F`
  differs.
- **S-b. `build.rs` consults the spec DURING the walk.** The spec carries a
  constructor per mentioned path (`fn(name, peek, optional, wrapper) -> Box<dyn
  Node>`, emitted by `form!` with the field's settings baked in); the walk uses
  it instead of `dispatch!` when one is present. Handles (1) and (2) with one
  mechanism, and `apply_specs`/`distribute_specs` largely go away, which is
  the "fixed at compile time, less runtime work" Todd asked for. Costs: the
  walk needs the spec and the path at every leaf, including `[]` row
  selectors and variants, and every structural edit already re-walks, so that
  path has to carry the spec too.
- Claude's lean: **S-b**, because it removes a mechanism instead of adding one,
  and it is the same shape as `field:`. Worth a spike on one nested `Vec` row
  before committing.

### DONE 2026-10-17: S-b ADOPTED AND INTEGRATED

Todd adopted S-b, including finding 1 below. The spike was merged into
`widget` (PR #12) and then finished: the walk applies EVERY spec setting
as it builds (`label`, `attrs`, `custom_widget`, `choices`, and the
constructor), `LeafArgs::node` is the one place a leaf is assembled (both
`dispatch!` and a constructor go through it), and `apply_specs`,
`Node::distribute_specs` with its five impls, `ListSet`'s `[]` key rewriting
and `FormState::edit`'s re-apply are deleted. The `specs.rs` suite passed
unchanged; the spike's tests live on as `tests/suite/spec_constructors.rs`.

### Spike result, 2026-10-17: S-b HOLDS (draft PR #12, `spike/s-b`)

Branch `spike/s-b`, worktree `../formoxus-spike-sb`, uncommitted. `just ci`
green there, plus 6 new tests in `tests/suite/spike_leaf_ctor.rs`: a
`venues[].capacity` constructor reaches rows built by `form_for`, rows added
by `AddRow`, and `shapes[].$Circle.radius` both chosen later and built from a
value. What it took:

1. **`Walk { mode, specs }`**, one `Copy` value threaded through `build.rs` in
   place of `mode`, so no seventh parameter.
2. **`spec_key(path)`** folds each `.#n` into `[]` (`venues.#3.city` →
   `venues[].city`): one lookup, replacing `ListSet`'s key substitution.
3. **`FieldSpec::node: Option<NodeCtor>`**, `NodeCtor` an
   `Arc<dyn Fn(LeafArgs) -> Box<dyn Node> + Send + Sync>`; `scalar_node` uses
   it before `dispatch!`. `LeafArgs::node(field)` builds the default
   `FieldNode` around a field, so `populate` stays `pub(crate)`.
4. **A real bug the walk never hit:** `enum_node` built a variant's fields
   under `shape.radius`, not `shape.$Circle.radius`. Fixed; the
   `form_for`-variant test fails without it.
5. **`Node::edit` takes `&SpecsByPath`** (five impls) so `AddRow` and
   `ChooseVariant` build with the spec; `FormState::edit` passes its own.

Findings for the real implementation:

- **The payoff is bigger than leaves.** The same lookup at construction can
  apply `label`, `attrs`, `custom_widget` and `choices` to EVERY node, which
  deletes `apply_specs`, all five `distribute_specs` impls, `ListSet`'s `[]`
  substitution, and `FormState::edit`'s re-apply. The spike kept them only
  to stay small.
- **Nothing checks a constructor's type against the shape.** A `u32` field
  on a `String` leaf fails only at `write_value_into`. Step 6's `const _`
  checks must cover `field:`, including `[]` and `$Variant` paths.
- **`FieldSpec` equality becomes identity** for a spec with a constructor
  (`Arc::ptr_eq`): two separately built specs are never equal.

## Step 4: the four built-in fields

- [x] **`StringField`** (2026-10-17, `fields/string_field.rs`, settings `pub`): `min_length`, `max_length`, `pattern`.
- [x] **`NumberField<T: Number>`** (2026-10-17, `fields/number_fields.rs`; `check` done, typed): `min: Option<T>`, `max: Option<T>`.
  `check_bounds` and its tests move here, typed: `T` compares to `T`, no more
  reading a bound as `i128` or `f64`. `int_bound`/`float_bound` also feed
  `FieldAttrs::to_attributes`, so they move only once `widget_attrs()` emits
  `min`/`max` instead.
  - **`Number`** (decided 2026-10-17): a SEALED trait for exactly i8–i64,
    u8–u64, f32, f64 (no `usize`/`isize`, as in `build.rs`), with
    `#[diagnostic::on_unimplemented]`, `const KIND: FieldType`, and
    `fn is_nan(&self) -> bool` (default `false`, overridden for floats). The
    bound goes on the struct. Keep its type list in step with `dispatch!`.
  - **A whole float bound on an integer field is REFUSED** (Todd,
    2026-10-17): `min: 2.0` on an `i32` is now a type error, not quietly
    read as `2`. Flips `a_whole_float_bound_on_an_integer_field_is_used_as_an_integer`
    and `a_fractional_bound_on_an_integer_field_is_a_caller_bug`
    (`field_node.rs`). Typed bounds also retire `field_kind::bound_is_whole`,
    `bound_is_exact` and probably `bound_in_range` with their goldens
    (`form_fractional_bound_on_an_integer`, `form_integer_bound_too_big_for_f64`,
    `form_bound_overflows_f32`); each gets checked for whether rustc's own
    error now says it well enough, or whether a golden should pin the new one.
- [x] **`BoolField`** (2026-10-17, `required_true` only; B-1 answered: `FieldNode` hands an unticked checkbox to `check` as `false`): an unticked checkbox means `false`. **Decision B-1
  (Todd, when we get here):** `fn empty_value(&self) -> Option<Self::Value>`
  on the trait, so `FieldNode` turns `Empty` into `false` itself, or
  `BoolField::parse` returns `Valid(false)`. Today's special cases
  (`is_unticked_checkbox`, `raw_value_to_validate`, the `"false"` in
  `write_value_into`, and `required: true` meaning "must be ticked") all
  collapse into whichever is chosen.
- [x] **`ChoiceField<T>`** (built 2026-10-17, WIRED 2026-10-18: `choices:` is a FIELD-level key, `choice_field_for` builds it, `ChoiceValue` lets `(true, "Yes")` and `&str` tables type; widget args deleted; choices exclude validated keys), typed: owns `choices`; membership ("not one of the
  available choices") moves into its `check`. Closes the gap in
  `.claude/memory/enum_as_a_value_choice.md` for a unit enum once Step 6 can
  name it.
- [x] **The constraints leave `FieldNode`** (done by the constraint move; A-1 answered: render-only attributes stay a `FieldAttrs` on the node). `attrs`' validated keys move into
  the fields above; `check(&Value)` replaces the attribute walk over the raw
  string; `widget_attrs()` emits them. **Decision A-1 (Todd):** the
  NON-validated author attributes (`placeholder`, `autocomplete`, `class`, a
  quoted key) are presentation, not field config; they go on the widget
  (Step 5), or stay a `FieldAttrs` on the node until then?
- [x] **`choices` leaves `FieldNode`** and `FieldSpec` (2026-10-18; the widget reads `Field::choices()` until Step 5, and a choice field defaults to a select).
- [x] **`dispatch!`** maps each `ScalarType` to its field (DONE 2026-10-17; every field built with its settings off, so `check_attrs` still does the checking): every numeric one
  to `NumberField<$ty>` (Todd, 2026-10-17), `String` to `StringField`, `Bool`
  to `BoolField`. That makes `dispatch!`'s numeric arms and `Number`'s impls
  one visible list. `ScalarField` is deleted.
- [x] **THE CONSTRAINT MOVE, designed 2026-10-17 (Todd + Claude); DONE the same day, `just ci` and `just e2e` green.** The model
  field's TYPE picks the field, through a trait rustc resolves:
  ```rust
  // formoxus::field_kind (doc-hidden)
  pub trait DefaultField { type Field: Field + Default + Send + Sync; }
  // String → StringField, bool → BoolField, each Number → NumberField<T>
  impl<T: DefaultField> DefaultField for Option<T> { type Field = T::Field; }
  pub fn field_for<M, P: DefaultField>(
      _: fn(&M) -> &P, set: impl FnOnce(&mut P::Field)) -> NodeCtor
  ```
  and `form!` emits only the settings, never a type name:
  `.with_node("age", field_for(|__m: &Person| &__m.age, |f| { f.min = Some(13); f.max = Some(120); }))`.
  `[]` rows already project to the element (`field_kind::row`). A setting the
  field lacks (`f.min_length` on a `NumberField`) does not compile; the
  `applies` checks stay for their better message. `DefaultField` is the
  compile-time half of Step 6's type → field registry. Decided with it:
  - **A.** A bound has the field's declared type: `min: MIN_AGE` must be a
    `u8` on a `u8` field. Retires `bound_in_range`/`bound_is_whole`/
    `bound_is_exact` and their goldens; rustc's errors replace them.
  - **B.** Attributes stop validating. `with_attrs` PANICS on a validated key
    and points to `with_node`; suite tests using it convert.
  - **C.** `choices:` moves in a SEPARATE, later change (typed choices break
    today's string tables).
  - **D.** No `DefaultField` for newtypes. A validated setting on a newtype
    field does not compile; custom field types will declare their own
    settings when that comes up.
  - **E.** (Todd) Retire the runtime newtype unwrapping too (`wrapper` in
    `build.rs`/`FieldNode`, `newtype_inner`): "now that we can declare both
    field and widget, we shouldn't guess about exotic types. We should make
    devs specify them." Needs `field:` (Step 6) first, so a `Markdown(String)`
    field has a way to be declared; until then it stays.
  - **Two errors for a wrong-kind setting, accepted for now** (Todd,
    2026-10-18): `min` on a `String` gets rustc's "no field `min` on
    `StringField`" AND the `applies` message; a struct or enum gets "has no
    field that takes settings" (its main span is the whole `form!`) AND the
    `applies` message. Revisit with `field:` in Step 6.
  - **Not checked: a float bound too precise for its type.** rustc rounds
    `9_007_199_254_740_993.0` silently. A literal could be checked at
    expansion; a named const cannot (its digits are gone). Left to clippy's
    `excessive_precision`.
  Runtime side: `Number` gains a `T` → `Bound` conversion so `widget_attrs()`
  emits `min`/`max`; `StringField` emits `minlength`/`maxlength`/`pattern`;
  `BoolField` emits `required` for `required_true`; `render` merges
  `widget_attrs()`; `check_attrs`, `check_bounds` and `FieldNode`'s
  `range_message` are deleted.
- [x] **`form!` builds the field when a setting belongs to it** (done by the constraint move; sequencing,
  found 2026-10-17). Once `min` lives on `NumberField` rather than in
  `FieldAttrs`, `with_attrs` can no longer carry it: `form!` has to emit a
  constructor (`with_node`, Step 3) that builds `NumberField::<T> { min, max }`.
  `form!` sees no types, so `T` comes from the projection closure it already
  emits for its checks (`field_kind::shape_of(|__m: &Model| &place)`, `[]`
  rows included), via a typed helper such as `number_field_for(|__m| &place,
  min, max) -> NodeCtor`. Open: an `Option<u32>` field projects to
  `&Option<u32>`, so the helper needs to peel it (a small trait implemented
  for `T` and `Option<T>`). Until this lands, a field built by `dispatch!` has
  `min: None` and the bounds stay in `check_attrs`, so moving them is safe
  only together with this box.
- [ ] **Associated consts for the compile-time checks** (decided 2026-10-17:
  emitted `const _`, not spec tables): at least `const KIND: FieldType`.
  `Field` must never be used as `dyn` for this to work; `FieldNode<F>` is
  generic and `Node` is the `dyn` one. **Deferred to Step 6 (2026-10-18):**
  `DefaultField` + `field_for` already make rustc check a setting against the
  model's type, which was most of the point; a `KIND` const earns its keep
  when `field:` names a dev's own field.
- [x] Tests (Claude): each field's `parse`/`check` inline; the suite's
  constraint tests should pass unchanged, which is the point. (Done 2026-10-18;
  they did.)

## Step 5: the `Widget` trait and one `WidgetProps`

Absorbs Step 4d of `ATTRIBUTES_PLAN.md` (the custom-widget trait, and the
built-in widgets' boilerplate) and its decision D4. Built-in and custom
widgets become ONE concept.

### Decisions first (Todd)

- **W-A. DECIDED 2026-10-18 (Todd): a widget is a struct of PROPS, like a
  Dioxus component.** Worked out in conversation; the reasoning:
  - **Props are `pub` fields on the widget's type**, the same move as a
    field's settings, so rustc checks names and types and `form!` keeps no
    list. They go in the widget's braces, `widget: MarkdownWidget { preview:
    true }`, emitted as a struct expression with `..Default::default()`, each
    key keeping its span. Field-level keys stay field settings and HTML
    attributes, so a prop can never collide with an attribute's name.
  - **`EXTENDS: AttrSet`**, Dioxus's `#[props(extends = textarea)]`: the HTML
    attributes the widget passes through and spreads where it likes. A const
    bitset over `Attr`; `AttrSet::valid_on(element)` reads the table, plus
    `.with`, `.without`, `ANY`; owned attributes are never in one. It is the
    only compile-time fact left, since props need none.
  - **Validated keys never need to be in `EXTENDS`.** `min_length` and the
    rest are the FIELD's settings: always server-checked, reaching the widget
    through `Field::widget_attrs()`, and emitted only where it can carry them.
    So a custom widget over a `StringField` lists nothing to compile.
  - **Parts shrink to CLASS SLOTS.** Per-part attribute addressing (D0's
    points 3–5 in `ATTRIBUTES_PLAN.md`) was never built and is dropped; the
    widget decides where every attribute goes. `Classes` stays keyed by
    (widget, slot), and a prop overrides one slot for one field: `RadioGroup`
    gets `group_class`/`input_class` props, settling that hold.
  - **One dev-facing trait, the `dyn` one hidden.** Consts make a trait
    non-`dyn`, so the dev implements `Widget` (Sized, with the consts and
    `render`) and formoxus has a `#[doc(hidden)] DynWidget` with a blanket
    impl, held as a newtype `AnyWidget(Box<dyn DynWidget>)` with hand-written
    `Clone`/`PartialEq` (deriving `PartialEq` over `Box<dyn …>` hits a rustc
    bug). Spiked in a standalone file: compiles, and a `const _` assert reads
    "evaluation panicked: `rows` goes on a `textarea`, and `MarkdownWidget`
    has none".
  - **Shape 1: `#[derive(Widget)]`** writes the facts (`#[widget(extends =
    textarea)]`, per-prop `#[widget(default = …)]`, so `Default`); the dev
    writes `render` by hand. Chosen over an attribute on a fn (`#[component]`
    style) because a type keeps room for associated functions. The attribute
    is `#[widget(…)]`, not `#[props(…)]`, which is Dioxus's. A derive is fine
    here despite CLAUDE.md's "never derives": a widget is the dev's own type,
    in the dev's own crate. The plain trait comes first; the derive is a
    later box.
  - **Multi-control widgets, both kinds, are wanted** (Todd): a SET widget
    laying out a struct's leaves as one control (`ContactInfoWidget`; names,
    parsing and errors already work through the `FieldSet`), and a SPLIT
    widget putting one scalar over several controls (`DateWidget`; needs
    sub-paths and Django's `decompress`/`value_from_datadict` pair).
    **Step 5 builds SINGLE-CONTROL widgets only** (Todd, 2026-10-18), and must
    not rule the others out (its acceptance box); they are Step 10.
  - **Prop values are plain expressions, emitted with `.into()`** (Todd,
    2026-10-19). So `label_class: "big bold"`, never the `[big, bold]` class
    grammar: `form!` cannot know which keys a widget wants converted, since
    the dev names them. `.into()` on everything EXCEPT a bare number
    literal (Todd, 2026-10-19): `rows: 5.into()` into a `u32` does not
    compile, since the literal falls back to `i32` and there is no `i32 →
    u32` `From`, the failure `choices` hit. So `form!` emits a number literal
    as written and wraps every other value in `.into()`.
- **W-B. DECIDED 2026-10-19 (Todd): `BuiltinWidget` is retired.** Its three
  jobs all move: naming a widget goes to a type path, attribute checking to
  `Widget::EXTENDS`, and class-slot keys to something every widget has (a
  `const NAME` on the trait), since a custom widget has slots too and an enum
  of the built-ins cannot key them. `parts()` goes; slots are names a widget
  declares. `formoxus-spec` keeps `Element`, `FieldControl`, the attribute
  table and the new `AttrSet`. **Lowercase names stay as the built-ins'
  spelling** (`widget: textarea` → `::formoxus::widgets::Textarea`), and a
  custom widget is written as a path (`widget: crate::MarkdownWidget { … }`),
  as discussed earlier. The macro keeps a small name → path table for the
  lowercase names.
- **W-C. DECIDED 2026-10-19 (Todd): Django's names, extended.** The input
  type must be in the Rust type, since `EXTENDS` is a const and attributes
  differ by type (`step` on `number`, not `text`), so it is one type per
  input type either way; the question was naming. `TextInput`,
  `NumberInput`, `EmailInput`, `PasswordInput`, `HiddenInput`,
  `CheckboxInput`, `Textarea`, `Select`, and HTML's name + `Input` where
  Django has none (`TelInput`, `SearchInput`, `UrlInput`, `ColorInput`,
  `DateInput`, `TimeInput`, `DatetimeLocalInput`, `MonthInput`,
  `WeekInput`). One `macro_rules!` generates the text-like ones over a shared
  render. Rejected: a generic `Input<Email>`, noisier in `form!` and with no
  home for a one-type prop (Django's `PasswordInput { render_value }`).
  The radio widget stays `RadioGroup`, not Django's `RadioSelect` (Todd,
  2026-10-19).

### Boxes

- [x] **`AttrSet`** in `formoxus-spec`: a const bitset over `Attr`,
  `valid_on(element)`, `with`, `without`, `ANY`, never an owned attribute.
- [x] **`trait Widget`, `DynWidget` and `AnyWidget`, one change** (Todd,
  2026-10-19: together, because `Field::default_widget` returns `Box<dyn
  Widget>` today and a trait with consts cannot be `dyn`). In a new
  `formoxus/src/widgets/widget.rs`; `widgets.rs` gains `pub mod widget;` and
  `pub use widget::{AnyWidget, Widget};`. The stub and its `// TODO: move`
  leave `field.rs`. Shape proven in the W-A spike (2026-10-18), which had
  `PARTS` where this has `NAME`/`EXTENDS`/`default_classes`:

  ```rust
  use std::{any::Any, fmt::Debug};
  use dioxus::prelude::Element;                    // NOT formoxus_spec::Element
  use formoxus_spec::AttrSet;
  use crate::{classes::Slot, widgets::widget_props::WidgetProps};

  pub trait Widget: Clone + Debug + PartialEq + Default + Send + Sync + 'static {
      /// Keys this widget's class slots in `Classes` (W-B). Unique per widget.
      const NAME: &'static str;
      /// The HTML attributes this widget passes through (W-A).
      const EXTENDS: AttrSet;
      fn render(&self, props: WidgetProps) -> Element;
      /// Each class slot this widget has, with its default (`fx`) classes.
      fn default_classes() -> Vec<(&'static str, Slot)>;
  }

  pub(crate) trait DynWidget: Debug + Send + Sync {
      fn render(&self, props: WidgetProps) -> Element;
      fn name(&self) -> &'static str;
      fn default_classes(&self) -> Vec<(&'static str, Slot)>;
      fn clone_box(&self) -> Box<dyn DynWidget>;
      fn as_any(&self) -> &dyn Any;
      fn eq_dyn(&self, other: &dyn DynWidget) -> bool;
  }

  impl<W: Widget> DynWidget for W {
      fn render(&self, props: WidgetProps) -> Element { Widget::render(self, props) }
      fn name(&self) -> &'static str { W::NAME }
      fn default_classes(&self) -> Vec<(&'static str, Slot)> { W::default_classes() }
      fn clone_box(&self) -> Box<dyn DynWidget> { Box::new(self.clone()) }
      fn as_any(&self) -> &dyn Any { self }
      fn eq_dyn(&self, other: &dyn DynWidget) -> bool {
          other.as_any().downcast_ref::<W>() == Some(self)
      }
  }

  #[derive(Debug)]
  pub struct AnyWidget(Box<dyn DynWidget>);         // field private
  impl AnyWidget {
      pub fn new(widget: impl Widget) -> Self { Self(Box::new(widget)) }
      pub fn render(&self, props: WidgetProps) -> Element { self.0.render(props) }
      pub fn name(&self) -> &'static str { self.0.name() }
      pub fn default_classes(&self) -> Vec<(&'static str, Slot)> { self.0.default_classes() }
  }
  impl<W: Widget> From<W> for AnyWidget { fn from(w: W) -> Self { Self::new(w) } }
  impl Clone for AnyWidget { fn clone(&self) -> Self { Self(self.0.clone_box()) } }
  impl PartialEq for AnyWidget { fn eq(&self, o: &Self) -> bool { self.0.eq_dyn(&*o.0) } }
  ```

  Why each piece:
  1. **Every bound on `Widget` has a job.** `Clone`/`PartialEq` let
     `FieldNode`, and so `FormSpec`, keep deriving theirs; `Debug` likewise;
     `Default` is what `form!`'s `..Default::default()` needs (later box);
     `Send + Sync` keep `AnyWidget` storable wherever a spec goes; `'static`
     is what `as_any` needs.
  2. **`EXTENDS` is not on `DynWidget`**: only `form!`'s `const _` asserts
     read it, through the concrete type, never at run time.
  3. **`default_classes` is a fn, not a `const SLOTS`** (Todd): `Slot`'s
     `with_invalid` takes `impl Into<Cow>`, so it cannot be const, and every
     built-in's wrapper slot has an invalid class. No `&self` on the trait's
     version, since the slots are the type's, not one value's.
  4. **`render` takes `props` by value** (Todd): `FieldNode` builds a fresh
     one each render, and the widget moves strings out of it into `rsx!`.
  5. **`render` is a plain method, NOT a component**, so it must not call
     hooks itself: `FieldNode::render` calls it inside its own component, and
     a hook there would join that component's hook list. A widget that needs
     state renders a `#[component]` of its own from `render`, which is what
     the built-ins will do with today's `Input`, `Textarea` and the rest.
  6. **`DynWidget` is `pub(crate)`, not `#[doc(hidden)] pub`.** `AnyWidget`'s
     field is private and its methods forward, so no dev ever names it, and
     private-in-public never comes up. (W-A said `doc(hidden)`; this is
     tighter.)
  7. **`Clone` and `PartialEq` are written by hand.** Deriving `PartialEq`
     over a `Box<dyn …>` trips a rustc bug (it tries to move out of the box),
     and `Clone` needs `clone_box`. `eq_dyn` downcasts, so two widgets of
     different types are unequal and two of one type compare their props.
     `Debug` can be derived, since `dyn DynWidget: Debug`.
  8. **`From<W>`** so a holder can take `impl Into<AnyWidget>`, and
     `default_widget` can end in `.into()`.

  `Field::default_widget` returns `AnyWidget` and keeps its `todo!()`; nothing
  constructs one until the built-in types exist. One test file names the old
  stub: `tests/suite/spec_constructors.rs`'s `CapField` overrides
  `default_widget`, so it imports `formoxus::widgets::AnyWidget` and returns
  that. The sketch above, dropped in with those edits, passed workspace
  clippy (`-D warnings`) and the suite, 2026-10-19. Name clash to expect at
  that box (not this one): W-C's `Textarea`, `Select` and `RadioGroup`
  widget types share names with today's components in `formoxus::widgets`.
  Tests (Claude), inline in `widget.rs` with two toy widgets: `eq` across
  equal props, different props and different types; a clone is equal; `name`
  and `default_classes` reach through `AnyWidget`.
- [ ] **The built-in widget types**, per W-C, each wrapping today's component.
  FIRST of the remaining boxes (Todd, 2026-10-20). Todd writes the first few
  to learn the pattern, then hands the rest to Claude. Decided 2026-10-20:
  1. **Today's components take a `View` suffix** (`InputView`,
     `TextareaView`, `SelectView`, `RadioGroupView`), `pub(crate)`, so the
     bare names belong to the widget types. A stopgap: Todd hopes to retire
     the components outright.
  2. **No `field_type` in `WidgetProps`; `FieldNode` converts first.** The
     only use of `field_type` in every component is `merge_with_attrs`,
     which turns a `Bound` into the field's kind (`int_bound`/`float_bound`).
     Resolving the bounds once, before a widget sees them, means no widget,
     built-in or custom, ever deals with a field kind.
  3. **`Classes` keeps its `BuiltinWidget` key for now.** Each type's
     `default_classes()` returns the slots `Classes::fx()` already builds;
     re-keying onto `NAME` waits for the delete box.
- [ ] **`form!`: props in the widget's braces**, a struct expression with
  `..Default::default()` (today's "a widget takes no arguments" error goes,
  with NO special case for `choices` inside the braces: unreleased, so
  rustc's "no field `choices`" is enough, Todd 2026-10-20); a `const _`
  assert per field-level HTML attribute against `EXTENDS`, skipping
  validated keys. `custom(Path)` goes too (W-B): a lowercase ident is a
  built-in name, anything else a path. AFTER the built-in types (Todd,
  2026-10-20), since the struct expression and the asserts both name one.
  Goldens (Claude) once this lands: a misspelled prop, a prop of the wrong
  type, an attribute not in `EXTENDS`.
- [ ] **`Field::default_widget`** returns one; `ChoiceField` builds its chooser
  with its choices inside it (Django style: `Select { choices }`).
- [ ] **`build.rs` resolves the widget once**; `FieldNode` holds `widget:
  AnyWidget`, never an `Option`.
- [ ] **One `WidgetProps`** (`id, path, label, attrs, extra_attrs, values,
  errors, classes`) replaces `FieldProps`, the old `WidgetProps`, and
  `ScalarWidget`'s props. `ScalarWidget` itself goes: `FieldNode::render`
  calls `self.widget.render(props)`.
- [ ] **The boilerplate** (4d's last box): a plain fn `field_row(…)` rendering
  wrapper, label and errors around the form control a widget builds; the radio
  group keeps its own shape. Snapshot-guarded.
- [ ] **Flatten `FieldControl` into `Element`** (Todd, 2026-10-20), in the
  same change as the delete below: `Element` becomes `Div | Fieldset | Label
  | Legend | Input(InputType) | Textarea | Select`. Its jobs are gone: typing
  `BuiltinWidget::form_control()`/`parts()` leaves with `BuiltinWidget`, and
  `true_and_on(…)` (with `is_also_validated_on` and `is_allowed`, called
  only by `formoxus-spec`'s own tests) is replaced by W-A's rule that a
  validated key is a field setting, server-checked and emitted only where it
  fits. Drop the three `true_and_on` rows' extra arms. CLAUDE.md's "control"
  exception shrinks to `fx-control`.
- [ ] **Delete** `WidgetType`, the `Custom` fn pointer, `FieldSpec::custom_widget`,
  and (W-B) `BuiltinWidget`, `RenderedInputType` if nothing else needs it,
  `SlotKey::Part`'s enum key and the macro's "unknown widget" table beyond
  the lowercase built-in names.
- [ ] **The variant picker** (`VariantSelect`) becomes a chooser widget that
  still fires `on_edit` (decided 2026-10-16).
- [ ] **`#[derive(Widget)]`** with `#[widget(extends = …)]` and per-prop
  `#[widget(default = …)]`, once the built-ins show what it should write.
  `CLAUDE.md`'s "never derives" gains its exception.
- [ ] Tests (Claude): `widget_matrix` and the custom-widget tests move to the
  new trait; goldens for a key a widget does not take, a misspelled prop, and
  a prop of the wrong type.
- [ ] **Acceptance: nothing rules out Step 10.** `render` gets the whole
  `ValuesStore` and its path, not one value string; `AnyWidget` does not
  depend on `FieldNode`, so a `FieldSet` can hold one later; and the trait
  leaves room for split hooks (`sub_names`, `combine`) as default methods.

## Step 6: `field:` in `form!`, and the type → field registry

- [ ] **`field: ProgramSnippetField`** for a path, like `widget:`; built
  through Step 3's mechanism.
- [ ] **`form!` emits `const _` checks** against `<F as Field>::KIND` (and
  the widget's facts, W-A) for every field it names.
- [ ] **A named field wins over any shape.** Today a constructor is consulted
  only in `scalar_node` (`build.rs`, "A spec that names this leaf with a
  constructor builds it"), and `node_for_shape` sends a struct to
  `struct_node` and a list to `list_node` before that. So `field: X` on a
  struct-valued path would build a fieldset and never `X`. The check moves to
  the top of `node_for_shape`. (Found 2026-10-18.)
- [ ] **`Field::format(&self, value: &Self::Value) -> String`**, the reverse
  of `parse` (Django's `prepare_value`), defaulting to facet's display as
  `FieldNode::raw_value` does today. A struct value has no display, so
  without it an existing value cannot refill its input; `populate`'s
  `""`-is-absence check goes through it too. (Found 2026-10-18.)
- [ ] **A registry from model type to field type**, for fields the runtime
  walk meets with only a `Shape` (unnamed, nested, in a row or variant).
  Subsumes `.claude/memory/widget_registry_idea.md`: each field declares its
  default widget, so one registry, not two.
- [ ] Tests (Claude): a custom field from outside formoxus in the suite (the
  `ProgramSnippetField` shape: a textarea whose text must compile), and in
  the examples' gallery.

## Step 7: `ErrorsStore`

Agreed in principle since 2026-09-21 (`.claude/memory/error_model_design.md`,
"ErrorsStore"); in scope since 2026-10-16. A client projection with ONE
writer (`Form::validate`, `push_field_error`, `apply_errors`); `FormState`
stays the authority. `WidgetProps.errors` becomes a read from it, so one
field's errors changing no longer re-renders the whole form. **Decision E-1
(Todd):** before Step 5, so `WidgetProps` is designed around the store once,
or after, so Step 5 lands smaller?

## Step 8: `FormDef`, `Context`, and server-only checks

The "hide `FormSpec` and `FormState`" idea (design note), answered A–F on
2026-10-16: `form! { Address { … } }` emits `AddressForm`, `form! { AddressEdit
for Address { … } }` names it, item and expression both, `Form<Address>` kept.

- [ ] **`trait FormDef { type Model; type Context; fn spec(ctx) -> … }`**;
  `use_form(AddressForm, &ctx)`, `Submission::accept(AddressForm, &ctx, &values)`.
  About 215 `form!` uses and every `empty_form`/`form_for` move (Todd).
- [ ] **`Context` in `form!`** (`context: T`, else `()`), read by closures
  (`choices: |ctx| …`). The server builds its own and never accepts one from
  the client. Likely renamed (Todd: it collides in feel with `NodeContext`).
- [ ] **Review note C: a field's `Context` vs its form's.** A field is written
  once and used in many forms, and one form mixes fields. Options: fields
  generic over the context (`impl<C: HasDb> Field<C>`); an axum `FromRef`-style
  projection from the form's context; or `Context` reaches only server checks,
  through `FieldNode`, as `&dyn Any`.
- [ ] **Runtime attribute values (Todd, 2026-10-19; a possibility, not
  decided).** Tier 2, render-time, in the choices note's numbering. `step:
  compute_step()` stopped compiling once `form!` began checking `step` at
  compile time (above zero, whole on an int field), as `min: f()` never
  could; and once `form!` has built the spec, nothing can reach in to change
  it. Two shapes:
  (a) **a closure over the `Context`**, as `choices: |ctx| …` will be:
  `step: |ctx| ctx.step`. The const checks skip a closure value, and the
  same checks run when it is called, or not at all for a pass-through
  attribute. It falls out of the `Context` box above.
  (b) **a hook on the form's render** (Todd's idea) letting a dev adjust a
  field's attributes as it is emitted. More general, since it needs no
  `Context`, but it is a second way to set an attribute, and it bypasses the
  table's checks unless it goes back through them.
- [ ] **`check_on_server`** runs in `Submission::accept`, never on the
  client; errors come back like `reject_field`'s.
- [ ] Gallery example: "username not taken" against a fixed server-side list
  (`["admin", "todd"]`), no database. E2E pair (Claude).
- [ ] Suite (Claude): the ~220 runtime-free engine tests import
  `formoxus::__private`, staying in the suite.

## Step 9: cleanup and the inventory pass

- [ ] **Drop `raw` from `FieldValue::Invalid`, if we can** (queued 2026-10-17;
  the design note's point 3 has the one path it protects and the test that
  pins it).
- [ ] **Walk the name inventory** (end of the design note) and strike what
  this refactor removed; list what is left with a reason for each.
- [ ] **The macro side's own `FieldSpec`** (and `FormSpecInput`, `FieldBody`,
  …) collides in name with the runtime one; rename whichever survives.
- [ ] **Rewrite `ATTRIBUTES_PLAN.md`'s Step 4** for what is left (below).
- [ ] Docs: README, the prelude, `CLAUDE.md`'s vocabulary sections.

## Step 10: multi-control widgets

Added 2026-10-18 (Todd: both kinds are wanted). Numbered last only so the
step numbers other notes cite stay put; it can start any time after Step 6.
Each kind is built for ONE real widget first, not as a framework (agreed
2026-10-15).

- [ ] **Set widgets: `ContactInfoWidget`.** A widget on a `FieldSet` that lays
  out its struct's leaves as one control. Names, parsing, validation and
  per-leaf errors already work, so this is layout only: the set holds an
  optional `AnyWidget` and hands it its children's props.
- [ ] **Split widgets: `DateWidget`.** One scalar over several controls
  (day/month/year). Needs sub-paths (`when.day`), Django's
  `decompress`/`value_from_datadict` pair as `Widget` default methods, and
  changes to `ValuesStore`, `distribute_values` and the server's matching of
  a submitted name to its path.
- [ ] Tests (Claude): each in the suite, an e2e form for each, and the
  gallery.

---

## What this replaces in `ATTRIBUTES_PLAN.md`

That plan's Step 4 (emit only valid attributes, the custom-widget trait) was
written against `WidgetType` and one struct per field. Todd's answer F
(2026-10-16): this refactor comes BEFORE it and may make parts unnecessary.
My reading, to confirm when Step 9 rewrites it:

1. **4d (the `Widget` trait, the boilerplate) and D4: absorbed** by Step 5.
2. **4a (a `FieldControl` per field at compile time): reshaped.** Its
   `default_control(shape)` mirrors `default_widget`; with Step 4's consts and
   Step 5's W-A, the form control comes from the field's and widget's own
   consts instead.
3. **4b, 4c, 4e, D1–D3: still stand**, rewritten against widget types.
   4b's assertions become Step 6's `const _` checks.
4. **"Then, in order" 2 (reshape `FieldProps`): absorbed** by Step 5's one
   `WidgetProps`. **3 (`form!` is the only door, `__private`): absorbed** by
   Step 8. **4 (split `fields.rs`): absorbed** by Steps 2 and 4.

## Review notes on the `Field` trait draft (2026-10-16)

- **A.** `check_on_server` had no default. SETTLED (F-C).
- **B.** `async fn` in a public trait has no `Send` bound. SETTLED (F-C).
- **C.** `type Context` on `Field` vs the form's `Context`. OPEN → Step 8.
- **D.** The default `check`'s unused `value` warned. SETTLED: `_value`.
