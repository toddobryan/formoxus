# What `form!` should accept

An inventory of every constraint and setting, and which of the three tiers it
belongs to. Status as of 2026-09-21: tier 3 is mostly built, tier 1 is validated
but unreachable (nothing sets it), tier 2 has its syntax but no attributes.

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
| `step` | Int, Float, (Temporal) | `step` | **not designed yet** |
| `validator` | any | — | not built |
| `help_text` | any | — | not built |
| `initial` | any | — | not built |
| `choices` | any scalar | — | built, but in TIER 2 — see below |

Notes:

- **`pattern` must be a `LitStr`.** Regex parsing allocates, so it cannot happen
  in a `const` block; the macro has to see the literal to validate it with
  `regress`. Every other tier-1 value may be any const expression.
- **`step` is missing from the first pass and is a real constraint** — HTML
  reports `stepMismatch`, so a browser will block a value formoxus would accept.
  Gotcha: `type="number"` rejects decimals unless `step="any"` or a fractional
  step is given.
- **`help_text`** is pure formoxus (no HTML equivalent); Django has it and it is
  a straightforward parity item. Renders as a hint near the field, and wants an
  `aria-describedby` wiring to be worth anything.
- **`initial`** is a create-mode default. `empty_form` currently starts every
  field empty.

### The `choices` inconsistency — decide this

`choices` shipped inside the widget block (`widget: select { choices: STATES }`).
That was right when choices were a *menu*. They are now also a **domain**: the
membership check makes them decide which values are valid, which is the tier-1
test.

So today, swapping `widget: select { choices: STATES }` to `widget: text`
silently drops the validation along with the menu — exactly the failure the
tier-1/tier-2 line exists to prevent.

Options: move `choices` to the field level (a breaking change to syntax that has
shipped once, in `examples/src/examples/select.rs`), or accept the split and
document that choices are presentation-only when the widget is not a chooser.
The first is more consistent; the second is cheaper.

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

**The macro can do these itself**, from tokens, at parse time (`expand` has no
error channel — `impl_form` only returns `Err` from parsing):

- `min_length > max_length`, `min > max` — unsatisfiable in any widget
- `pattern` parses under `regress` — same engine that runs it, so "compiles at
  build time" means "compiles at runtime"
- duplicate and unknown keys

**rustc does these**, via a `const {}` block in a generated generic fn that reads
`T::SHAPE` — verified working, including the bound-fits-the-type case:

- constraint vs. value kind (`max_length` on a `bool`)
- a bound that overflows the field's own type (`max: 1e50` on an `f32`) — the
  bound is emitted as a literal INSIDE the const block, so const eval sees both
  it and `T`; no const generics needed
- the validator's parameter type matching the field's, via a witness call

A shape-based check beats marker traits here: one source of truth with
`value_kind()`, no hand-kept impl list, no orphan problem, and it can say "can't
tell" for a newtype instead of rejecting it.
