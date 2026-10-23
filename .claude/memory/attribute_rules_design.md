---
name: attribute-rules-design
description: "DECIDED by Todd 2026-10-02/03, nothing built yet: every field attribute in form! goes through ONE table of handling rules, in a third crate `formoxus-attrs` shared by formoxus and formoxus-macros; legality is per WIDGET; custom widgets become trait types declaring ATTRS (ANY allowed). Server checks stay. Absorbs C6 and issue #4"
metadata:
  type: project
---

**The step-by-step build order, with files and line numbers, is
`formoxus/ATTRIBUTES_PLAN.md`** (created 2026-10-04; Todd asks for updates
as things change). This note holds the decisions and reasoning.

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
- **The trait also makes a widget declare its default classes** (Todd,
  2026-10-08). Required, with no default, so every widget author has to state
  them, the same as `ATTRS`. **How is left OPEN:** Todd leans toward a required
  FUNCTION rather than a `const CLASS: &'static str`; a function also leaves
  room for (d) below. What it buys:
  - (a) `class_plus` gets a base to append to. Formoxus resolves `Class`
    (replace) and `ClassPlus` (append) BEFORE calling the widget, which
    receives one finished `class` in its attributes: the same rule as
    `FieldAttrs::class(base)` for the built-ins (ATTRIBUTES_PLAN.md 3d, step 4).
  - (b) Built-ins can implement the same trait, so the literal base strings
    step 4 writes into each widget move to one place per widget.
  - Open (c): it names the class of the element the attributes land on, not
    the wrapper (`fx-form-field`). `RadioGroup` (a group AND several inputs)
    may need a second, optional declaration; settle together with the
    `RadioGroup` class-placement hold (`group_class`/`input_class`).
  - Open (d): issue #10 (configurable class names). Whatever the widget
    declares is then a DEFAULT, overridden from `Formoxus` config at the
    `Form` boundary ([[config-cascade]]); document it as a default, not the
    final class.

## DECIDED 2026-10-10 (Todd): widgets have PARTS, named by their elements

> **SUPERSEDED IN PART 2026-10-18 (Todd, FIELDS_PLAN.md W-A).** Parts survive
> only as CLASS SLOTS (`Classes` keyed by widget and slot). Addressing
> attributes to a part (decisions 3–5 below, and open point A) is DROPPED,
> never having been built: a widget is a struct of props with `EXTENDS:
> AttrSet`, and its `render` decides where every attribute goes. Per-element
> classes per field become props (`RadioGroup`'s `group_class`/`input_class`).
> **Those props were DROPPED 2026-10-20 (Todd)**, settling the class-placement
> hold: a field's `class`/`class_plus` go on the radio group's `<fieldset>`
> (where `EXTENDS` and the spread already pointed), inner slots are restyled
> per app or form through `Classes`/`classes_plus:`, and per field by CSS from
> the fieldset's class. Nothing addresses one inner element for one field.

Todd floated it 2026-10-09: treat `RadioGroup` as the first multi-element
widget (cf. Django's `MultiWidget`). Claude wrote it up 2026-10-10; Todd
ADOPTED it the same day, with three decisions of his own (below). Points still
open are at the end. It replaces Step 4's "one `FieldControl` per field" in
`formoxus/ATTRIBUTES_PLAN.md` (D0 there).

### The problem: one fact, patched four times

Step 4 as first planned gave each field ONE `FieldControl`, the element its
attributes land on. `RadioGroup` has several kinds of element, and each place
that assumption failed was patched on its own:

1. **D1.** `required_true` is `true_and_on(Select | Input(Radio))`, but the
   spread lands on the `Fieldset`, so 4b would refuse a required-true radio
   group over bools.
2. **`FieldAttrs::without(Attr::Required)`** in `radio_group.rs` hand-routes
   one attribute off the fieldset and onto the radios.
3. **An author `class` goes nowhere** on a radio group
   (`on_a_radio_group_an_author_class_goes_nowhere_yet` in
   `tests/suite/author_attrs.rs`); open question 3 below; the
   `group_class`/`input_class` hold; the TODO at the top of `RadioGroup`.
4. **The trait's default class, open (c)** above: a radio group has more than
   one base class (`fx-radio-group`, `fx-control fx-radio`, `fx-choice`).

They are all the same fact: a widget can have more than one element that
takes field attributes.

### What was decided (Todd, 2026-10-10)

1. **Adopted.** A widget declares its parts; an author can address one.
2. **A part is named by its HTML ELEMENT** (Todd): "If the docs say which
   elements a widget expands to, it's easy for users to know where attributes
   would end up." The docs show each widget's element tree, and the part names
   are read straight off it. **Every widget has a `label` part** (the
   wrapper), not only the radio group.
3. **The part block goes inside the widget's braces**, next to `choices`:
   `widget: radio_group { choices: STATES, input: { class: [inline] } }`.
   Field-level keys stay the attribute namespace (~95 keys).
4. **A validated key may NOT be addressed to a part**: refused. Todd:
   "Validation is a property of the widget as a whole." Message along the
   lines of "`max_length` goes on the field, not inside `input: {…}`".
5. **Which part gets an unaddressed attribute is a CONVENTION over the
   element names**, not something each widget marks. (The convention itself
   is open point A below.)

### The parts, read off today's markup

| widget                          | element tree (part names)                                   |
|---------------------------------|-------------------------------------------------------------|
| `Input` (text-like, `number`, …) | `label` > (label text, `input`, errors)                     |
| `Textarea`                      | `label` > (label text, `textarea`, errors)                  |
| `Select`, the variant select    | `label` > (label text, `select` > options, errors)          |
| `Checkbox`                      | `label` > (`input`, label text, errors)                     |
| `hidden`                        | `input` only                                                |
| `RadioGroup`                    | `fieldset` > (`legend`, `label` > `input` per choice, errors) |

In the single-element widgets the `<label>` IS the field wrapper: it carries
`fx-form-field`, and holds the label text (a `<span class="fx-field-label">`),
the control and the errors. So the `label` part is the wrapper, and a `class`
there replaces `fx-form-field` the same way one on the control replaces
`fx-control`. This is the "wrapper part" the first write-up kept out; Todd's
decision 2 brings it in, and it now overlaps issue #10 (configurable class
names), which has to treat the declared classes as defaults.

**Not parts:** the label-text `<span>` and the required-marker `<span>`
(the same element name in two roles, so the name cannot address one), the
`<option>`s, and the error list. The working rule: a part is an element name
that has exactly one role in its widget.

### Where each attribute goes

| attribute                         | lands on                                    |
|-----------------------------------|---------------------------------------------|
| validated (`validated()`)         | the form control(s): `input`/`textarea`/`select`; never addressable (decision 4) |
| addressed to a part               | that part                                   |
| not validated, unaddressed        | open point A                                |
| formoxus's own (`aria-invalid`)   | wherever the widget puts it (unchanged: on the group, per ARIA 1.2) |

**The checks are 4b's assert unchanged**, `Attr::is_allowed(field_type,
element)`; only where the element comes from changes (the part the attribute
routes to). Each part's `class`/`class_plus` resolves against that part's own
base class, which answers open (c) above.

### What it does to Step 4

1. **D1 dissolves.** `required_true` and `required` route to the radios
   (`Input(Radio)`), which the table allows, so `without(Attr::Required)`
   goes. The legend's ` *` marker still asks `contains(Attr::Required)` of the
   whole `FieldAttrs`, before it is split.
2. **D3 is the same decision in a new place.** `max_length` on a radio group
   is checked against `Input(Radio)` instead of `Fieldset`; `maxlength` is
   valid on neither, so it is still refused unless D3 adds a `true_and_on`.
3. **D4's `AttrSet` belongs to a part**, and so does the default class. The
   trait declares `PARTS` instead of `ATTRS`.
4. **4a: every widget gets its parts**, from the `widgets!` table for a named
   widget. `default_control` (what the form control is when there is no
   `widget:`) is still needed for the validated-key check. To address a part,
   a field must name its widget: `widget: text { label: {…} }`, so
   single-element widgets start taking braces.
5. **4b: the assert picks its element by the routing rule**, and gains "no
   part `x` on `radio_group`": at expansion for a built-in, and for a custom
   widget by a const lookup in its `PARTS`.
6. **4c: `to_attributes(control)` becomes `for_part(part)`**, which routes and
   then filters. Every widget calls it once per part.
7. **4d: `const PARTS`**, each carrying its `AttrSet` and default class.
8. **The built-in widgets' DESCRIPTIONS move into the shared crate, which is
   renamed `formoxus-attrs` → `formoxus-spec`** (Todd, 2026-10-10; first step
   of 4a, done now). A dependency-free `BuiltinWidget` enum holds, per widget:
   its `form!` name, its parts and which is the form control, whether it takes
   `choices`, whether it accepts an optional field, which `FieldType`s it
   renders, and the default widget per field type. That retires today's
   copies: `widgets!` mapping names to variant STRINGS, `rule()` and
   `field_kind::WidgetClass` mirroring `ScalarWidget`'s arms, `CHOOSERS`, the
   optional-field refusals, and the planned `default_control`. Decision 6's
   smaller "input types formoxus renders" enum lives beside it (Todd: yes).
   Staying in `formoxus`: the components, `ScalarWidget`'s dispatch (its match
   becomes exhaustive over `BuiltinWidget`), `WidgetType::Custom` (a component
   fn pointer; `WidgetType` becomes roughly `Builtin(BuiltinWidget) |
   Custom {…}`), shape → `FieldType` (needs facet), and classes
   ([[emitted-classes-and-strings]], #10).

### Still kept out

1. **Value decomposition**, `MultiWidget`'s actual job: one value spread over
   several inputs with their own names (a date as day/month/year), with
   Django's `decompress`. Parts never change what submits.

   **Revisited 2026-10-15 (Todd raised it; still kept out of Step 4, but
   Step 4 must not rule it out).** Todd's motivating case was a
   `QuestionWidget` for a multiple-choice question, as a teacher writes it
   and as a student answers it. Worked through, neither half is a
   MultiWidget:
   - *Teacher authoring* is a struct (`prompt`, `choices: Vec<String>`,
     `correct`), which reflection already renders as a `FieldSet` holding a
     `ListSet`. The gap is custom LAYOUT for a composite member (e.g. a radio
     beside each choice to mark the correct one), i.e. a custom renderer for
     a struct/list member, plausibly the same trait as custom widgets.
   - *Student answering* submits ONE value. The question and options are
     display data: a `RadioGroup` with run-time choices
     (`FormSpec::with_choices` already takes any iterator) plus somewhere to
     show the prompt.
   The real MultiWidget cases are one value over several form controls:
   split date and time (Django's `SplitDateTimeWidget`), date as three
   selects (`SelectDateWidget`), money as amount + currency, duration as
   h/m/s, a min/max range, lat/long.
   **Attributes are not the hard part:** D0's parts already allow N form
   controls (`RadioGroup` has one `<input>` per choice), so a multi-part
   widget is the same per-part addressing. What Step 4 should do is settle
   A below as a per-widget PRIMARY part that unaddressed keys go to, which a
   multi-control widget can name too. **Values are the hard part:**
   `ValuesStore` is one raw string per path, and the server matches a
   submitted `name` to that path, so a multi-part value needs sub-paths
   (`when.date`, `when.time`, combined at parse) or one packed string:
   Django's `decompress`/`value_from_datadict` pair. That touches
   `ValuesStore`, `distribute_form_values` and the wire format.
   Plan agreed: build it only for a real consumer, starting from ONE
   concrete widget (split date-time or money), not a general framework.
   Todd "having trouble thinking of examples" is the signal to wait.
2. **Per-choice attributes** ("this one radio is disabled"). They belong with
   `Choice` ([[chooser-ideas]] already has per-choice hint text); a part's
   attributes go on every instance.

### Open, for Todd (raised 2026-10-10)

**B and C mostly dissolved 2026-10-10** by un-nesting the labels (see the
next section). A, D and E stand.

Todd could not decide these in the abstract ("having trouble seeing that far
into the future"), so they are worked through as `form!` → HTML examples in
`formoxus/WIDGET_PARTS_EXAMPLES.md`, which also adds E (custom widgets with
two of the same element) and doubles as raw material for the docs.

- **A. The convention for an unaddressed, unvalidated attribute.** "The
  outermost part" does not work now that every widget has a `label` part: it
  would move `placeholder`, `rows` and `autocomplete` onto the `<label>`
  wrapper, where they are invalid and would be refused. Claude's proposal:
  **the element that stands for the field**: its form control when it has
  one, the `<fieldset>` when it has several. That is exactly today's behaviour
  for every widget. The alternative, always the form control, sends a radio
  group's `class`, `title` and `data-*` to every radio, and flips
  `on_a_radio_group_author_attributes_land_on_the_fieldset`.
- **B. `label` means different things in a radio group.** Read off the
  elements, a radio group's `label` part is each CHOICE's `<label>`; the
  group's own label is `legend`. Honest to the markup, but someone styling
  "the label" of a text field and of a radio group writes two different keys.
- **C. `label` one brace level apart.** Field level, `label: "Name"` is the
  label TEXT; inside the widget's braces, `label: { class: […] }` is the label
  ELEMENT. No parse clash, but two meanings.
- **D. The table has no word for `<label>` or `<legend>`.** Validity is keyed
  on `FieldControl`, which is `Input(_)`, `Textarea`, `Select` and `Fieldset`.
  A `label` part needs `Label` and `Legend` added, and every global row
  (`class`, `title`, …, written `on: Input(_) | Textarea | Select | Fieldset`)
  gains them, or a `Global` shorthand. And `FieldControl` then names
  non-controls, which the CLAUDE.md exception ("control" names the bare form
  element, nothing else) does not cover; `Fieldset` was already called "a
  stretch". Rename it (`Element`?), or live with it?

## Labels beside their inputs, and configurable classes (DECIDED 2026-10-10)

Todd pulled issues #9 and #10 ahead of Step 4, because Step 4's parts are read
off the markup and both change it. Build order: P1 → P2 → P3 in
`formoxus/ATTRIBUTES_PLAN.md`, "Before Step 4", which also lists what is
what Todd decided for each (P1-a/b/c, P3-a/b, all 2026-10-10).
**Ids: a RANDOM form id + path** (Todd, 2026-10-10, after two rejected
schemes: a `form!` key, since `form!` only declares how a form renders; then
`{title}-{model}[-{n}]-{path}` with a render-time instance number). 8 chars of
base62 picked in `use_form` through `use_server_cached`, so hydration reuses
the server's value. Only fullstack renders a component twice (server, then
hydration), and a FormState is never sent across, so a plain random id would
roll twice and split `id`/`for` (server's) from `aria-describedby` and newly
inserted nodes (client's). SPIKED and proven, with a control run showing the
split; `dioxus-fullstack-core` becomes a dependency with no formoxus feature,
because Cargo feature unification switches the hook's behaviour. Details in
the plan's P1-a.

**Labels beside their inputs (P1, #9).** Every `<label>` uses `for`, radios
included ("for consistency"); the field wrapper becomes a `div`. Ids are
`{form_id}-{path}`, with the form id deterministic (hydration must match the
server render; Dioxus 0.7.10 has no `useId`, only `current_scope_id`). Why:
error text inside a wrapping label joins the accessible name, and
`aria-describedby` cannot take it back out. Effect on the PARTS open points:
**B and C mostly dissolve.** Every `<label>` now captions exactly one input:
a text field's `label` is its caption (the wrapper becomes a `div` part), a
radio group's caption is its `legend`, and each radio's `label` captions that
radio. And field-level `label: "Name"` and in-widget `label: {…}` now
describe the same element.

**Configurable classes (P3, #10).** A SYSTEM is MVP; presets beyond `fx()`
and `bare()` are not (later: `pico()`, `bootstrap5()`, `tailwind()`, a Bulma
prototype). Keyed by (widget, part), plus fixed slots for non-widget
elements. Overridable at the app and the form; per field is
`class`/`class_plus` against the configured base.

**What the frameworks need (checked 2026-10-10, docs of each):** all mark
the INPUT when invalid: Bootstrap 5 `is-invalid` (with `.invalid-feedback` a
following SIBLING, which the un-nested markup gives), Bulma `is-danger` (and
`help is-danger` on the message), Foundation 6 `is-invalid-input` (and
`is-invalid-label`, and `is-visible` on `.form-error`), daisyUI 5
`input-error`; Pico uses `aria-invalid` only, Tailwind nothing. Django's own
`error_css_class` goes on the row wrapper, the old Bootstrap 3 style. Hence
P3-a's proposal: base class plus optional invalid class, per part. **Bulma's
`div.control` around the input is RECOMMENDED, not required** (Claude had said
required); classes likely cover it, short of icons and add-ons.

**Custom rendering elsewhere, for comparison:** Django swaps STRUCTURE through
templates at four levels (widget, field `template_name` since 5.0, form,
project `FORM_RENDERER`), and framework support lives in crispy-forms template
packs. leptos_form configures classes and label wrapping only; its sibling
leptos_form_tool has a `FormStyle` trait with a method per control. Formoxus's
equivalent of Django's templates is the `Widget` trait (4d) plus the paused
widget registry ([[widget-registry-idea]]), so no separate rendering layer.

## Element vs FieldControl (DECIDED 2026-10-11, Todd: option C)

**SUPERSEDED 2026-10-23 (`FIELDS_PLAN.md` item 15):** option A won in the end.
`HtmlElement` is flat, `Div | Fieldset | Label | Legend | Input(InputType) |
Textarea | Select`, and `FieldControl` is deleted: both of its jobs went
(`true_and_on`, below, and `BuiltinWidget`'s form-control part). What follows
is the history.

Parts can be `div`/`label`/`legend`, so the table must say what those may
carry. Three options were weighed: A, grow `FieldControl` into one renamed
`Element` (simple, but the type no longer says "this is a control", so
`true_and_on(Label)` compiles); B, a separate enum for parts (smallest now,
but needs a second validity concept beside `on:`); **C, chosen: a flat
`Element` (superset) keys the table and parts, a narrowed `FieldControl`
(`Input(_) | Textarea | Select`, subset) types everything that must be a
control (`true_and_on`, a widget's form-control part), with
`From<FieldControl> for Element`.** (As built 2026-10-12, `Element` is
NESTED rather than flat: `Div | Fieldset | Label | Legend |
Control(FieldControl)`. Global rows are `on: _`; only `disabled`, `form`,
`name` are `Control(_) | Fieldset`.) `Fieldset` stops being a control. Same
shape as decision 6. Found while weighing it: "valid on all four elements"
(73 of 95 rows) is NOT "global"; `disabled`/`name`/`form`/`readonly` are on
all four controls but not on a `<label>`, so those rows need sorting. Build
steps in the plan's P2.

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
5. **`Input` becomes the router for every `<input>`** (Todd; scheduled into P2
   2026-10-11, alongside `BuiltinWidget`): it matches on
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

10. **The `validated:` column, BUILT 2026-10-03** (`true_and_on(…)`,
    `is_also_validated_on` and `is_allowed` DELETED 2026-10-23, item 15:
    W-A's rule, that a validated key is a field setting, server-checked and
    emitted only where it fits, replaced them. `validated:` is now only
    `false` or `true`) (replacing `check: bool`,
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

13. **Step 3's shape (Todd, 2026-10-04).** The macro holds a field's
    attributes in **`ParsedAttrs`** (in `formoxus-macros`), and the run-time
    twin is **`FieldAttrs`** (merging today's `AllAttrs` and `FieldAttrs`).
    `ParsedAttrs` owns the routing, the duplicate check (`IndexMap::insert`
    returning the old value) and the ownership refusal. It stores the author's
    `Ident`/`LitStr` with each entry, so errors can point at the key, and it
    holds no `&'static str` (nothing parsed is `'static`; the run-time twin's
    strings ARE, because the macro emits them as literals). Its parts:
    - the TABLE map, keyed by `Attr`: identifier keys, looked up as the
      `snake_case` variant name, fully checked. Checked FIRST, so `aria_invalid`
      hits its owned row instead of slipping through as an extra;
    - `class_plus`: classes to append. **Only one of `class:` (replace, a
      token list in the table map) or `class+:` (append); both is a compile
      error** — "if you're replacing the list, put them all in there";
    - EXTRAS: **string-literal keys passed through unchecked**
      (`"hx-get": "/search"`, `"data-terms": "v2"`, `"x-cloak"` bare). This is
      Dioxus's own split (quoted names bypass its lists; `Attribute::new` takes
      any name) and covers htmx, Alpine and every `data-*`, including names
      with underscores. A quoted name the table knows (`"maxlength"`) is
      refused with "write `max_length:`", so quoting cannot bypass checks.
      Caveat to document: a name the DOM rejects (Vue's `@click`) fails in the
      browser, not in `form!`.
    - **No separate `aria`/`data` maps.** ARIA is a closed set (about 48), so
      each `aria-*` becomes a table row: `Author`, valid on every control
      (global), except `aria-invalid` (owned) and probably `aria-describedby`
      (owned from the start, since #9 will set it). Per-role ARIA validity is
      out of scope.
    `AttrSource` needs a list variant for `class` (and later `style`).

14. **Refinements to 13 (Todd, 2026-10-04).** `aria-describedby` is OWNED
    from the start (avoids a breaking change when #9 sets it). `ParsedAttrs`
    is ONE map, not separate parts: `ParsedAttrs(IndexMap<AttrId,
    ParsedAttr>)` with `enum AttrId { Std(Attr), NonStd(String) }` (a quoted
    key like `"hx-get"` is not an `Ident`, and keying by text avoids `"a"` vs
    `r"a"` comparing unequal) and `struct ParsedAttr { key_span: Span,
    source: AttrSource }` (the span lives in the value, since `Std(Attr)` has
    none). `class+` becomes a table row, `ClassPlus` (emitting HTML `class`,
    like `Required`/`RequiredTrue` share `required`), and likewise
    `StylePlus`; the "only one of `Class`/`ClassPlus`" rule is a check in
    `ParsedAttrs`' insert. **DECIDED: the `form!` keys are `class_plus:` and
    `style_plus:`** (Todd: "I can't argue with avoiding special-casing"), the
    ones the snake_case rule gives for free, not `class+:`. Older notes saying
    `class+` mean this.

15. **The full table (2026-10-04): 95 rows**, alphabetized by variant, with
    the 51 ARIA rows (ARIA 1.3 minus the deprecated `dropeffect`/`grabbed`)
    alphabetized at the end, at Todd's request. Because `check` and the
    emission walk `Attr::ALL`, alphabetical order puts `maxlength` before
    `minlength`; harmless, since both length errors (or both bound errors) can
    only fire together if min > max, which `form!` refuses. **Owned:** `name`,
    `type`, `value`, `checked`, presence `required`, `form` (moves the control
    to another form), `id` (reserved for #9), `multiple` (one string per leaf),
    `aria-invalid`, `aria-describedby` (#9), and — Claude's call, flagged to
    Todd — `aria-required` and `aria-checked` (they restate state formoxus
    controls). **Merged:** `class`, `class_plus`, `style`, `style_plus`.
    **Left out:** `step` (browser-validated but not server-checked, which
    breaks the "validation reaches the server" rule; add it with a `check`
    arm), `accept` (no file fields), and globals meaningless on a field
    (`hidden`, `inert`, `contenteditable`, `draggable`, `popover`, `slot`, `is`,
    `nonce`, `item*`, `heading*`), all still reachable as quoted keys.
    `tabindex` is `String` because `-1` is common and `AttrType::Int` is
    unsigned. Tests: one per-row spec expectation (`expected()` in the test
    module, stated as keywords independently of the table) checked on all 22
    input types and the 3 other controls, plus a test that every row HAS one.

16. **The runtime key (Todd, 2026-10-06).** A quoted key has to survive into
    the expansion, so the runtime needs its own `AttrId`: `pub enum AttrKey {
    Std(Attr), NonStd(&'static str) }` in `formoxus-attrs`. It cannot BE the
    macro's `AttrId`, whose `NonStd(String)` holds text parsed at expansion
    time; the macro writes it out as a literal, which is `&'static str`. Values
    of quoted keys (and of the table's `String` rows: `title`, `autocomplete`,
    ARIA) are `AttrValue::String(String)`, which cost `AttrValue` its `Copy`
    (`&'static str` would have kept it, but then `title: format!(…)` could not
    be written). **ONE runtime map, `IndexMap<AttrKey, AttrValue>`**, replacing
    both `AllAttrs` and the old `FieldAttrs(IndexMap<&'static str, String>)`,
    chosen over two maps in one struct because it keeps the author's order
    across both kinds and the macro emits one collection.

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
   (Todd: lists, not space- or semicolon-delimited strings.) ANSWERED AND BUILT
   2026-10-07 (Todd; the error cases are listed in `ATTRIBUTES_PLAN.md` 3d): `class: [dark, my_class, "real_underscore"]` (brackets) and
   `style: { color: red, font_size: "20px" }` (braces). Bare identifiers turn
   `_` into `-` in class names, style properties AND style values alike (one
   rule); quote to keep a real underscore. Keywords allowed bare via
   `Ident::parse_any` (`position: static`). BEM's `card__title` must be quoted.
   Bare numbers ARE allowed (decided 2026-10-07), and `!important` last; was open: whether a style value may hold bare numbers (`20px`, `0`) or only
   a run of identifiers (Todd leaning to numbers too, 2026-10-07; `50%` and `-1px` are two tokens each, glued by the parser; quotes inside a quoted value pass through, since SSR escapes both kinds). Rust's lexer rejects
   `2em`/`1.5em` ("expected at least one digit in exponent") BEFORE the macro
   runs, so those can never be bare. The macro writes each declaration out as
   `"font-size: 20px"`, so both stay `AttrValue::List(&'static [&'static str])`
   and `AttrType::List` goes. Todd is writing the parser himself (2026-10-07).
3. `class` on a `RadioGroup`: the fieldset, every radio, or refused?
   ANSWERED by PARTS (2026-10-10, above): whichever part it is addressed to;
   unaddressed, per that section's open point A.
4. `id`: reserved until issue #9 settles its id scheme (`<label for>`, per-form
   prefix); see [[html-attributes-reference]] item 4.
5. ~~The macro side's knowledge of value types: mirrored list or loose parse?~~
   ANSWERED 2026-10-03: neither; the macro reads `formoxus-attrs` directly.
6. Emission when an attribute is invalid on the element but the author set it
   anyway: compile error for built-in widgets (Todd's intent), let through for
   `Custom`.
7. Does `disabled` get a rule about submission? A disabled control is not
   submitted, so a non-optional field reports "This field is required."
