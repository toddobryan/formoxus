---
name: html-attributes-reference
description: "Which HTML attributes are valid on which form element and <input> type, taken from the WHATWG spec on 2026-10-02, compared against dioxus-html 0.7.10, and mapped onto the element each formoxus widget spreads `attrs` onto. Written for C6 (author attributes) and useful for issue #4 (emit only valid pairs)"
metadata:
  type: reference
---

Pulled 2026-10-02 for C6 ([[mvp-scope]] step 3). Sources: the WHATWG HTML
Living Standard (`multipage/input.html`, `form-elements.html`, `forms.html`,
`dom.html`), parsed from the raw HTML rather than summarized, and the installed
`dioxus-html-0.7.10/src/elements.rs` and `attribute_groups.rs`. Re-pull before
relying on this for anything after a year or so, because the spec moves:
`alpha`, `colorspace`, `command` and `commandfor` are all recent additions.

## Where each widget's `attrs` land

The spread target decides which rows below matter. It is NOT always the
element the user thinks of as "the field".

| Widget | Spread lands on | Notes |
|---|---|---|
| `Input` | `<input type=…>` | rows depend on the type, see the table below |
| `Input` with `hidden` | nothing | the hidden branch has no spread at all (`a_hidden_input_gets_no_constraints`) |
| `Textarea` | `<textarea>` | |
| `Checkbox` | `<input type="checkbox">` | |
| `Select`, `VariantSelect` | `<select>` | the `<option>`s are generated, so they get nothing |
| `RadioGroup` | the wrapping `<fieldset>` | NOT the radios. A per-radio attribute such as `required` or `autofocus` given here lands on the fieldset, where it is invalid or meaningless. The same problem was why `aria_invalid` became a prop |
| buttons | `<button>` | through `ButtonSpec`, not `ScalarWidget` |

## Global attributes: valid on every element

From the spec (`dom.html`, "global attributes"): `accesskey`,
`autocapitalize`, `autocorrect`, `autofocus`, `class`, `contenteditable`, `dir`,
`draggable`, `enterkeyhint`, `headingoffset`, `headingreset`, `hidden`, `id`,
`inert`, `inputmode`, `is`, `itemid`, `itemprop`, `itemref`, `itemscope`,
`itemtype`, `lang`, `nonce`, `popover`, `slot`, `spellcheck`, `style`,
`tabindex`, `title`, `translate`, `writingsuggestions`.

Also valid everywhere: every `data-*`, `role` and `aria-*` (ARIA in HTML
restricts which roles and states suit which element, which is a separate
check), and the `on*` event handlers (irrelevant to `form!`, since a string
attribute cannot carry a Rust closure).

Of these, the ones a form author plausibly wants: `class`, `id`, `style`,
`title`, `autofocus`, `inputmode` (which keyboard a phone shows, the right fix
for numbers rendered as `type="text"`), `enterkeyhint`, `autocapitalize`,
`autocorrect`, `spellcheck`, `lang`, `dir`, `tabindex`, `data-*`, `aria-*`.

## Element-specific attributes (spec)

| Element | Attributes besides the global ones |
|---|---|
| `<input>` | `accept`, `alpha`, `alt`, `autocomplete`, `checked`, `colorspace`, `dirname`, `disabled`, `form`, `formaction`, `formenctype`, `formmethod`, `formnovalidate`, `formtarget`, `height`, `list`, `max`, `maxlength`, `min`, `minlength`, `multiple`, `name`, `pattern`, `placeholder`, `popovertarget`, `popovertargetaction`, `readonly`, `required`, `size`, `src`, `step`, `type`, `value`, `width`. Which ones apply depends on the type; see the next table |
| `<textarea>` | `autocomplete`, `cols`, `dirname`, `disabled`, `form`, `maxlength`, `minlength`, `name`, `placeholder`, `readonly`, `required`, `rows`, `wrap`. **No `pattern`**, although formoxus validates one; see [[constraint-attributes-gap]] |
| `<select>` | `autocomplete`, `disabled`, `form`, `multiple`, `name`, `required`, `size`. **No `placeholder`, `readonly`, `minlength` or `maxlength`** |
| `<fieldset>` | `disabled`, `form`, `name`. Nothing else: no `required` and no constraints |
| `<button>` | `command`, `commandfor`, `disabled`, `form`, `formaction`, `formenctype`, `formmethod`, `formnovalidate`, `formtarget`, `name`, `popovertarget`, `popovertargetaction`, `type`, `value` |
| `<option>` | `disabled`, `label`, `selected`, `value` |
| `<label>` | `for` |
| `<legend>` | none |
| `<form>` | `accept-charset`, `action`, `autocomplete`, `enctype`, `method`, `name`, `novalidate`, `rel`, `target` |

## `<input>`: which attributes apply to which type

The spec's own non-normative summary table, cut to the types formoxus can
render. `disabled`, `form`, `name`, `type` and `value` apply to every type and
are left out, and so are the rows that only apply to types formoxus never
renders: `accept` (file); `alt`, `src`, `width`, `height`, `formaction`,
`formenctype`, `formmethod`, `formnovalidate`, `formtarget` (submit/image);
`popovertarget`, `popovertargetaction` (buttons). Range and file are also not
rendered.

| Attribute | hidden | text, search | tel, url | email | password | date, month, week, time | datetime-local | number | color | checkbox, radio |
|---|---|---|---|---|---|---|---|---|---|---|
| `autocomplete` | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Yes | Yes | · |
| `dirname` | Yes | Yes | Yes | Yes | Yes | · | · | · | · | · |
| `list` | · | Yes | Yes | Yes | · | Yes | Yes | Yes | Yes | · |
| `placeholder` | · | Yes | Yes | Yes | Yes | · | · | Yes | · | · |
| `readonly` | · | Yes | Yes | Yes | Yes | Yes | Yes | Yes | · | · |
| `required` | · | Yes | Yes | Yes | Yes | Yes | Yes | Yes | · | Yes |
| `size` | · | Yes | Yes | Yes | Yes | · | · | · | · | · |
| `minlength`, `maxlength`, `pattern` | · | Yes | Yes | Yes | Yes | · | · | · | · | · |
| `min`, `max`, `step` | · | · | · | · | · | Yes | Yes | Yes | · | · |
| `multiple` | · | · | · | Yes | · | · | · | · | · | · |
| `alpha`, `colorspace` | · | · | · | · | · | · | · | · | Yes | · |
| `checked` | · | · | · | · | · | · | · | · | · | Yes |

Facts worth knowing from this table:

- **`readonly` does not apply to a checkbox, radio, color or `<select>`.** A
  read-only checkbox is a common request, and the HTML answer is `disabled`.
  But a disabled control is left out of the submission, so for formoxus it
  arrives as `Empty`.
- **`placeholder` does not apply to a date, time or color input, or to a
  `<select>`.** `Select` makes its own "Choose..." option instead.
- **`multiple` on an email input** turns one leaf into a comma-separated list.
  The leaf still holds one string, so it would parse as one `String`, but the
  browser's email validation changes.
- The date types take `min`/`max`/`step` in their OWN value format
  (`2026-10-02`), not formoxus's `i128`/`f64` bounds. See issue #4.

## Where dioxus-html 0.7.10 disagrees

This only matters for UNQUOTED names in a hand-written `rsx!` call, where
`#[props(extends = input)]` filters by dioxus-html's list (see
[[hand-written-rsx-gotchas]]). An `Attribute` that formoxus builds at runtime
from a string name passes through regardless.

- Missing from dioxus: `alpha`, `colorspace` and `dirname` on `<input>`;
  `dirname` on `<textarea>`; `command` and `commandfor` on `<button>`; and the
  global `headingoffset`, `headingreset` and `writingsuggestions`.
- Dioxus extras that are not in the spec's list for these elements: `capture`
  (from the HTML Media Capture spec) and `directory` (non-standard) on
  `<input>`; `initial_value`, `initial_checked` and `initial_selected`, which
  are Dioxus's own uncontrolled-input helpers. `autofocus`, `spellcheck`,
  `tabindex` and `autocorrect` are listed per element by Dioxus but are global
  in the spec.

## For C6: what an author should and should not set

Attributes **formoxus already sets** on the controls (from `widgets/*.rs`,
2026-10-02):

| Widget | Sets itself |
|---|---|
| `Input` | `class`, `type`, `name`, `value`, `required`, `aria-invalid` |
| `Textarea` | `class`, `name`, `value`, `required`, `aria-invalid` |
| `Checkbox` | `class`, `name`, `type`, `checked`, `aria-invalid` (plus `required` via constraints) |
| `Select` / `VariantSelect` | `class`, `name`, `required`, `aria-invalid`, and every `<option>` |
| `RadioGroup` | `class` on the fieldset; `class`, `type`, `name`, `value`, `checked`, `required`, `aria-invalid` on each radio |

**PROBED 2026-10-02: an author attribute that a widget ALSO sets explicitly is
DUPLICATED, not replaced.** `with_attrs` giving `name`, `class` and `type`
rendered `<input class="fx-control fx-input" type="text" name="name" …
name="hijack" class="mine" type="email"/>`. `ScalarWidget`'s merge dedupes only
WITHIN the spread (which is why a raw `maxlength` cleanly replaced a
constraint's), and the widget's own `class:`/`name:`/`type:` are outside it.
In served HTML the parser keeps the FIRST of a duplicated attribute, so
formoxus's wins there; a client-side render sets attributes in order, so the
author's would likely win in the live DOM (unverified). Either way the markup
is invalid, and SSR and client could disagree. So the owned-attribute refusal
in `form!` is required, not polish, and `class:`/`class+:` must be resolved
into the widget's ONE `class` value, never put in the `attrs` map.
`tests/suite/author_attrs.rs` pins the maxlength override; the duplicate case
was left unpinned on purpose, since it should become impossible to write.

**SUPERSEDED in shape 2026-10-02** by [[attribute-rules-design]]: these groupings become ownership and validity RULES in one table, not separate mechanisms. The facts below still hold.

Groupings to decide in C6. These are suggestions, not decisions:

1. **Owned by formoxus, so an author setting one would break something:**
   `name` (the store key and the wire path), `type` (comes from `widget:`),
   `value`, `checked` and `selected` (from the store), `form` (moves the control
   into a different form), `multiple` (a leaf holds ONE string). Probably
   reject at compile time.
2. **Owned by formoxus, but with a `form!` key already:** `required`,
   `minlength`, `maxlength`, `pattern`, `min` and `max`. An author writing the
   raw attribute would put a rule in the browser that `check` never enforces,
   which is exactly the split the constraint keys exist to prevent. Probably
   reject, with a message naming the key to use instead.
3. **`aria-invalid`.** Resolved by formoxus at the `Form` boundary;
   caller-wins merge must not let an author override it (see
   [[shrink-fieldprops-idea]]).
4. **`id`, which collides with issue #9.** `aria-describedby` needs an id on
   each message, and probably on each control. If authors can set `id` before
   #9 lands, #9 inherits a collision to design around. Worth deciding the id
   scheme first, or reserving `id` for now.
   **Todd, 2026-10-02: when ids arrive, stop nesting the control in the
   `<label>` and use `<label for=ID>`**, with `.fx-form-field` moving to a `<div>`
   wrapper. This also repairs #9's own plan. A wrapping label's accessible name
   is ALL the label's text except the control, so the `FieldErrors` list inside
   it is part of the name, and `aria-describedby` only ADDS a description; it
   does not remove anything from the name. (The ` *` was never the problem: it
   is `aria-hidden`, and e2e proves it stays out.) With `for=`, the label holds
   only the caption and the marker, the errors stay the input's next sibling
   (so `input[aria-invalid="true"] + *` still works), and they become a
   description. Ids must be unique per page, so the scheme needs a per-form
   prefix, not just the path. This is breaking (markup and CSS vocabulary), and
   it belongs with #9 (item 4). Posted as a comment on #9.
5. **`class`.** Todd's `class:` / `class+:` work. **DECIDED 2026-10-02: they
   target the CONTROL** (the `<input>`/`<textarea>`/`<select>`), not the
   wrapper, so the `for=` restructuring in item 4 does not change them. On
   `RadioGroup` the spread target is the `<fieldset>`, so that one still needs
   a deliberate answer. The trap from
   [[hand-written-rsx-gotchas]] is that class merging exists only in the `rsx!`
   MACRO. An explicit `class: "fx-control fx-input"` plus a spread `class`
   attribute at runtime is not merged into one, so `class+:` has to build the
   joined string itself.
6. **`disabled`.** Valid, but a disabled control is not submitted, so the
   server sees `Empty` and reports "This field is required." on a non-optional
   field. Either document it or handle it.
7. **Pass through:** `placeholder`, `autocomplete`, `inputmode`,
   `enterkeyhint`, `autocapitalize`, `autocorrect`, `spellcheck`, `readonly`,
   `size`, `rows`, `cols`, `wrap`, `dirname`, `list`, `title`, `style`, `lang`,
   `dir`, `tabindex`, `autofocus`, `data-*`, `aria-*` (except
   `aria-invalid`), `step` (the one bound-like attribute with no `form!` key;
   see [[constraint-attributes-gap]] on why formoxus does not emit it).

Whether to also reject an attribute that is not valid on the widget's element
(say `rows` on an `Input`, or `placeholder` on a `select`) is issue #4's
question, and the tables above are the data it needs. `field_kind`'s const
checks could do it at compile time for `Input`/`Textarea`/`Select`, since the
widget and, for `Input`, the type are both known to `form!`.
