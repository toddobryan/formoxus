---
name: attribute-rules-design
description: "DECIDED by Todd 2026-10-02/03, nothing built yet: every field attribute in form! goes through ONE table of handling rules, in a third crate `formoxus-attrs` shared by formoxus and formoxus-macros; legality is per WIDGET; custom widgets become trait types declaring ATTRS (ANY allowed). Server checks stay. Absorbs C6 and issue #4"
metadata:
  type: project
---

**Todd's executive decision, 2026-10-02.** It reshapes C6 ([[mvp-scope]] step 3)
and takes in issue #4. Nothing is built yet beyond `FieldAttrs` +
`with_attrs` + `to_attributes` (steps 1–3 of the first C6 plan), which this
design replaces.

## The decision

Everything `form!` says about a field is about the FORM FIELD, not the model.
No constraint lives on the model. (If one ever does, say a `max_length` on a
model field, transferring it to the form is a separate, principled question.)
So there is no category difference between `max_length` and `placeholder` in
how they are DECLARED: both describe how the field renders.

Therefore: **one big list of the attributes formoxus handles, and every
attribute has rules for how it is handled and what it writes to the HTML.**
What used to be special cases (owned attributes, `class` merging, validity per
element) are just less typical rules. There is no second mechanism.

Todd accepts the consequence that **switching a field's widget can invalidate
attributes even when the value's type does not change** (`rows` is fine on a
textarea and wrong on an input).

**Server-side validation STAYS** (Todd, confirmed the same day: "Everything
that touches validation needs to make it to the server side so it can be
validated there, as well"). "Presentational" describes how attributes are
declared, NOT that the browser is the enforcer. The browser's copy of a rule
is bypassable, which is the whole reason #6 needed `ValueKind::check`. So
"is also enforced by `validate`" is ONE OF THE RULES, carried by the
attributes that need it.

This supersedes the "keep `Constraints` and `FieldAttrs` as separate structs"
recommendation of the same day. That recommendation's worry (a rule and a
hint looking like the same kind of thing) is answered by putting the
difference in each attribute's rules instead of in two types.

## What each attribute's rules have to say

| Rule | Examples |
|---|---|
| **Value type** (how `form!` parses and type-checks the value) | flag: `required`, `readonly`, `disabled`; integer: `maxlength`, `minlength`, `rows`, `cols`, `size`; bound typed to the FIELD: `min`, `max` (today's `Bound` + the `bound_in_range`/`bound_is_whole`/`bound_is_exact` checks); regex checked at compile time: `pattern` (must stay a `LitStr`, because `regress` runs in the macro); text: `placeholder`, `title`; token list: `class`, `autocomplete`; declaration list: `style` |
| **Ownership** | OWNED, so refused at compile time: `name`, `type`, `value`, `checked` (Todd's list), plus probably `form`, `multiple`, `selected`, `aria-invalid`, and the presence `required`; AUTHOR-SET: most; MERGED: `class` (replace or append), `style` |
| **Where it is valid** | per element, and per `type` for `<input>`; the data is in [[html-attributes-reference]]. A `Custom` widget's element is unknown, so let it through |
| **Which value kinds it applies to** | `min`/`max` only on numbers, `maxlength`/`pattern` only on text, `required` only on a non-optional bool. Today's `field_kind::takes_bound`/`takes_length`/`takes_required`/`required_is_not_optional` gates |
| **Server check, if any** | `minlength`, `maxlength`, `pattern`, `min`, `max`, `required` (must be true). `placeholder`, `rows`, `class` etc. have none |
| **Patterns, not entries** | `data-*` and `aria-*` as prefix rules; `aria-invalid` stays owned (resolved at the `Form` boundary, see [[shrink-fieldprops-idea]]) |

### Rules that need care, already known

- **`pattern` on a `<textarea>` is REFUSED (Todd, 2026-10-02).** HTML has no
  `pattern` on `<textarea>`, and today formoxus validates one anyway
  server-side while emitting it to the DOM, where it is ignored. Rather than
  invent an "enforced but not emitted" rule for that one case, `pattern` is
  simply not valid on a textarea: a compile error like any other attribute on
  the wrong element. A regex over multi-line text can be written as a per-field
  validator (C4) if anyone needs it; revisit only if it turns out to be useful.
  CONSEQUENCE: `tests/suite/constraint_attrs.rs::a_textarea_gets_pattern_
  although_html_has_no_such_attribute` pins the old behaviour and becomes a
  trybuild golden instead when the table lands. With this gone, no attribute
  is yet known to need "server check but no HTML": every rule that is checked
  is also emitted where valid. Keep the two as separate rule fields anyway,
  since the table should not assume it.
- **`pattern` arrives unanchored.** HTML wraps it as `^(?:…)$`, `check` wraps
  it the same way. Never anchor it at emit time ([[constraint-attributes-gap]]).
- **Date types take `min`/`max`/`step` in their own value format**
  (`2026-10-02`), not `i128`/`f64`. That is a value-type rule that depends on the
  input type, and it is out of MVP scope (dates are) but the table must not
  make it impossible.
- **`step`:** formoxus deliberately emits none for a float
  ([[constraint-attributes-gap]]). As an author attribute it is fine.
- **`required` has two senses.** The PRESENCE `required` that formoxus emits for
  a non-optional field is owned. `required: true` on a bool is the #6 rule:
  must be true, server-checked, and emitted as HTML `required` only on a
  checkbox.

## Where the table lives: a third crate, `formoxus-attrs`

**DECIDED by Todd 2026-10-03** (superseding "as `const` data in `formoxus`"):
the attribute facts go in a third crate, **`formoxus-attrs`**, that BOTH
`formoxus` and `formoxus-macros` depend on.

Why a third crate: `formoxus-macros` cannot depend on `formoxus`, because
`formoxus` depends on it (to re-export the macros). That is why the macro
today can only reach type facts through generated `const _` asserts calling
`formoxus::field_kind`. A shared crate is reachable from both sides:

| Who | Uses the table for |
|---|---|
| `formoxus-macros`, at EXPANSION time | each attribute's VALUE TYPE, so it knows to parse `pattern` as a `LitStr` and `min` as an `Expr` (this ANSWERS old open question 5); and element validity, so "`rows` is not valid on this widget" is reported by the macro itself with a hand-written message and an exact span |
| `formoxus`, at RUN time | driving `check` (server side) and attribute emission |

**The split of checks this produces.** The macro sees widget NAMES but not the
field's TYPE (that is in the consumer's crate). So:
- widget/element validity → checked IN THE MACRO, from `formoxus-attrs`;
- value-kind checks (`min` only on numbers, `required` only on a non-optional
  bool, bound fits the type) → stay generated free `const _` asserts against
  the field's `Shape` ([[const-shape-walk-blocked]]);
- a CUSTOM widget's declared attributes → generated `const _` asserts too,
  since only the compiler, not the macro, can see another crate's declaration
  (below).

**Constraints on the crate:** tiny and DEPENDENCY-FREE (no Dioxus, no syn).
It is compiled twice, for the macro on the host and for the app including
wasm. Plain `const` data and `const fn`s. Three crates now publish and
version in lockstep.

`Constraints`, `FieldAttrs`, and `ValueKind`'s constraint fields still collapse
into ONE runtime representation, and `ValueKind::check` / `ValueKind::attrs`
iterate the table's rules instead of matching hand-written fields.

## Legality is a fact about WIDGETS (more exactly, the element they render)

**Todd, 2026-10-03.** Which attributes are legal is not a property of the
attribute alone but of the element a widget renders (`<textarea>`, `<select>`,
`<input type=…>`, or `RadioGroup`'s `<fieldset>`). A widget's allowed set =
what its element accepts per the spec ([[html-attributes-reference]]) minus
what the widget OWNS (`name`, `type`, `value`, `checked`, …). For built-in
widgets this is derived from the element facts.

## Custom widgets declare their attributes

**DECIDED by Todd 2026-10-03.**

- **A custom widget becomes a TYPE implementing a formoxus trait**, not a bare
  Dioxus component function (today `widget: custom(MarkdownWidget)` names a
  component fn, which has nowhere to hang a declaration). Roughly:
  `trait Widget { const ATTRS: AttrSet; fn render(props: WidgetProps) -> Element; }`.
  BREAKING, accepted (pre-publish). The orphan rule is no obstacle: the author
  implements formoxus's trait on their OWN type.
- **`form!` checks attributes against it at compile time** by emitting
  `const _: () = assert!(supports(<MarkdownWidget as Widget>::ATTRS, "rows"), …)`.
  The compiler evaluates it in the consumer's crate, where the impl is visible.
  The macro cannot do this check itself.
- **`AttrSet::ANY` is allowed** as an explicit opt-out, for a wrapper that
  forwards everything to one element. It is still a deliberate statement, so
  "custom widget authors must state what they support" holds.
- **Custom widgets must start RECEIVING attributes.** Today they get `values`
  and `props` only (see the `WidgetKind::Custom` arm in
  `formoxus-macros/src/form/widget.rs`, and "a custom widget supplies its own
  choices"). So `WidgetProps` has to carry the attributes. Todd: "we need to
  fix that."

## Build order (agreed 2026-10-03)

C6 grew from "mostly `form!` grammar" into this design, so it lands as a
sequence of commits, each independently green:

1. **Create `formoxus-attrs`** with the element facts (from
   [[html-attributes-reference]]) and the attribute table. Nothing uses it yet.
2. **Move the existing constraints onto it, with NO behaviour change**
   (`min_length`, `max_length`, `pattern`, `min`, `max`, `required`). The
   current tests are the regression guard: they should pass unchanged, except
   any that pinned a representation rather than a behaviour.
3. **Author attributes and the ownership rules** (refuse `name`, `type`,
   `value`, `checked`, …; `class` replace/append as a list; `style`).
4. **Element validity, then the custom-widget trait last** (`Widget` with
   `ATTRS`, `AttrSet::ANY`, custom widgets receiving attributes). Validity
   brings the `pattern`-on-textarea refusal; the suite test
   `a_textarea_gets_pattern_although_html_has_no_such_attribute` becomes a
   trybuild golden here.

The `FieldAttrs` + `with_attrs` + `to_attributes` already built (2026-10-02)
is the runtime half of step 3 in its first shape, and its tests in
`tests/suite/author_attrs.rs` carry over as behaviour guards.

## Findings this has to respect (from 2026-10-02 probes)

- **An author attribute a widget also sets is DUPLICATED, not replaced**
  (`name="name" … name="hijack"`). `ScalarWidget`'s merge dedupes only within
  the spread. In served HTML the first copy wins; on the client the last likely
  does (unverified). This is why ownership must be enforced and why `class` must
  be resolved into the widget's ONE `class` value, never spread. Details in
  [[html-attributes-reference]].
- **A spread attribute overrides a constraint attribute of the same name**
  (`maxlength="99"` replaced `max_length: 10`'s), so the browser and the server
  disagreed. Pinned by `an_author_attribute_overrides_a_constraint_attribute` in
  `tests/suite/author_attrs.rs`. With one table there is only one source for
  `maxlength`, so this becomes impossible to write.
- **`RadioGroup` spreads onto its `<fieldset>`; a `hidden` input has no spread.**
  Both pinned in `tests/suite/author_attrs.rs`.
- **Author attribute values render as text** (`rows="4"`), constraint values as
  numbers (`maxlength=10`). Once both come from one table, pick one per value type.

## Open questions

1. Are `form` and `multiple` owned? (`form` moves the control into another
   form; `multiple` breaks the one-string leaf.) Probably yes.
2. How is a `style` list spelled in `form!`, and how is a `class` list?
   (Todd: lists, not space- or semicolon-delimited strings.)
3. `class` on a `RadioGroup`: the fieldset, every radio, or refused?
4. `id`: reserved until issue #9 settles its id scheme (`<label for>`, per-form
   prefix); see [[html-attributes-reference]] item 4.
5. ~~The macro side's knowledge of value types: mirrored list or loose parse?~~
   ANSWERED 2026-10-03: neither; the macro reads `formoxus-attrs` directly.
6. Emission when an attribute is invalid on the element but the author set it
   anyway: compile error for built-in widgets (Todd's intent), let through for
   `Custom`.
7. Does `disabled` get a rule about submission? A disabled control is not
   submitted, so a non-optional field reports "This field is required."
