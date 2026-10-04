---
name: css-class-vocabulary
description: "The fx- CSS class vocabulary, DECIDED with Todd 2026-09-29 and swept in as MVP step 1. Records every name, the four gaps the old set had (controls, containers, legends, the checkbox), why 'fx-control' is a deliberate exception to the widget-never-control rule, and why <option> needs no classes at all"
metadata:
  type: project
---

**Decided 2026-09-29**, as MVP step 1's class half ([[mvp-scope]]). The
*override* mechanism is issue #10 and deliberately deferred — `Formoxus` is
`#[non_exhaustive]`, so adding a `classes` field later is not breaking, while
renaming the defaults is. That asymmetry is the whole reason this happened now.

## The prefix is `fx-`

Not `formoxus-`. Todd's call, on the dioxus-`dx` analogy, and it is the right
one for a reason worth keeping: **the famous `fx` prefixes are not class
names.** JavaFX's `-fx-` is a CSS *property* vendor prefix (`-fx-background-color`,
designed by Oracle precisely so JavaFX and HTML styles can share a stylesheet),
and Angular flex-layout's `fxLayout`/`fxFlex` are *attribute* directives.
Neither can collide with a class selector. The one real class-emitting overlap
found was `css-fx-layout`, a small SCSS flexbox library.

Decisive local fact: `examples/assets/main.css` already defined `--fx-text`,
`--fx-muted`, `--fx-border`, `--fx-error`, `--fx-radius`. `fx` was ALREADY this
project's CSS abbreviation, so `formoxus-` classes would have been inconsistent
with the stylesheet that shipped beside them.

The argument against, recorded because it is real: for a library that ships no
stylesheet, **the class names ARE the public API**, and `fx-form-field` in
someone's markup a year later does not say what produced it. Nine characters
against self-documenting HTML. Todd took brevity.

## `fx-control` is a DELIBERATE exception to the vocabulary rule

CLAUDE.md bans the word "control" — and its stated reason is "nothing here
renders a bare element." This class names exactly the bare element, so the
reason does not apply, and **"form control" is the HTML spec's own term** for
`<input>`/`<select>`/`<textarea>`. The CSS vocabulary is web-facing, not
Rust-API-facing, so there is no `widget` for it to compete with.
**Do not read `fx-control` as the reversed widget→control rename creeping back**
([[widget-is-the-umbrella-word]]).

**REVISED 2026-10-03:** the original note said "do not let the word back into
the Rust API on its strength". That was stricter than the reasoning supports,
and Todd extended the carve-out by one type: **`FieldControl`** in
`formoxus-attrs`, which names the bare element too (see CLAUDE.md's vocabulary
section). The rule now reads: "control" names the bare HTML element and nothing
else; anything that renders a field is a widget.

## The five gaps the old set had

Found by asking Todd's own test — "are there classes where people will want to
style?" — and the answer was no in five places. These are ADDITIVE, so they were
not strictly forced into this window; they went in because the vocabulary
comment had to be rewritten anyway.

1. **No control carried a class at all.** Not `input`, `textarea`, `select`
   (twice), the checkbox input or the radio inputs. The element a Tailwind user
   most wants to hit was reachable only as `.form-field input` — the exact
   descendant-selector fallback the class set exists to avoid. Now
   `fx-control` + `fx-input`/`fx-textarea`/`fx-select`/`fx-checkbox`/`fx-radio`.
2. **No container carried a class.** All three `<fieldset>`s — nested struct
   (`field_set.rs`), enum (`variant_set.rs`), `Vec` (`list_set.rs`) — and all
   three of their `<legend>`s. `radio-group`'s fieldset was the ONLY classed one.
   Now `fx-group` + `fx-field-set`/`fx-variant-set`/`fx-list-set`.
3. **Four legends, not one.** Todd assumed `<legend>` was RadioGroup-only; it is
   also all three containers. Three of them caption a CONTAINER and one captions
   a FIELD, which is why giving the fieldsets classes was the enabling move: once
   `.fx-list-set > legend` can distinguish them, all four legends can safely
   share `fx-legend` + `fx-field-label` and "things that look like labels" style
   alike, which is what Todd wanted.
4. **No per-choice label class.** Each radio's wrapping `<label>` is now
   `fx-choice`. Confirmed 2026-09-29 as the right name for a
   `checkbox_multiple`'s boxes too, when that ships: each is one option among
   several for ONE field, the same role a radio plays. It also matches the Rust
   type — `Choice` is one option, `fx-choice` is its rendered wrapper. The edge
   stays coherent because a SINGLE checkbox on a `bool` field is not a choice at
   all, it is the field, which is why it keeps `fx-field-label`.
5. **`fx-invalid`** on the errored field wrapper. Only the control carried
   `aria-invalid`, so reaching the wrapper required `:has()`. This also RETIRED
   the sibling-selector requirement that `FieldErrors`'s placement rested on —
   `.fx-invalid .fx-field-errors` does not care about order, which is the only
   reason the checkbox could put its caption between the box and the list.

## The checkbox was structurally different, and is no longer

`Checkbox` rendered `label { "{text}" {input} }` against every other widget's
`label.form-field > span.field-label + control + errors`. So `.form-field` AND
`.field-label` both silently missed every checkbox. Fixed: the label gets the
wrapper class, the text gets its `span`, and the input moved AHEAD of the text
(box-then-text is the convention; it was text-then-box).

Todd's reason for the original shape is worth keeping: he was thinking ahead to
a `checkbox_multiple`, where each box is NOT a field of its own. That case is
real but unbuilt, so the inconsistency was not paying for anything.
**Still deliberately omitted: the required marker** — HTML `required` on a
checkbox means "must be ticked", which is issue #6, not a marker question.

## `<option>` gets NO classes — verified, not assumed

Styling `<option>` is barely possible: Chromium may honor `color`,
`background-color`, `font-family`, `font-variant`, `text-align`, Firefox
respects the owning select's `font-size`, everything else is ignored. So Django
(`---------`), Rails (`include_blank`/`prompt`) and Bootstrap all add none, and
the universal convention is attribute-based — which formoxus already emits:

| | selector |
|---|---|
| placeholder | `option[value=""][disabled]` |
| `--none--` | `option[value=""]:not([disabled])` |
| real choice | `option:not([value=""])` |

**Freebie nobody would guess:** because the placeholder carries `value=""` and
the `<select>` carries `required`, **`select:invalid` already matches a required
select in its placeholder state** — the "grey it out until answered" effect, with
no markup change. Worth stating in the vocabulary comment.

## Buttons

`ButtonType::default_class()`'s bare `primary`/`outline danger`/
`outline secondary`/`danger`/`secondary` are **deleted, not made configurable**.
They were styled by nothing in the shipped stylesheet, absent from its
"complete vocabulary" comment, correct only for Pico, and collide with a
consumer's own CSS. `formoxus-button-<type>` carries the semantic intent instead.

- `fx-button` + `fx-button-{submit,reset,cancel,destructive,action}`
- `add-row`/`remove-row` joined the scheme as `fx-button fx-button-add-row` /
  `fx-button-remove-row` — they are buttons OUTSIDE `.fx-buttons`, so the shared
  class is the only thing that reaches every button.
- `ButtonType::Button` → **`ButtonType::Action`**, AND the `form!` keyword
  `button` → **`action`** with it. Renaming only the variant left
  `type: button` expanding to a class spelled `fx-button-action` — a mapping you
  have to know rather than read, which Todd called magic. `fx-btn-*` was
  considered as a way to make `fx-btn-button` tolerable enough to keep the old
  name; rejected because it fixes the spelling and not the mapping, and because
  the row is `fx-buttons`, so it would have forced either `fx-buttons` beside
  `fx-btn` or an ugly `fx-btns`. Keyword, variant and class now all say
  `action`. The vocabulary comment in `button.rs` went from "three of these five
  are literally the HTML `type` attribute" to two, with a note saying why
  `action` deliberately is not HTML's word.
- `default_class()` → `classes()`: nothing overrides it and it returns two tokens.
- `formoxus-button-problems` → `fx-problems`; once `fx-button-<type>` is a
  pattern, the old name reads as "a button of type *problems*".

## Deferred to C6, not done here: `class+`

Todd's syntax idea for author-supplied attributes — `class: "x y"` REPLACES,
`class+ "x y"` APPENDS. Both are needed: a Tailwind user wants `fx-control`
gone, someone adding `data-*` wants it kept. Recorded with three findings:

- **Spelling.** Bare `class+ "x y"` breaks the `key: value` shape of that block;
  `class+: "x y"` or `class += "x y"` are the candidates.
- **It generalizes.** `aria-describedby` is a token list too — and issue #9 adds
  one, where appending is exactly what a consumer needs. So is `rel`, `accept`,
  `aria-labelledby`. If `+` means "join with a space" it is right for all of
  them; `style` needs `;` and is the lone exception (moot today, formoxus emits
  no inline styles).
- **Implementation constraint.** `Vec<Attribute>` has nowhere to record
  "append" — `ScalarWidget`'s merge is a name-keyed insert. The appended classes
  need their own home on `FieldSpec`, joined after resolution, NOT a slot in the
  attrs vec.

## Sweep traps

- **`radio-group` is also a ROUTE SLUG** (`/t/radio-group` in
  `examples/src/test_forms.rs` and three `e2e/tests/widgets.rs` tests). A blanket
  rename breaks the e2e suite.
- **`fx-` prefixing hides itself from `contains_substring`.** `fx-field-label`
  CONTAINS `field-label`, so assertions of the form
  `not(contains_substring("field-label"))` keep passing either way and prove
  nothing about the rename. The exact-HTML assertions are the ones that catch it.
- `"required"` appears as both the class and the HTML ATTRIBUTE, and several
  tests assert on the attribute.

## Left undone, deliberately

`VariantSelect` gained `fx-invalid` (derived from `errors.is_empty()`, since it
predates `FieldProps` and has no resolved `aria_invalid` to read) but still emits
**no `aria-invalid` at all** — so the errored state is reachable from CSS and
invisible to a screen reader. Filed as issue **#11**. Nearly unreachable in
practice: it needs a tampered submission or a refusal to pick a required variant.
