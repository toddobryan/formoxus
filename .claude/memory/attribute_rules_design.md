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
   **Crate CREATED 2026-10-03, empty** (crate doc only): workspace member and
   default member, `workspace.dependencies` entry with path + version (same
   publish rule as `formoxus-macros`), depended on by both `formoxus` and
   `formoxus-macros`, README and licenses symlinked to the root, added to
   `just msrv` and CLAUDE.md's layout. Verified: workspace tests/clippy/rustdoc,
   `cargo +1.90 check`, the wasm example build, and `cargo package --list`.
   The table itself is Todd's to write.
   **`InputType` MOVED into it 2026-10-03** (`formoxus-attrs/src/input_types.rs`),
   now `Copy, Eq, Hash` with a `const fn html_type(self)`, re-exported from
   `formoxus::widgets::types` so `formoxus::widgets::InputType` is unchanged
   for consumers and for the macro's generated paths. `WidgetType` cannot move
   (`Custom` holds a fn returning a Dioxus `Element`); a new element enum
   (`Input(InputType)`, `Textarea`, `Select`, `Fieldset`) is what should join
   `InputType` there, with `WidgetType` mapping onto it. **Named
   `FieldControl` (Todd, 2026-10-03)**; see [[widget-is-the-umbrella-word]] for
   why "control" is allowed for it. The validity table is a `const fn match`
   generated by a `macro_rules!` table (like `field_body!`/`widgets!`), so the
   attribute must be an ENUM: a `const fn` cannot `match` on `&str` (checked:
   "cannot match on `str` in constant functions").
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

## TODO: cross-check the table against dioxus-html (Todd, 2026-10-03)

**Write a test in `formoxus` (NOT in `formoxus-attrs`)** asserting that every
attribute the table allows on an element has a matching constant in Dioxus's
own list, e.g. `dioxus::html::elements::input::placeholder`. Dioxus's lists are
in `packages/html/src/elements.rs` (`builder_constructors!`) and
`attribute_groups.rs` (globals) at tag `v0.7.10`; each attribute is a
`pub const name: AttributeDescription = (name, namespace, volatile)`, and
`rsx!` checks an unquoted name by referring to that const. Dioxus checks names
only (the type column is discarded), with one flat list per element and no
per-`<input type>` rules.

Purpose: each gap is either an **exception encoded in the test** or **an
upstream issue/PR asking Dioxus to add it**. Known gaps already (see
[[html-attributes-reference]]): `alpha`, `colorspace`, `dirname` on `<input>`;
`dirname` on `<textarea>`; `command`, `commandfor` on `<button>`; globals
`headingoffset`, `headingreset`, `writingsuggestions`.

**Why `dioxus-html` cannot be the source or a dependency of `formoxus-attrs`:**
checked 2026-10-03, it is not a definitions-only crate. Its direct deps include
`dioxus-core`, `dioxus-core-macro`, `dioxus-hooks`, `futures-util`, `euclid`,
`keyboard-types`, `tracing` and more, 87 crates in its normal tree. That would
break `formoxus-attrs`'s no-dependency rule and drag Dioxus into the macro's
build. `formoxus` already reaches it free, through `dioxus::html` (the `html`
feature is on by default via `default` → `lib` → `html`), so the test costs
nothing there.

## Decisions after the 2026-10-03 snags

Entering the spec facts literally hit two snags: `min`/`max` are not valid on
`type="text"`, which is how formoxus renders numbers by default, and
`required` is one HTML attribute with two senses. Todd, after a break,
2026-10-03:

1. **Two columns, `on:` and `for:`.** `on:` is the `FieldControl`s an
   attribute may be EMITTED on (straight from the spec); `for:` is the value
   families of FIELD it applies to (`min`/`max` → `Int | Float`). The rule
   joining them:
   - `for:` does not match the field → compile error;
   - matches, and `on:` matches the control → accepted, emitted, and
     server-checked if `check: true`;
   - matches, `on:` does NOT match, `check: true` → accepted and
     server-checked, NOT emitted (`min` on a number rendered as text: today's
     behaviour, kept);
   - matches, `on:` does not match, `check: false` → compile error (nothing
     would emit or enforce it, e.g. `rows` on an `<input>`).
   `for:` gets its own `applies_to(self, ValueFamily)` with its own glob
   import, because `ValueFamily::Text` and `InputType::Text` would collide. (Renamed: `ValueFamily` is now `FieldType`; see 8.)
2. **Bare flags in `form!`** (`agreed => { required_true }`, `readonly`),
   parsed by `form!`'s hand-written parser. This REPLACES `required: true`
   shipped in #6. `required` splits into two rows: `Required` (presence, owner
   `Formoxus`, `on:` excludes the checkbox) and `RequiredTrue` (owner
   `Author`, `for: Bool`, `on:` the checkbox only). Both emit HTML `required`.
3. **The `form!` key is the variant name in snake_case**, converted by
   `form!` (it already has `heck`): `max_length` ↔ `MaxLength`,
   `required_true` ↔ `RequiredTrue`. No `key:` column; `name:` stays the HTML
   name. The table generates `variant_name()` (`stringify!`); `form!` looks
   the key up AT EXPANSION, so a typo gets "unknown key", a did-you-mean
   suggestion and the legal list, never "no variant in enum `Attr`".
4. **Owned rows are precise**, not `on: _`, so that un-owning one later says
   exactly where it is valid. `checked` is owned like `name` and `value`.
5. **`Input` becomes the router for every `<input>`** (Todd): it matches on
   its input type and renders `Checkbox` for a checkbox, the text code for
   the text-like types, and so on. `WidgetType::Checkbox` goes away
   (`form!`'s `checkbox` becomes `Input(Checkbox)`), and `RadioGroup` stays
   the group widget.
6. **Two enums for input types.** `InputType` in `formoxus-attrs` mirrors ALL
   22 HTML input types (facts); `WidgetType::Input` takes a SECOND, smaller
   enum holding only the types formoxus renders, so `Input(Radio)` or
   `Input(Submit)` cannot be written by anyone. `Input`'s match is then
   exhaustive with no panic arm, and a one-line conversion maps the small
   enum onto `InputType` for table lookups. `FieldControl` then needs no
   separate `Checkbox`/`Radio` variants: `checked`'s `on:` is
   `Input(Checkbox | Radio)`.
7. **`form!` is the only supported way to build a `FormSpec` outside the
   crate.** The builders, `FormField`'s fields and `ScalarWidget` move behind
   `#[doc(hidden)] pub mod __private`. Todd: "If someone wants to dig into API
   they shouldn't be using, screw 'em." Integration tests may still use
   `__private` (it is `pub` to the compiler). About 210 builder calls in 15
   suite files plus `widget_matrix` use the builders today; converting them is
   churn, so this comes LAST, after the table and the `FieldProps` reshaping.
   Runtime guards that only a hand-built spec could reach (the fractional
   bound panic, the `maxlength` override test) can then be deleted.

8. **Renames (Todd, 2026-10-03), to untangle the `Value…` names from the
   `value` attribute:** `ValueType` → **`AttrType`** (the type of value an
   ATTRIBUTE takes: `Text`, `Int`, `Bound`, `Regex`, `Flag`, …), and the
   proposed `ValueFamily` → **`FieldType`** (the COARSE type of data a FIELD
   holds: `Text`, `Int`, `Float`, `Bool`; `u8` and `i64` are both `Int`, which
   its doc comment must say). Table columns: `value:` → **`type:`**, and the
   field-type column is **`for:`** (`Min { type: Bound, for: Int | Float }`).
   Verified: keywords work in `macro_rules!` both as literal matcher tokens and
   even as metavariable names (`$type`, `$for`; only `$crate` is reserved).
   Consequence for step 2: `formoxus`'s `ValueKind` loses its constraint
   payloads to the table and should be REPLACED by `FieldType`, not kept as a
   second name for it. `FieldValue`, `ValuesStore` and `ValuesByPath` stay.

9. **`placeholder` is `for: Text | Int | Float`, not `Bool`** (Todd,
   2026-10-03): a bool renders as a checkbox, a `select` or a radio group, and
   `placeholder` is valid on none of them, so `Bool` could only ever produce the
   less helpful "not valid on this control" error. A later idea, letting
   `placeholder:` set a select's "Choose..." text, is recorded in
   [[emitted-classes-and-strings]].

10. **The `validated:` column, BUILT 2026-10-03** (replacing `check: bool`,
    and superseding the `server_only:`, `Check` enum and `BrowserAndFormoxus`
    designs discussed the same day). Column order is now
    `name, owner, type, for, on, validated`. Values: `false`; `true` (formoxus
    validates it, and the browser too wherever it is emitted); or
    `true_and_on(<FieldControl pattern>)`, which is `true` PLUS controls where
    it is accepted and validated by formoxus but NOT emitted. Rows:
    `Min`/`Max` are `true_and_on(Input(Text))` (a number rendered as text);
    `RequiredTrue` is `true_and_on(Select | Input(Radio))`; `Pattern` is plain
    `true`, so a textarea is REFUSED. Generated: `validated() -> bool`
    (replaces `checked_by_server`) and `is_also_validated_on(FieldControl)`. A
    helper `validated_flag!` accepts exactly the three spellings and gives
    "`validated:` takes `false`, `true` or `true_and_on(...)`, not `no`"
    otherwise. Named so nothing says WHERE the check runs, which stays true
    once blur validation runs the same Rust in the browser.
    **`Attr::is_allowed(field_type, control)`** (hand-written, not generated)
    is the acceptance rule: `applies_to(field_type) && (is_valid_on(control)
    || is_also_validated_on(control))`. Ownership is a separate check.
    Tests (21 in `attrs.rs`) check every row against the spec's keyword lists
    for all 22 input types, the non-input controls, all field types, the
    `true_and_on` controls (and that none is also in `on:`), and the rule,
    including `min` on text allowed vs `pattern` on textarea refused.

11. **Step 2d's `check` iterates attributes, not field types** (Todd,
    2026-10-04): for each attribute in `Attr::ALL` order that the field has and
    that `validated()`, run that attribute's check; the field type is an INPUT
    (it picks `i128` vs `f64` for `Min`/`Max` and drives the bound widening),
    not the dispatch. **`Min`/`Max` keep the COMBINED "in the range X up to
    (and including) Y" message**: the pair is handled once, on `Min`, with
    `Max` skipping itself when `Min` is present. A guard skips any attribute
    where `!applies_to(field_type)`, keeping today's silent-ignore for a
    hand-built spec until decision 7 makes it unreachable. `Required`
    (presence) is not checked in this loop; it stays in `validate`'s `Empty`
    handling.

12. **STEP 2 DONE (2026-10-04), no behaviour change.** `formoxus::attrs`
    re-exports the crate; `field_kind`'s `takes_length`/`takes_pattern`/
    `takes_bound`/`takes_required` ask the table through `applies(attr, shape)`
    (`Unknown` → let through, `Other` → refuse); `Constraints` is replaced by
    `AllAttrs(IndexMap<Attr, AttrValue>)` (in `formoxus`; `AttrValue` and
    `Bound` moved to `formoxus-attrs`); `ValueKind` is gone, replaced by
    `FieldType`. `check` and `constraint_attributes` are methods on `FormField`
    (NOT `FieldType`: a crate cannot add methods to another crate's type, and
    both need `int_bound` with the field's name). Both walk `Attr::ALL`, so the
    TABLE'S ROW ORDER is the message and attribute order (`MinLength` was moved
    before `MaxLength` for that). `ScalarWidget` takes `field_type`,
    `constraint_attrs` and `required_true`. Verified: 544 tests, all goldens
    unchanged (one added: `form_pattern_on_a_number`), 29 e2e, clippy, rustdoc,
    1.90, wasm. `AllAttrs` (constraints) and `FieldAttrs` (author attributes)
    are still separate; merging them, and picking the surviving name, is step 3.

### Agreed to come AFTER the table holds today's attributes

**Reshaping `FieldProps`** (Todd's earlier shrink idea, [[shrink-fieldprops-idea]]):
`required` and `aria_invalid` become attributes, leaving `path`, `label` and
`errors`. The table answers the objections that made them props: ownership
stops a consumer overriding `aria-invalid`, and `RadioGroup` can route each
attribute to the radios or the fieldset by asking `is_valid_on`, which also
settles the open `RadioGroup` question. **DECIDED (Todd, 2026-10-03): attributes
stay TYPED (a map keyed by `Attr`) until emission**, so a widget that must
reason about one (`Select`'s placeholder, the ` *` marker) asks
`contains(Attr::Required)` instead of searching strings; only at the last
moment does the map become the `Vec<Attribute>` that is spread.

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
