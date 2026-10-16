---
name: django-parallel-design
description: "2026-10-15/16 SPIT-BALLING, nothing decided: follow Django's Form/Field/BoundField more closely. How each Django piece maps onto formoxus, where the parallel breaks, and the FormDef/XForm/Context idea for hiding FormSpec and FormState"
metadata:
  type: project
---

Todd's question (2026-10-15, home): would following Django's design more
closely make formoxus easier for devs to understand? **Still spit-balling; no
design is decided.** His sketch is the WIP `Field<T>`/`Widget` in
`formoxus/src/widgets/widget.rs` on the `widget` branch (it does not parse:
`<T: Clone, Debug>` declares a second type param named `Debug`, and
`Vec<fn(T) -> Option<ValidationMessage>,` is missing a `>`; left as is because
it is his sketch). Django source read: 6.1.2, `django/forms/{forms,fields,
boundfield}.py`.

## Django's three classes, in one line each

- **`Field`**: pure declaration, stateless, shared by every instance:
  coercion (`to_python`), `validate`, `validators`, `required`, `label`,
  `initial`, `help_text`, `widget`, `widget_attrs()`. One subclass per value
  type.
- **`Form`**: the class is the declaration (`base_fields`); an instance is
  one request: `data` (raw `QueryDict`), `initial`, `fields` (a DEEP COPY of
  `base_fields`, so one request can change a field), lazy `errors`,
  `cleaned_data`, `full_clean()` (fields, then `clean_<name>()`, then
  `clean()` for cross-field checks on PARTIAL data). "Bound" is just
  `is_bound` (data was passed), not a separate class.
- **`BoundField`**: Field + form + name, built on access and cached. Stores
  no state of its own: `errors`, `data`, `value()`, `initial` all read back
  through the form. `build_widget_attrs` is where field facts, form policy
  and errors become HTML attributes (required, disabled, aria-invalid,
  aria-describedby).
- `Widget` renders AND reads its value back (`value_from_datadict`), which
  is the MultiWidget hook.

## Mapping onto formoxus

| Django | formoxus |
|---|---|
| `Form.base_fields` | `T`'s facet shape, walked at runtime by `members_for` |
| `ModelForm.Meta` (widgets/labels by name) | `FormSpec<T>`, `fields: IndexMap<String, FieldSpec>` |
| form instance (`fields` copy, errors) | `FormState<T>`: `{ spec, members: Vec<Box<dyn FormMember>>, errors }` |
| `Field` (subclass per type) | `FormField<T>`, `T` playing the subclass; but it also holds `value` and `errors` (state) |
| `BoundField` | `FieldProps` + `RenderCtx`, built per render |
| `initial` | `Form::initial_signal`: a whole `FormState` snapshot, structure AND values, read only by `reset` |
| `data` | `Form::values_store`: raw string per path, written per keystroke; THE source of truth for values |
| `full_clean()` | `Form::validate()`: distribute store into tree, parse, check |
| `cleaned_data`/`errors` | leaves' `FieldValue`s + errors in `state_signal`, as of the last validate |
| `Form.clean()` | `validator:` in `form!`, `fn(&T) -> Vec<ValidationError<T>>`; `ValidationError::at(path!(…))` = `add_error(field)`, `::form` = `__all__` |
| `clean_<name>()`, `Field.validators` | MISSING (per-field validators are open in [[error-model-design]]) |

Points that came out of it:
1. Reactivity does not kill `BoundField`. It changes where state lives (the
   form instance outlives a request, so state is in signals), not the need
   for a throwaway view per render. `FieldProps` already IS one.
2. `Form<T>` has no Django counterpart: it exists because a Dioxus form is
   long-lived and edited in place. Nesting: `Form` holds `Signal<FormState>`,
   which holds `FormSpec`. The spec stays inside because structural edits
   (add a row, pick a variant) rebuild part of the tree and re-apply it.
3. `FieldValue::{Empty, Valid, Invalid}` is a PARSE outcome, not bound vs
   unbound: `form_for` gives `Valid` leaves unbound, a submitted `""` gives
   `Empty` bound. Only `Invalid` implies bound. Nothing records boundness.
4. A typed `Field<T>` cannot sit in `IndexMap<String, Field<T>>` (one `T`).
   Type erasure solves it (`Box<dyn FormMember>`), not `FormSpec`.
5. Three moments: macro expansion sees only tokens (no types, no shape);
   after expansion rustc knows each field's type, reachable through the
   projection closure `shape_of(|__m: &Model| &__m.bio)` (see
   [[const-shape-walk-blocked]]); the full shape walk is runtime. So typed
   per-field `initial`/`validators` in `form!` are feasible via inference on
   the projection closure, then ERASED to cross the untyped spec map.
6. Django keeps `initial` and `data` side by side; formoxus overwrites one
   slot per leaf. "Changed" (Django `has_changed`) can be DERIVED: store vs
   `initial_signal.collect_values()`, no new state (string compare; parse
   both sides if `"3"` vs `"3.0"` matters). "Touched" cannot be derived:
   a separate `Store<HashSet<String>>` written `onblur`, built together with
   on-blur validation. Not in `FieldValue`, which lags the store.
7. On-blur validation would leave `state_signal` with values of mixed age;
   the store stays the truth for values, the state for structure (the store
   can hold stale paths after a reset). Needs a per-path validate and care
   that writing `state_signal` re-renders the whole form ([[error-model-design]]'s `ErrorsStore`).

## The idea Todd liked: hide `FormSpec` and `FormState`

Todd: "I'd love it if we only had to export Form." Truly unavailable is
impossible: `form!` expands in the dev's crate, so anything it names must be
`pub`, and def-site hygiene is unstable. The floor is `#[doc(hidden)] pub mod
__private`, already decision 7 in [[attribute-rules-design]] (2026-10-03,
"screw 'em"). But devs can be left with nothing to NAME:

```rust
form! { Address { … } }                  // emits AddressForm (Todd: default XForm for model X)
form! { AddressEdit for Address { … } }  // override syntax: OPEN
// expansion: pub struct AddressForm;
// impl ::formoxus::FormDef for AddressForm {
//     type Model = Address; type Context = ();
//     fn spec(ctx: &()) -> ::formoxus::__private::FormSpec<Address> { … } }
let form: Form<Address> = use_form(AddressForm, &());   // Todd: KEEP Form<X>, not Form<XForm>
Submission::accept(AddressForm, &(), &values)
```

- Not `impl … for Address` like `SHAPE`: the orphan rule (models from other
  crates; the reason the macros are function-like) and one model wanting
  several forms. A local marker struct avoids both. Also the closest Django
  parallel: `class AddressForm(ModelForm)`.
- `form!` becomes an ITEM for shared forms; it can stay usable as an
  expression if `use_form` takes the def by value
  (`{ struct ContactForm; impl …; ContactForm }`, block-local, no collisions).
- Todd will eat the refactor (215 `form!` uses, goldens, every
  `empty_form`/`form_for`).
- Suite: ~75 `empty_form`/`form_for` calls are inside `use_form` and move to
  the public API; ~220 are runtime-free engine tests (validate,
  collect/distribute, edit). Recommended: keep them in the suite importing
  `formoxus::__private`, NOT inline (CLAUDE.md's inline-test rule; also
  `form!` inside formoxus needs `extern crate self as formoxus;`).
- `widget_matrix` cannot use `form!` (runtime widget values; it exists to
  bypass `form!`'s compile-time refusal). Suggested: also an assertion in the
  suite, runtime-renders == `BuiltinWidget::renders` for every pair.

## `Context`: what the spec depends on per request

Django's extra `__init__` kwargs that adjust `self.fields` (a queryset
filtered by teacher). For `apcsp-dioxus`: choices by who is asking (this
teacher's sections), role-dependent fields, data-driven bounds (seats left),
maybe locale (error text runs on the server; see
[[emitted-classes-and-strings]]). A dev-defined struct, read by closures in
`form!` (`choices: |ctx| …`), typed by inference. NOT initial values (the
model), NOT live state, NOT async checks (`Submission::reject_field`).
**Rule: the server builds its own Context and never accepts one from the
client** (a forged list could widen the choices). Async loading is the
caller's job; `spec` stays synchronous. Todd 2026-10-16: a context that goes
stale while a form is open is FINE, no polling; a deleted section is caught
by choice-membership validation (`fields.rs`, "not one of the available
choices"). Todd also: putting off runtime choices was a mistake, since it
would have forced a more flexible design up front, so `Context` should be
in the trait from day one.

## Open, for Todd

A. Override syntax (`AddressEdit for Address`?).
B. `Context` in now (`context: T` in `form!`, else `type Context = ();`).
C. `form!` as item only, or item + expression.
D. Whether to rename toward Django: `FieldProps` → `BoundField`; a `Widget`
   trait with a read side; `FormSpec` is `Meta`, not `Form`.
E. Typed per-field `initial`/`validators`: via the projection closure, then
   erased in `FieldSpec`.
F. Does this replace Step 4's plan (D0–D4) or sit before it?

## THE GOAL: simplify (Todd, 2026-10-16, last thing before bed)

"This refactoring is mostly born out of a desire to simplify. We have an
awful lot of similarly named concepts that do similar things." Look at each
one carefully: which are truly necessary, which can be combined. The Django
parallel is a means; FEWER CONCEPTS is the end.

His example: if `FieldSpec` carried THE widget (not
`custom_widget: Option<WidgetType>`), fixed at compile time, much runtime
work goes away and the complexity moves into `formoxus-spec` and
`formoxus-macros`, "where God-fearing people don't need to look at it."

**The catch to check first:** `form!` sees only the fields the dev MENTIONS.
A field the dev never names (and every field inside a nested struct, a
`Vec` row or an enum variant) is found only by the runtime shape walk, and
const code cannot walk a struct's fields (crossing a `ShapeRef`; see
[[const-shape-walk-blocked]]). So "the widget is always known at compile
time" holds for mentioned fields (a `const` from `shape_of(|m| &m.x)`;
`field_kind::renders` is already a `const fn`), not for the rest, unless the
macro is given the field list some other way. Work out whether the runtime
default-widget path can at least become one `const fn` over a shape shared
by both sides.

**Inventory to go through (2026-10-16 snapshot of public type names):**
- Form layer: `Form`, `FormState`, `FormSpec`, `FieldSpec`, `SpecsByPath`,
  `Formoxus` (app defaults), `Submission`, `WireForm`.
- Members/fields: `FormMember`, `FormField`, `FieldValue`, `FieldAttrs`,
  `FieldSet`, `ListSet`, `VariantSet`, `OptionMember`, `Edit`, `RenderCtx`,
  plus the new sketch's `Field`.
- Widgets: `WidgetType`, `BuiltinWidget`, `ScalarWidget`, `WidgetProps`,
  `FieldProps`, the new sketch's `Widget`, `Part`.
- Elements/types: `Element`, `FieldControl`, `InputType`,
  `RenderedInputType`, `FieldType`, `FieldTypeWithOptional`.
- Values: `ValuesByPath` vs `ValuesStore` (same map, two containers).
- Errors: `ValidationError`, `ValidationMessage`, `FormErrors`,
  `ErrorsByPath`, `FormError`, `FieldError`, `FormAccessError`.
- Choices: `Choice` vs `VariantChoice`.
- Attributes: `Attr`, `AttrKey`, `AttrType`, `AttrValue`, `FixedKey`,
  `Owner`, `Bound`.
- Buttons/slots: `ButtonSpec`, `ButtonType`, `ButtonFn`, `Fns`, `Handler`,
  `UncheckedHandler`, `Provider`, `IntoSlot`, `Invocation`, `Slot`,
  `SlotKey`, `Classes`, `ClassEdit`.
- formoxus-macros has its OWN `FieldSpec` (plus `FormSpecInput`,
  `FormSpecMeta`, `FieldBody`, `SpecPath`, `PathInput`, `WidgetRef`, …),
  which collides in name with the runtime one.
