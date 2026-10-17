# Fields and widgets as traits: the build order

A working checklist for the refactor on the `widget` branch: fields and
widgets become traits, with Django-style concrete types behind each.
**The reasoning is in `.claude/memory/django_parallel_design.md`**
("Decided, 2026-10-16 afternoon", "Shape of the field trait", and the
2026-10-17 notes); this file is the order to do things in and the decisions
still open. Claude keeps it current when asked.

> **Status, 2026-10-17.** Step 1 is DONE except the one error Step 2 fixes
> (`dispatch!` at `build.rs:335` still builds the old `Field::<$ty> { … }`
> literal). Step 2 is next. Line numbers are from `e56b73c` plus Todd's
> uncommitted Step 1; ask for a refresh rather than trusting an old one.

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
- **Not moved, on purpose:** the bound tests in `fields.rs`
  (`int_bounds_are_inclusive`, `float_bounds_are_inclusive`, the NaN and
  infinity ones). They test the field's range CHECK (`check_bounds`, still in
  the commented-out `fields.rs`), not `FieldAttrs`; `field_attrs.rs` only
  uses the bound fns to emit `min`/`max`. They come back in Step 2 and move
  to `NumberField<T>` in Step 4, with `int_bound`/`float_bound`.

## Step 2: `FieldNode<F>`, with no behaviour change

**The idea:** re-plumb before redesigning. `FieldNode<F>` replaces the old
struct, but the node keeps `attrs`, `custom_widget` and `choices` exactly as
they are, and one stand-in field reproduces today's behaviour. The suite stays
green through the whole step, so any failure in Step 4 is a real behaviour
change, not plumbing.

### Decisions first (Todd)

- **F-A. `default_widget` before `Widget` exists.** It has no default and
  `Widget` is an empty placeholder, so the stand-in can only `todo!()`. Either
  give it a `todo!()` default, or comment it out of the trait until Step 5.
  Rendering keeps using `WidgetType` until then either way.
- **F-B. `type Context` for now.** The stand-in uses `type Context = ();`.
  The real question (review note C below) waits for Step 8. Fine?
- **F-C. `check_on_server`'s default** (review notes A and B): a default that
  returns no messages, and `-> impl Future<Output = …> + Send` instead of
  `async fn`. Cheap to settle now, since every field type writes it otherwise.

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
  ScalarField::default() }`.
- [x] **The commented-out tests in `fields.rs`** come back (Claude,
  2026-10-17): the 20 on `FieldAttrs` inline in `field_attrs.rs`, the 32 on
  checks and `validate` inline in `field_node.rs`, against
  `FieldNode<ScalarField<T>>`. The attribute walk is `check_attrs` there.
- [x] **Delete `fields.rs`** (2026-10-17).
- [x] **Acceptance:** `just ci` green with no test changed except type names
  (2026-10-17; clippy, test, docs and msrv pass, and `fmt` needs only the
  pre-commit hook). F-A settled for now: `#[expect(clippy::todo)]` on each
  `default_widget` until Step 5.

## Step 3: decide how `form!`'s settings reach a field

**This is the crux of Steps 4 and 6, and the biggest open question.** Today
`build.rs` builds every leaf with defaults, and afterwards `apply_specs` walks
the tree and MUTATES each node from its `FieldSpec` (`label`, `attrs`,
`custom_widget`, `choices`; `distribute_specs` at `fields.rs:484`). That
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

## Step 4: the four built-in fields

- [ ] **`StringField`**: `min_length`, `max_length`, `pattern`.
- [ ] **`NumberField<T>`**: `min`, `max`. `check_bounds` and its tests move
  here, typed: no more reading a bound as `i128` or `f64`. `int_bound`/
  `float_bound` also feed `FieldAttrs::to_attributes`, so they move only
  once `widget_attrs()` emits `min`/`max` instead.
- [ ] **`BoolField`**: an unticked checkbox means `false`. **Decision B-1
  (Todd, when we get here):** `fn empty_value(&self) -> Option<Self::Value>`
  on the trait, so `FieldNode` turns `Empty` into `false` itself, or
  `BoolField::parse` returns `Valid(false)`. Today's special cases
  (`is_unticked_checkbox`, `raw_value_to_validate`, the `"false"` in
  `write_value_into`, and `required: true` meaning "must be ticked") all
  collapse into whichever is chosen.
- [ ] **`ChoiceField<T>`**, typed: owns `choices`; membership ("not one of the
  available choices") moves into its `check`. Closes the gap in
  `.claude/memory/enum_as_a_value_choice.md` for a unit enum once Step 6 can
  name it.
- [ ] **The constraints leave `FieldNode`.** `attrs`' validated keys move into
  the fields above; `check(&Value)` replaces the attribute walk over the raw
  string; `widget_attrs()` emits them. **Decision A-1 (Todd):** the
  NON-validated author attributes (`placeholder`, `autocomplete`, `class`, a
  quoted key) are presentation, not field config; they go on the widget
  (Step 5), or stay a `FieldAttrs` on the node until then?
- [ ] **`choices` leaves `FieldNode`** and `FieldSpec`.
- [ ] **`dispatch!`** maps each `ScalarType` to its field; `ScalarField` is
  deleted.
- [ ] **Associated consts for the compile-time checks** (decided 2026-10-17:
  emitted `const _`, not spec tables): at least `const KIND: FieldType`.
  `Field` must never be used as `dyn` for this to work; `FieldNode<F>` is
  generic and `Node` is the `dyn` one.
- [ ] Tests (Claude): each field's `parse`/`check` inline; the suite's
  constraint tests should pass unchanged, which is the point.

## Step 5: the `Widget` trait and one `WidgetProps`

Absorbs Step 4d of `ATTRIBUTES_PLAN.md` (the custom-widget trait, and the
built-in widgets' boilerplate) and its decision D4. Built-in and custom
widgets become ONE concept.

### Decisions first (Todd)

- **W-A. Where a widget's compile-time facts live.** `Box<dyn Widget>`
  rules out associated consts on `Widget`, so the facts `form!` checks (which
  attributes it takes, its parts, its default classes) need either a companion
  non-`dyn` trait (`WidgetKind`, implemented next to `Widget`) or a const
  value the macro can name.
- **W-B. What stays in `formoxus-spec`.** Today `BuiltinWidget` there is how
  `form!` knows `widget: textarea`. With widget types, `widget: textarea` can
  name `::formoxus::widgets::Textarea` and check through W-A's consts, which
  would let `BuiltinWidget` shrink to the data spec genuinely needs (parts,
  elements). Keep it whole, shrink it, or retire it?
- **W-C. The widget type list.** Django's names (`TextInput`, `NumberInput`,
  `EmailInput`, …, `Textarea`, `CheckboxInput`, `Select`, `RadioSelect`,
  `HiddenInput`) or one type per `RenderedInputType`. Raised before: one type
  per input `type`.

### Boxes

- [ ] **`trait Widget`**: at least `render(&self, props: WidgetProps) ->
  Element`; `Send + Sync`, `Debug`, and a `clone_box` (a `FormState` crosses
  to the server). Moves out of `field.rs` (its `// TODO: move`).
- [ ] **The built-in widget types**, per W-C, each wrapping today's component.
- [ ] **`Field::default_widget`** returns one; `ChoiceField` builds its chooser
  with its choices inside it (Django style: `Select { choices }`).
- [ ] **`build.rs` resolves the widget once**; `FieldNode` holds `widget:
  Box<dyn Widget>`, never an `Option`.
- [ ] **One `WidgetProps`** (`id, path, label, attrs, extra_attrs, values,
  errors, classes`) replaces `FieldProps`, the old `WidgetProps`, and
  `ScalarWidget`'s props. `ScalarWidget` itself goes: `FieldNode::render`
  calls `self.widget.render(props)`.
- [ ] **The boilerplate** (4d's last box): a plain fn `field_row(…)` rendering
  wrapper, label and errors around the form control a widget builds; the radio
  group keeps its own shape. Snapshot-guarded.
- [ ] **Delete** `WidgetType`, the `Custom` fn pointer, `FieldSpec::custom_widget`.
- [ ] **The variant picker** (`VariantSelect`) becomes a chooser widget that
  still fires `on_edit` (decided 2026-10-16).
- [ ] Tests (Claude): `widget_matrix` and the custom-widget tests move to the
  new trait; goldens for a key a widget does not take.

## Step 6: `field:` in `form!`, and the type → field registry

- [ ] **`field: ProgramSnippetField`** for a path, like `widget:`; built
  through Step 3's mechanism.
- [ ] **`form!` emits `const _` checks** against `<F as Field>::KIND` (and
  the widget's facts, W-A) for every field it names.
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

Raised in review, not yet discussed. Settled ones move into the step above.

- **A.** `check_on_server` has no default → Step 2, F-C.
- **B.** `async fn` in a public trait has no `Send` bound; server handlers
  need one → Step 2, F-C.
- **C.** `type Context` on `Field` vs the form's `Context` → Step 8.
- **D.** The default `check`'s unused `value` warns; `_value`.
