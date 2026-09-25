# What `form!` should accept

An inventory of every constraint and setting, and which of the three tiers it
belongs to.

> **Status, 2026-09-25.** Tier 1 is reachable and CHECKED: `form!` accepts the
> five constraint keys, they reach `ValueKind::check` on both sides of the wire,
> and "Compile-time checks" below are built, except whether a bound fits the
> field's type. Tier 3 is mostly built. Tier 2 has its syntax but no attributes,
> and no widget renders any constraint attribute yet either. **`CONSTRAINTS_NEXT.md`
> is the build order** for what remains; this file stays the inventory.

## The three questions that decide the tier

1. **Does it change which values are VALID?** → tier 1, a field-level constraint.
   formoxus enforces it on both sides of the wire, and it survives a widget swap.
2. **Does it change how the widget looks or behaves, without changing validity?**
   → tier 2, a widget attribute, inside `widget: x { … }`, gated by widget.
3. **Does it apply to the whole form?** → tier 3.

The tier-1/tier-2 line is load-bearing, not tidiness: a constraint in the widget
block would be silently discarded by a widget override, which is what
`fields.rs`'s note about constraints living on the value kind exists to prevent.

```rust
bio => {
    max_length: 500,                        // tier 1 — survives a widget swap
    widget: textarea { rows: 10 },          // tier 2 — presentation
},
```

---

## Tier 1 — field-level constraints

Enforced by `ValueKind::check` (built) plus the per-field validator (not built).
Rendered as an HTML attribute *only where the element accepts one* — see
"rendering" below.

| key | value kinds | HTML attr | status |
|---|---|---|---|
| `min_length` | Text | `minlength` | check built |
| `max_length` | Text | `maxlength` | check built |
| `pattern` | Text | `pattern` | check built |
| `min` | Int, Float, (Temporal) | `min` | check built |
| `max` | Int, Float, (Temporal) | `max` | check built |
| `validator` | any | — | not built |
| `help_text` | any | — | not built |
| `initial` | any | — | not built |
| `choices` | — | — | stays TIER 2, see below |

Notes:

- **`pattern` must be a `LitStr`.** Regex parsing allocates, so it cannot happen
  in a `const` block; the macro has to see the literal to validate it with
  `regress`. Every other tier-1 value may be any const expression.
- **`step` is deliberately NOT offered.** It is a spinner affordance that happens
  to also be a constraint, and an author who wants it can write
  `(value - min) % step == 0` as a validator in two lines.
  **But one consequence survives that decision:** `type="number"` defaults to
  `step="1"`, so a browser rejects `1.5` in a float field rendered as `number`
  even though formoxus accepts it. A `Float` field with `widget: number` should
  therefore emit `step="any"` automatically — not a feature, a divergence fix.
- **`help_text`** is pure formoxus (no HTML equivalent); Django has it and it is
  a straightforward parity item. Renders as a hint near the field, and wants an
  `aria-describedby` wiring to be worth anything.
- **`initial`** is a create-mode default. `empty_form` currently starts every
  field empty.

### `choices` stays in tier 2 — SETTLED 2026-09-21

It looks like it belongs in tier 1, since the membership check makes choices
decide which values are valid. It does not, for two reasons.

**The tier follows whether the SERVER can independently know the list**, and only
one of the three kinds of choices qualifies:

| kind | in the spec? | who checks membership |
|---|---|---|
| static | yes | `Submission::accept`, from the shared spec |
| render-time (fetched, or from a render parameter) | no | the author's validator, which can redo the fetch |
| reactive (depends on other fields) | no | cross-field by definition — the form validator |

Splitting one concept across two syntaxes to capture the static case would be
worse than the thing it fixes, since the other two kinds have to live in the
widget block regardless.

**And a widget swap cannot silently drop them.** The macro already gates
`choices` to the four choosers, so `widget: text { choices: … }` is a compile
error — losing the validation requires deleting the `choices` line, which is
explicit. A swap *among* choosers (`select` → `radio_group`) keeps them, because
all four accept choices.

That is the real difference from `max_length`: `text` ↔ `textarea` ↔ `password`
is a common swap and all three accept the constraint, so a widget-block
`max_length` would vanish unnoticed. Choosers only swap among themselves.

---

## Tier 2 — widget attributes

Inside `widget: <name> { … }`. The brace syntax is built; only `choices` is
parsed so far. Gate by widget from one table, grouped so shared attributes are
declared once — see the `widget_attrs!` sketch in the design discussion.

**global** (every widget)
`class` · `id` · `style` · `title` · `tabindex` · `autofocus` · `hidden` ·
`data-*` · `aria-*`

**editable** (anything you type into)
`placeholder` · `readonly` · `autocomplete` · `inputmode` · `spellcheck` ·
`autocapitalize` · `size`

**textarea only** — `rows` · `cols` · `wrap`

**select only** — `size` (visible rows; a *different* meaning from an input's
character width, same name and type) · `multiple`

**file only** — `accept` · `capture` · `multiple`

**choosers** — `choices` (see the inconsistency above)

`data-*` and `aria-*` cannot be enumerated, so they need the string-literal
escape rsx already uses: `"data-testid": "bio"`.

### Two that are not purely presentational

- **`disabled`** — a disabled control does not submit at all, so its path is
  absent from the wire and `apply_leaves` leaves the value untouched. That is a
  semantic consequence, not styling, and wants documenting wherever it lands.
- **`readonly`** — does submit, unlike `disabled`. Worth stating the difference
  in the same place.

---

## Tier 3 — form-level

| key | status |
|---|---|
| `title` | built |
| `label_case` | built |
| `browser_validation` | built (emits `novalidate`; leaves input attributes alone) |
| `validator` | built (form-wide; cannot target a field yet — see the error-model memory) |
| `buttons` | built |
| `class` / `id` on the `<form>` | not built |
| `autocomplete` (form-wide `off`) | not built |

---

## Deliberately NOT settable

- **`name`** — derived from the field path, which IS the wire format.
- **`value`** — comes from the model.
- **`type`** — that is what `widget` selects.
- **`required`** — derived from `Option<T>`, so the type decides. The one gap:
  "this checkbox must be ticked" is not expressible, because a `bool` field is
  always satisfied by `false`. If that is wanted it is a validator, not a
  `required`.

---

## Rendering: which attributes actually reach the DOM

A constraint always applies; the *attribute* is emitted only where HTML accepts
it. This is a render-time decision and **never a compile error** — the case that
settles it is that `Int`'s default widget is `text` (deliberately, so a
half-typed value does not vanish) and `text` accepts neither `min` nor `max`, so
compile-time gating would reject `count => { min: 0 }`, the most ordinary
declaration in the library.

Verified applicability:

- `minlength` / `maxlength` → text, search, url, tel, email, password, **and
  `<textarea>`**
- `pattern` → text, search, url, tel, email, password. **NOT `<textarea>`.**
- `min` / `max` / `step` → number, range, date, month, week, time,
  datetime-local. **NOT text.**

The same table drives tier 2, so build it once.

---

## Compile-time checks

Every one of these is about the field's TYPE or the constraint's own values,
never the widget. Where HTML accepts an attribute is a render-time question;
see "Rendering" above.

**The macro does these itself, from tokens, while it parses.** It has to be
at parse time because `expand` has no error channel, and `impl_form` only
returns `Err` from parsing.

- duplicate and unknown keys
- `pattern` compiles under `regress`, exactly as HTML's "compiled pattern
  regular expression" does it: the bare pattern, then `^(?:…)$`, both with the
  `v` flag. This one cannot be a const check, because compiling a regex
  allocates. It is also why `pattern` is a `LitStr` and not an `Expr`. *Built.*

**rustc does these, through free `const _` items that `form!` emits.**
`formoxus::field_kind` explains why they are free const items and not a
`const {}` block in a generated generic fn: `cargo check` never evaluates the
latter, and nothing ever evaluates it inside the uncalled `__paths_exist`. Each
assertion is spanned onto the author's value, so the caret lands there.

- **Constraint vs. value kind.** `max_length` on a `bool`, `min` on a `String`,
  or anything on a struct path. The field's type is inferred from a projection
  closure, `shape_of(|__m: &Model| &__m.field)`, and classified the way
  `member_for_shape` classifies it. A plain newtype is let through as "can't
  tell". *Built.*
- **`min > max`, `min_length > max_length`.** These are const asserts rather than
  token comparisons so that `min: MIN_AGE` and `max: 2 * N` are evaluated too,
  with `as f64` letting an integer and a float compare. The cost is a fixed
  message that cannot quote the values. *Built.*
- **Does a bound fit the field's type?** `max: 1e50` on an `f32`, or
  `min: -1000` on an `i8`, which is vacuous. *Not built; step 6b.*
- the validator's parameter type matching the field's, via a witness call.
  *Not built; belongs to the error model.*

A shape-based check beats marker traits here: one source of truth with
`value_kind()`, no hand-kept impl list, no orphan problem, and it can say "can't
tell" for a newtype instead of rejecting it.
