---
name: hand-written-rsx-gotchas
description: "Two rsx! traps hit writing the RadioGroup widget by hand (2026-09-25) — no statements in a `for` body, and what a forgotten `rsx!` wrapper looks like; will recur with every new widget in src/widgets/"
metadata:
  type: project
---

Hand-written `rsx!` in `formoxus/src/widgets/`, as opposed to the
macro-*generated* rsx traps in [[formoxus-darling-gotchas]]. These will recur:
the roadmap still has `select_multiple`, `checkbox_multiple` and `file` to write.

**An `rsx!` `for` body takes nodes, not statements.** `for x in xs { let y = …; … }`
inside an `rsx!` fails with a bare `error: expected identifier` and the caret on
the `=`. This bites whenever each iteration needs its own setup — the case that
found it was `RadioGroup`, where every radio needs its own `path.clone()` to move
into its own `onchange` handler, and there is nowhere in the loop to make the
clone. `Select` hides the problem by having exactly one handler for the whole
widget.

Fix: build the children as a `Vec<Element>` with `.map()` *outside* the `rsx!`,
where statements are legal, then splice with `{ items.into_iter() }`.
`members/field_set.rs` already does this for its members, so it is the house
idiom rather than a workaround.

**A forgotten `rsx! { … }` wrapper reads as a struct-literal parse error, and
masks everything else.** Without it, rustc parses `fieldset { class: "…" }` as a
struct literal and says so:

```text
error: expected one of `,`, `:`, or `}`, found `{`
   |     fieldset { class: "form-field radio-group",
   |     -------- while parsing this struct
   |         legend {
   |         ------ ^ while parsing this struct field
help: try naming a field
   |         legend: legend {
```

The tells: rustc talking about **structs and fields** where you wrote markup, and
offering to name an element as a field. Two traps in reading it:

- It is a *parse* error, so type checking never runs and NOTHING downstream is
  reported. A large rsx block that yields only two errors is itself the signal —
  a genuinely tangled one yields many.
- The accompanying `unused_imports` warning names imports the body visibly uses
  (`ValuesByPath`, `FieldProps`, `SelectChoice`, `get_current` were all reported
  unused), because the body they appear in never parsed. Do not chase it.

**Text vs nodes, for reference:** a text child is a quoted literal with the
interpolation inside the quotes (`"{text}"`, formatted like `format!`); a node
child is a braced expression (`{ element }`). An `if` with no `else` renders
nothing, which is how `Select` and `VariantSet` gate an optional label and the
` *` required marker — wrapper always present, contents gated.

## Pass-through attributes: `extends`, and why `class` is not a namespace (2026-09-26)

Found while plumbing `#[props(extends = …)] attrs: Vec<Attribute>` through the
widgets, for [[constraint-attributes-gap]] and author-supplied attributes.

**`extends = X` names an HTML ELEMENT, not the component.** The macro looks for a
trait called `XExtension`, which only real elements have, so a wrong name fails
with a "similarly named trait" suggestion pointing somewhere useless
(`ColgroupExtension` for `radio_group`). The trap is that it *coincides* for most
of formoxus's widgets — `Input`→`input`, `Textarea`→`textarea`, `Select`→`select`
— so the rule looks like "lowercase the component". It is not: `Checkbox` renders
an `<input type="checkbox">` and needs `extends = input`, and `RadioGroup` renders
a `<fieldset>` and needs `extends = fieldset`.

**The `extends` target IS the per-widget attribute filter**, which is worth
knowing before building one by hand. It decides which names a caller may write
*unquoted*: under `fieldset`, `placeholder:` is a compile error, which is correct.
Limitation: a quoted arbitrary name (`"data-hx-post": "/save"`) bypasses the check
under any target, so it constrains the unquoted vocabulary only.

**The spread must be LAST among an element's attributes.** `rsx!` reads everything
after `..attrs` as children, so an attribute below it is
`error: Attributes must come before children in an element`. In `Select` that puts
the spread between the attributes and the `<option>` children.

**`class` cannot ride the namespace mechanism, and there is no extension point.**
Verified in the installed sources:

- `Attribute.namespace` takes exactly ONE value across all of `dioxus-html` —
  `"style"`, 407 times, for the CSS properties in `attribute_groups.rs`. Nothing
  else uses an attribute namespace.
- The grouping is hardcoded per renderer: `dioxus-ssr-0.7.10/src/renderer.rs:151`
  is `else if attr.namespace == Some("style")`. Any other namespace falls through
  to `write_attribute` and is emitted once per occurrence — so inventing a
  `"class"` namespace yields duplicates, not concatenation.
- Real XML namespaces DO exist in Dioxus, but on **elements** (`svg
  "http://www.w3.org/2000/svg"` and children, `elements.rs:1639+`). The attribute
  field got the same name and then only ever served the style hack, which its own
  doc comment admits ("Doesn't exist in the html spec").
- `class` merging exists only in the MACRO: `dioxus-rsx-0.7.10/src/element.rs`
  has `merged_attributes`, with tests showing `class: "foo"` + `class: "bar"`
  collapsing to `"foo bar"` at compile time. A runtime `Vec<Attribute>` arriving
  through a spread never goes through it.

**Consequence for the attrs design.** Dioxus does not dedupe: two `maxlength`
attributes both render and HTML parsing takes the FIRST, so a widget's own value
silently beats a caller's. An `IndexMap<&'static str, Attribute>` fixes that by
replace-on-insert — right for the constraints, WRONG for `class`, which wants
append. So "duplicates impossible by design" is really "duplicates resolved by
per-key policy". Cleanest split found so far: keep the widget's own `class` out of
the spread channel entirely and compose it where the widget builds its class
string, which is also where [[emitted-classes-and-strings]] will land.

*Unverified:* SSR writes duplicates and HTML parsing takes the first, but a
client-side render goes through `setAttribute`, where the LAST write would win —
which would make SSR and CSR disagree. Check `dioxus-web` before relying on
either.
