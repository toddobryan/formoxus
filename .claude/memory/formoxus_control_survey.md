---
name: formoxus-control-survey
description: "Survey of Django's and leptos_form's form-control taxonomies, the verified facet attribute-grammar capabilities, and the ControlType/ValueKind/Constraint design that came out of it — conversation of 2026-09-10 into 09-11, DESIGN ONLY, nothing implemented"
metadata:
  node_type: memory
  type: project
---

**Status: NOTHING HERE IS IMPLEMENTED.** This is the record of a design
conversation on the night of 2026-09-10 into the morning of 2026-09-11, on
branch `facet` at `0e713a8`. It picks up from the handoff in
[[facet-form-design-decisions]] "Retiring the derive path, part 1: the control
taxonomy", which ended on an open question. That question is now answered (see
"Decisions" below) and the design has moved considerably past it.

The expensive parts to re-derive are the two surveys and the facet
attribute-grammar findings, which were read out of source rather than
remembered. Line numbers are from the vendored 0.46.5 crates.

## Where we actually stopped

Todd's last instruction: **he is writing what he wants `LoginForm` to look
like**, and we will work backwards from that to the API, rather than continuing
to design the taxonomy forward. He said "I'm getting lost in the weeds," which
is the right call — the taxonomy discussion below had gotten three levels deep
into cases (`month`/`week` carriers, `Vec<T>` ambiguity) that Login does not
touch.

So: **do not resume by building `ValueKind`.** Resume by reading Todd's ideal
`LoginForm` and diffing it against what exists.

The four things the reflect path lacks for Login parity, established by reading
both call sites:

| gap | cost |
|---|---|
| title | `FormState.title: Option<String>` already exists (form.rs:146-152); needs a setter and rendering |
| password override | the real work — see "Decisions" |
| form-level errors rendered | `FormState.errors: Vec<FormError>` already exists; nothing draws it |
| buttons/handlers | ~zero; the page owns `form` and writes its own `<button>`, as `form_demo.rs` does |

Current reflect call site (`crates/web/src/views/form_demo.rs`):

```rust
let form = use_form(empty_form::<T>());   // or form_for(&t)
{ form.render() }                          // fields only
form.validate() -> Option<T>
form.values() -> Store<HashMap<String, String>>
```

Current derive `LoginForm` (`crates/api/src/auth.rs`, `crates/web/src/views/auth/login.rs`):

```rust
#[derive(Form, Debug)]
#[form(title = "Sign In", button(type = "submit", name = sign_in, text = "Sign In"))]
pub struct LoginForm {
    pub username: String,
    #[form(component = PasswordInput)]
    pub password: String,
}
```

## Decisions reached

**1. Password rides on a form-centric attribute — for form-purpose structs only.**
Todd's words: "for structs that just represent forms (like Login), it's better to
decorate the fields so that the presentation and field are together. For
independent models (like Question), we're going to have to figure out a way to
attach attributes after the fact. But let's do Login with attrs."

So there are TWO mechanisms, not one, and which applies depends on whether the
struct exists to be a form. The after-the-fact mechanism for domain models is
still undesigned — it is the widget-registry question, still open.

Rejected earlier the same day, and still rejected: keying off
`#[facet(sensitive)]`. `sensitive` means "redact in Debug"; coupling it to widget
choice means marking a field for LOGGING silently changes its rendering.

**2. Do the taxonomy split BEFORE Login**, so Login is written against the final
shape and never has to be touched twice. (Todd chose this over "after Login".)

**3. Constraints nest in the VALUE kind, not in `ControlType`.** Todd pushed back
on making `Constraint` orthogonal — "which constraints are allowed are dependent
on the control/input types" — and he was right that they must nest in something,
but the axis is the value type, not the control. The deciding argument:

- A `String` with `max_length`, overridden from `Text` to `Textarea` or
  `Password`, must KEEP `max_length`. If the constraint lives inside the control
  variant, a presentational override silently discards validation.
- Django agrees: `max_length` is a `CharField` argument, not a `TextInput` one,
  and survives rendering as `Textarea`/`PasswordInput`/`EmailInput`.
- Grouping by control also inherits HTML's irregularities: `min`/`max`/`step`
  apply to `date`/`month`/`week`/`time`/`datetime-local` as well as `number`/
  `range` (so the predicate is "ordered", not "numeric"), and `pattern` applies
  to `text`/`search`/`url`/`tel`/`email`/`password` but NOT to `textarea`.

**4. `InputKind` was misnamed, not mis-shaped.** Its variants
(`Text`, `Boolean { optional }`, `Int { min, max }`, `Float`) are value families,
not HTML elements — and `Int { min, max }` is already "value family plus its
constraints", which is the target shape. So `InputKind` -> `ValueKind` is mostly a
rename plus fields; `ControlType`/`InputType` is the genuinely new construction.
This makes the split cheaper than it first looked.

**5. Control resolution must happen at-or-before VALIDATE, not at render.**
Todd proposed storing the `Shape` and resolving the control late, so the default
is never "overruled". The principle is right; two facts constrain the mechanism:

- `input_kind` is NOT render-only. It is read at `fields.rs:125` (gates the
  *required* error) and `fields.rs:152` (gates `partial.set(false)`), both via
  `is_unticked_checkbox`. And for a real reason: **a checkbox omits its value
  when unticked**, which is a property of the control and is what makes blank
  mean `false`. `validate(&mut self)` takes no context to thread it through.
- **`Shape` alone is insufficient.** Because `Option` peeling wraps rather than
  parameterizes, both `bool` and `Option<bool>` produce a `FormField<bool>` —
  the Option-ness lives in the `OptionMember` decorator. `T::SHAPE` is `bool`'s
  shape either way, and that flag is exactly what separates checkbox from
  tri-state select.

Also: **no need to store a `Shape` at all.** `FormField<T>` is already generic
over `T: Facet`, so `T::SHAPE` is free inside every method. A `shape` field
would duplicate the type parameter.

Resulting arrangement — default as a FUNCTION, override as a SLOT:

```rust
pub struct FormField<T: …> {
    pub name: String,
    pub label: Option<String>,
    pub optional: bool,                   // what the walk learned; not a control property
    pub control: Option<ControlType>,     // None = nobody overrode this
    pub value: FieldValue<T>,
    pub errors: Vec<FieldError>,
}
fn default_control(&self) -> ControlType  // from ValueKind + self.optional
fn control(&self) -> ControlType { self.control.clone().unwrap_or_else(|| self.default_control()) }
```

`is_unticked_checkbox` becomes `matches!(self.control(), Input(Checkbox))`, which
reads better than today's `Boolean { optional: false }`.

Incidental win: pulling `optional` out as its own field. It was never a control
property — it is a structural fact from the build walk, and having it live inside
`InputKind::Boolean { optional }` is why that variant has to dissolve.

**6. Write the enums comprehensively, then comment out the unimplemented ones**
(Todd: "they'll be there, mocking us until we get around to implementing them").
One exception agreed: `File` should be a COMMENT, not a commented-out variant,
because its blocker is a data-model change and not effort — see below.

**7. Enforcement and authoring of bounds are COUPLED.** No point adding a bounds
check to `validate()` before author bounds exist, because the type's own bounds
are already enforced by `parse_scalar` — `"300".parse::<u8>()` fails on its own.
The interesting bounds are the author's, narrower than the type. So
`Constraint::Range` and `#[facet(formoxus::min(…))]` land in the same commit or
neither does.

## A correction worth keeping

I framed numeric bounds as "for the error message, not for HTML min/max" and let
that imply they were not client-enforceable. Todd corrected it: bounds available
client-side let us **skip a server round-trip**. Two separate questions were
collapsed:

1. Can the BROWSER enforce it natively via `min=`/`max=`? No — we render text.
2. Can WE enforce it client-side before a round-trip? Yes — `validate()` runs in
   wasm.

Only (1) is blocked by rejecting `type="number"`. The corrected axis, which is
the same two-tier shape already recorded for `Provider` scoping in
[[formoxus-feature-parity]]:

| tier | who runs it | standing |
|---|---|---|
| client (`validate()` in wasm) | our Rust, pre-submit | UX only, never a boundary |
| server | our Rust, authoritative | the actual boundary |
| browser-native (`min=`, `maxlength=`) | the UA | opportunistic, per control type |

This also strengthens the deferred blur-validation item: parse errors mostly
surface on their own, but a range violation is invisible until submit. Bounds are
what make blur validation worth building.

## facet attribute grammar — VERIFIED capabilities

Read from `facet-macros-impl-0.46.5/src/attr_grammar/make_parse_attr.rs` and
`facet-0.46.5/src/lib.rs`.

**The cost is LOWER than [[facet-form-design-decisions]] estimated.** That note
said formoxus "must export an `__attr!` macro … a small proc-macro surface".
There is no proc-macro code to own: `facet::define_attr_grammar!` generates the
attribute types, the `#[macro_export] macro_rules! __attr!` dispatcher, and the
proc-macro re-exports from a declarative block. Pre-1.0 churn risk still stands;
the authoring cost does not.

```rust
facet::define_attr_grammar! {
    ns "formoxus";
    crate_path ::formoxus;

    pub enum Attr {
        Password,                        // unit
        Min(i64), Max(i64),              // newtype i64
        Validate(validator ValidatorFn), // fn(&T) -> Result<(), String>
    }
}
```

**Integers work as newtype variants but NOT as struct fields.** So
`#[facet(formoxus::range(min = 0, max = 100))]` will NOT compile:

- line 322: "Newtype holding `i64` — for numeric validation attributes like `min`, `max`."
- line 686: `i64 -> NewtypeI64`
- line 779: `"unsupported field type: {ty_str}. Supported types: bool, &'static str, Option<&'static str>, Option<bool>, Option<char>"`

What works is two separate attributes:
`#[facet(formoxus::min(0))] #[facet(formoxus::max(100))]`. Arguably better, since
min and max are independently optional. Note the carrier is `i64`, so a bound
above `i64::MAX` on a `u64` field is not expressible — our own
`InputKind::Int { min, max }` uses `i128` precisely because `u64::MAX` overflows
`i64`. Author bounds and type bounds therefore cannot share a representation
without a widening.

**There is a `validator` variant kind, and it is a bigger deal than min/max.**
`ValidatorFn = unsafe fn(value: PtrConst) -> Result<(), String>`
(`facet-core/src/types/ty/field.rs:353`, re-exported at `facet::ValidatorFn`). A
wrapper is generated so auto-deref works — you write `fn(&str) -> Result<(),
String>` for a `String` field, not `fn(&String)`. facet's own builtin grammar
uses the sibling `predicate` kind for `#[facet(skip_serializing_if = is_empty)]`;
nothing builtin uses `validator`, it exists for extension crates.

Three limits, the first of which matters to the derive retirement:

1. **Container-level validators are explicitly rejected** — the generated macro
   emits `compile_error!("Container-level predicate attributes like `…` are not
   supported")`, because no `$ty` is available there. So
   `check_new_and_confirm_match` (cross-field, form-level, and required for
   `ChangePasswordForm`) CANNOT ride on this. Form-level validation stays
   formoxus's own mechanism.
2. The validator sees `&T`, the PARSED value — right for bounds, but it cannot
   inspect what the user typed when parsing failed.
3. The fn pointer is stored raw in `attr.data`, not wrapped in the `Attr` enum,
   so reading it back needs a small unsafe accessor modeled on facet's
   `skip_serializing_if_fn()`, and calling it needs a `PtrConst` to the parsed
   `T`.

**Recommendation given:** take `Min`/`Max` now (plain data, no unsafe), leave
`Validate` undeclared until `Question`.

## Survey: leptos_form has essentially NO constraint support

Read from source, not docs.rs — the docs page does not show this.

| file | types | `Config` |
|---|---|---|
| `impls/str.rs` | `String`, `Cow<str>`, `Oco<str>` | `()` |
| `impls/num.rs` | all 12 int types, `f32`, `f64` | `()` |
| `impls/misc.rs` | `Uuid` | `()` |
| `impls/misc.rs` | `NaiveDate`, `NaiveDateTime`, `DateTime<Utc/Local/FixedOffset>` | `{ format: &'static str }` (strftime only) |
| `impls/collections.rs` | `Vec<T>` | `VecConfig { item_label, size, remove }` |

No `maxlength`, `minlength`, `min`, `max`, `step`, `pattern`, or `placeholder`
anywhere. Its field attributes are all presentation: `class`, `style`, `id`, `el`,
`group`, `label`, `error`, `config`. Struct-level: `component`, `error`,
`field_class`, `groups`, `id`, `label`, `wrapper`.

**So leptos_form is not a source of constraint design.** Three things there ARE
worth taking:

- **`el` as a field attribute** is our `ControlType`, expressed as a Rust type
  (`HtmlElement<Input>` vs `HtmlElement<Textarea>`) with a `DefaultHtmlElement`
  trait giving the per-type default. Same shape as the type-directed registry,
  and it confirms "element choice is per-field, defaulted per-type".
- **`error` as a 5-way mode** (`component`/`container`/`default`/`none`/`raw`),
  settable at struct OR field level — the configurable-error-rendering gap
  already listed in [[formoxus-feature-parity]].
- **`f32`/`f64` render `type="text"` while the integers use `type="number"`** —
  someone else hit the same wall Todd did.

## Survey: Django, which is the one worth mining

Django separates exactly what we are separating, and the bridge between them has
a name worth stealing: **`Field.widget_attrs(widget)`** — a constraint that has
an HTML spelling gets pushed into the widget's attrs automatically; one that does
not stays server-side. `max_length` on a `CharField` becomes `maxlength=` without
anyone asking.

- **`Field`** = coerce + validate. Owns the constraints.
- **`Widget`** = render. Owns the element.

### Constraints, grouped by value type

| value kind | arguments |
|---|---|
| text | `max_length`, `min_length`, `strip`, `empty_value`; `RegexField` adds `regex`; `SlugField` adds `allow_unicode` |
| numeric | `max_value`, `min_value`, `step_size`; `DecimalField` adds `max_digits`, `decimal_places` |
| date/time | `input_formats` (`input_date_formats`/`input_time_formats` on `SplitDateTimeField`) |
| choice | `choices`; typed variants add `coerce`, `empty_value`; `ModelChoiceField` adds `queryset`, `empty_label`, `to_field_name` |
| file | `max_length` (filename), `allow_empty_file` |
| repeating | (formsets) `extra`, `min_num`, `max_num`, `can_delete`, `can_delete_extra`, `can_order` |
| every field | `required`, `label`, `label_suffix`, `initial`, `widget`, `help_text`, `error_messages`, `validators`, `localize`, `disabled` |

### Widgets

`Input` subclasses: `TextInput`, `NumberInput`, `EmailInput`, `URLInput`,
`PasswordInput`, `HiddenInput`, `ColorInput`, `SearchInput`, `TelInput`,
`DateInput`, `DateTimeInput`, `TimeInput`. Other: `Textarea`. Selectors:
`CheckboxInput`, `Select`, `SelectMultiple`, `NullBooleanSelect`, `RadioSelect`,
`CheckboxSelectMultiple`. File: `FileInput`, `ClearableFileInput`. Composite:
`MultiWidget`, `SplitDateTimeWidget`, `SplitHiddenDateTimeWidget`,
`SelectDateWidget`, `MultipleHiddenInput`.

### Three findings that corroborate choices already made here

1. **Django independently reached the "madness" conclusion twice.** `DateField`'s
   default widget renders `type="text"`, NOT `type="date"` — you opt in with
   `attrs={"type": "date"}`. And `IntegerField`/`FloatField`/`DecimalField` fall
   back from `NumberInput` to `TextInput` whenever `localize=True`, because
   browser number inputs mishandle locale. "Parse it ourselves, render text" is
   the mainstream position.
2. **`CheckboxSelectMultiple` deliberately omits `required`** on its individual
   checkboxes — the browser would demand all of them be ticked. Same trap
   `BooleanInput` already sidesteps, arrived at independently.
3. **`NullBooleanField` -> `NullBooleanSelect` with Unknown/Yes/No** is precisely
   our `Option<bool>` tri-state. Direct corroboration for
   `Boolean { optional: true }` becoming `Select`, and for the shape supplying
   those particular choices.

## The control taxonomy, comprehensively

### All 22 `<input type>`s

| type | submits | needs beyond a value | verdict |
|---|---|---|---|
| `text` | String | — | live |
| `password` | String | — | live (what Login needs) |
| `checkbox` | presence | — single / choices for a group | live |
| `search`, `tel`, `url`, `email` | String | — | stub; pure cosmetics |
| `hidden` | String | — | stub; wanted for round-tripping ids |
| `color` | String `#rrggbb` | — | stub |
| `date`, `time`, `datetime-local`, `month`, `week` | String | — | stub |
| `number` | String | min/max/step | stub; already ruled out |
| `range` | String | min/max/step REQUIRED to mean anything | stub |
| `radio` | String | choices + shared `name` | stub; see trap 3 |
| `file` | NOT a String | — | comment, not a variant |
| `button`, `submit`, `reset`, `image` | nothing / coords | — | EXCLUDE |

The last four are not field controls — they produce no value for a model field,
and the page already owns its own buttons. Including them would make
`ControlType` mean "any input element" rather than "how this field is edited".

### Elements for `ControlType`

`Input(InputType)` live; `Select { multiple, size }` live; `TextArea { rows,
cols, wrap }` stub (cheap, and wanted for `Markdown`); `Output`, `Progress
{ max }`, `Meter { min, max, low, high, optimum }` excluded as OUTPUT elements —
including them widens the type from "how the user edits this" to "how this is
displayed".

`datalist` is a modifier on `Input(Text)` via `list=`, not a control of its own —
same category as `placeholder`.

### Four traps

1. **`datetime-local` is the only variant whose HTML string is not its lowercased
   name.** A derived `as_str()` is right 21 times out of 22 and silently wrong
   once. Use an explicit match.
2. **There is no `type="datetime"`** — removed from the spec, replaced by
   `datetime-local`. Browsers fall back to `text`.
3. **`radio` and checkbox-groups break "one path, one element."** Both render
   several DOM nodes sharing one `name`. Everything in `reflect/` assumes a path
   maps to one control, `get_current`/`write_value` included. Radio is not "a
   select with different CSS".
4. **`file` does not fit the value model at all.** `ValuesByPath` is
   `HashMap<String, String>`; a file input's value is a `FileList`. Django
   threads `files` as a separate dict alongside `data`
   (`value_from_datadict(data, files, name)`), which hints at the eventual shape.
   A commented-out variant would imply the blocker is effort.

## The eight ValueKinds, and the rule that bounds the list

| ValueKind | carrier | constraints | controls |
|---|---|---|---|
| `Text` | `String` | `min_length`, `max_length`, `pattern` | text, search, tel, url, email, password, textarea |
| `Int` | `i8`…`i128`, `u8`…`u64` | `min`, `max`, `step` | number, range, text |
| `Float` | `f32`, `f64`, later `Decimal` | `min`, `max`, `step` | number, range, text |
| `Bool` | `bool` | — | checkbox, select (tri-state) |
| `Temporal` | 5 carriers | `min`, `max`, `step` | date, time, datetime-local, month, week |
| `Choice` | enum / key / `RecordId` | membership in a set | select, radio |
| `MultiChoice` | `Vec<T>` / `HashSet<T>` | membership + cardinality | select multiple, checkbox group |
| `File` | NOT a String | `accept`, `multiple`, max size | file |

**The rule: a family earns a variant only if it has a distinct constraint
vocabulary.** That test removes four apparent candidates — `color` (no constraint
attributes apply at all), `email`/`url`/`tel`/`search` (their authorable
constraints are Text's; the browser validates by type), `hidden` (a control
applied to any kind), `range` (same value as number, different widget). Four
things that look like types turn out to be presentation, which is the split
earning its keep.

### Two places it gets genuinely hard

**Temporal is one vocabulary over five carriers.** `min`/`max`/`step` apply to
all, but `step` means days for `date`, seconds for `time`, months for `month`. So
`Temporal { kind: TemporalKind, min, max, step }` rather than five variants. And
**`month` and `week` have no natural chrono type** — `chrono::Month` is a month
NAME, `IsoWeek` is only obtainable from a date. Both need newtypes; they are the
two to comment out hardest.

**`Choice`/`MultiChoice` collide with member kinds we already have.**

- A FIELDLESS enum is a `Choice`, but today it goes through `VariantSet`, which
  builds a selector plus an always-empty members region. `SelectType` and
  `AnswerChoice` on `Question` are exactly this shape, so "when is an enum a
  scalar choice rather than a variant set?" arrives WITH `Question`, not later.
- `Vec<T>` is ambiguous. `Vec<Struct>` is unmistakably `ListSet`. `Vec<SomeEnum>`
  could be a multi-select or a list of row-level pickers, and nothing in the
  shape distinguishes them.

Both are resolved by the call site or registry rather than by the shape — the
same conclusion already reached about a select's options, now one level up, to
whether something is a select at all.

### Why the ordering is convenient

Only `Text`, `Int`, `Float`, `Bool` are reachable from the scalars `dispatch!`
handles today, and those four are all that Login and `ChangePasswordForm` need.
`Temporal` needs `Facet` on chrono types; `Choice`/`MultiChoice` need the
registry; `File` needs the second value channel. The comment-out line falls
exactly where the prerequisites start.

## The three-way arrangement, as it stands

| lives on | overridable? | who reads it |
|---|---|---|
| `ValueKind` (with constraints) | no — derived from `T` | `validate`, `write_value_into`, `default_control` |
| `optional` | no — from the build walk | `default_control`, `is_unticked_checkbox` |
| `control: Option<ControlType>` | yes — attribute or call site | `render` |

---

# CORRECTION (2026-09-12): the attribute path was BUILT, then ABANDONED

**Everything above about the SURVEYS still stands** — Django, leptos_form, the 22
input types, the 8 ValueKinds, the rule that bounds the list. **What is superseded
is this file's recommendation**, which was "form-centric facet extension
attributes for form-purpose structs". That was implemented, proven to work, and
then deleted the same day in favour of formoxus's own proc macro. See
[[facet-form-design-decisions]] "The `form2!` decision" for the reasoning; this
section keeps the *evidence*, because it took hours to establish and is the only
reason the decision is defensible rather than a coin flip.

## It worked. That is the point.

`crates/formoxus/src/reflect/attrs.rs` held a real `define_attr_grammar!` block
with `Password`, `Control(Option<ControlType>)`, `Label(&str)`, `Title(&str)`, and
six passing tests in `crates/formoxus/tests/reflect/facet_forms.rs`. All deleted
at Todd's instruction once `form2!` won. **Do not re-litigate this by rebuilding
it** — it is not that the mechanism failed, it is that a proc macro does strictly
more for the same cost.

## Verified facts about `define_attr_grammar!` (0.46.5)

Read from `facet-macros-impl-0.46.5/src/attr_grammar/make_parse_attr.rs`. These
are the expensive ones, all confirmed by compiling rather than by reading alone.

**The authoring cost is LOW.** No proc-macro code to own: the macro generates the
attribute types, the `#[macro_export] macro_rules! __attr!` dispatcher, and the
proc-macro re-exports from one declarative block. The earlier note that formoxus
"must export an `__attr!` macro … a small proc-macro surface" overstated it.

**Value kinds a variant may hold:** unit; `&'static str`; `i64`; `usize`
(explicitly "for length validation attributes like `min_length`, `max_length`");
`Option<&'static str>`; `Option<char>`; a struct declared in the same block;
`shape_type` (stores `&Shape`); `predicate` (`fn(&T) -> bool`); `validator`
(`fn(&T) -> Result<(), String>`); `fn_ptr`; `make_t`; `arbitrary`; and
`ArbitraryType` — any other type, holding a real Rust value.

**Struct-shaped variants are the restricted one.** Their fields may only be
`bool`, `&'static str`, `Option<&'static str>`, `Option<bool>`, `Option<char>` —
NO integers. So `#[facet(ns::range(min = 0, max = 100))]` cannot work while
`#[facet(ns::min(0))]` can.

**Four traps, each of which cost a compile cycle:**

1. **A lone PascalCase identifier is parsed as a STRUCT variant**, not
   `ArbitraryType` — "struct variant must reference a defined struct". Only a
   *multi-token* type path falls through, so it must be written
   `Control(crate::reflect::widgets::ControlType)`.
2. **`ArbitraryType` payloads must be `Option<T>`.** The dispatcher hands the
   value through wrapped in `Some`, so `Control(ControlType)` is a type error.
   The kind's own doc example is `Option<DefaultInPlaceFn>`.
3. **Payload types must implement `Facet`.** A non-builtin grammar with no
   function-pointer variants derives `Facet` on the generated `Attr`. (Adding any
   `validator`/`predicate` variant suppresses that derive and lifts the
   requirement — a strange coupling, but real.)
4. **Payloads must be `const`-constructible.** The value lands in
   `static __ATTR_DATA: Attr = …;`, which is what buys full rustc type-checking —
   but it forbids `String` and `Vec`. A `ControlType::Select { choices: Vec<_> }`
   could never be an attribute payload.

**The read-back protocol is NOT uniform, and `get_as` is shape-checked, so asking
wrongly returns `None` silently rather than failing:**

| declared kind | what `Attr.data` holds | how to read it |
|---|---|---|
| unit | `static __UNIT: () = ()` | `has_attr(ns, key)` — presence IS the signal |
| `&'static str` | the `&str`, bare | `get_as::<&'static str>()` |
| arbitrary type | the generated `Attr` enum | `get_as::<Attr>()` |
| `shape_type` | a bare `&Shape` | `proxy_shape()`-style accessor |
| predicate/validator | a raw function pointer | unsafe read of `data` |

**Container-level `validator`/`predicate`/`make_t` are REJECTED** with a
`compile_error!`, because the generated macro receives `{ $field:tt : $ty:ty | … }`
at field level and `{ | … }` at container level — no `$ty` to wrap the function
against. **A variant is treated as a container, not a field**, for the same
reason. So `check_new_and_confirm_match` (cross-field, form-level) could never
have ridden on this mechanism.

**`#[storage(flag)]` / `#[storage(field)]` do nothing for an extension crate** —
`Storage` is `#[allow(dead_code)]`, "used for documentation/validation only …
routing is hardcoded in process_struct.rs for builtin attrs". So every formoxus
attribute would have been an O(n) scan of `field.attributes`, never a flag check
like `is_sensitive()`.

**Tests exercising the attributes CANNOT live inside formoxus.**
`#[facet(formoxus::password)]` routes to `formoxus::__attr!`, and the path
`formoxus::` does not resolve inside formoxus itself. That is why
`crates/formoxus/tests/reflect.rs` exists as a target — and it needs `#[path]`
for submodules, since cargo only auto-discovers `tests/*.rs`.
