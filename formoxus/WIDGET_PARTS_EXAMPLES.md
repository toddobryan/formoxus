# Widget parts: `form!` → HTML, worked through

Examples for deciding the open points of widget PARTS (A–E below), and raw
material for the eventual docs. The design is in
`.claude/memory/attribute_rules_design.md`, "DECIDED 2026-10-10: widgets have
PARTS"; the build order is Step 4 of `ATTRIBUTES_PLAN.md`.

**Every "today" block is real output**, rendered from `12cb3c3` through the
suite's `render_to_html` and only re-indented. The "with parts" blocks are
hand-written: same markup, with only what parts would change.

All examples use this model:

```rust
#[derive(Facet)]
struct Signup {
    name: String,
    email: String,
    bio: Option<String>,
    age: u8,
    state: String,
    agreed: bool,
    newsletter: Option<bool>,
    contact: Contact, // enum Contact { Email, Phone }
}

const STATES: &[(&str, &str)] = &[("AL", "Alabama"), ("AK", "Alaska")];
```

---

## 1. What each widget expands to (doc material)

The part names are read straight off these trees. Elements in *italics* are
not parts: the same element name has two roles in the widget (the label-text
`<span>` and the required-marker `<span>`), or there is one per choice
(`<option>`), or formoxus owns it (the error list).

```text
text, email, number, … (every <input> widget)
  label.fx-form-field                    ← part `label`
    *span.fx-field-label*  (label text, then *span.fx-required*)
    input.fx-control.fx-input            ← part `input`   (the form control)
    *errors*

textarea
  label.fx-form-field                    ← part `label`
    *span.fx-field-label*
    textarea.fx-control.fx-textarea      ← part `textarea` (the form control)
    *errors*

select   (also an Option<bool> with no widget:, and an enum's variant picker)
  label.fx-form-field                    ← part `label`
    *span.fx-field-label*
    select.fx-control.fx-select          ← part `select`  (the form control)
      *option* …
    *errors*

checkbox
  label.fx-form-field                    ← part `label`
    input.fx-control.fx-checkbox         ← part `input`   (the form control)
    *span.fx-field-label*                (text AFTER the box)
    *errors*

radio_group
  fieldset.fx-form-field.fx-radio-group  ← part `fieldset`
    legend.fx-legend.fx-field-label      ← part `legend`  (the label text)
    label.fx-choice        × per choice  ← part `label`
      input.fx-control.fx-radio          ← part `input`   (the form control)
    *errors*

hidden
  input                                  ← part `input`   (no label at all)
```

Notice already: in a single-element widget the `<label>` is the WHOLE field
(text, control and errors), but in a radio group `label` is one choice. That
is open point B.

---

## 2. A text field: nothing changes

```rust
name => { placeholder: "Ada Lovelace", class_plus: [wide] },
```

Today:

```html
<label class="fx-form-field">
  <span class="fx-field-label">Name<span class="fx-required" aria-hidden="true"> *</span></span>
  <input class="fx-control fx-input wide" type="text" name="name" value=""
         required=true placeholder="Ada Lovelace"/>
</label>
```

With parts: **identical**, under every option for A. Unaddressed attributes go
to the form control, and there is only one candidate.

### 2b. Addressing the label

```rust
name => {
    widget: text { label: { class_plus: [row] } },
    placeholder: "Ada Lovelace",
},
```

```html
<label class="fx-form-field row">                       <!-- ← the label part -->
  <span class="fx-field-label">Name<span class="fx-required" aria-hidden="true"> *</span></span>
  <input class="fx-control fx-input" type="text" name="name" value=""
         required=true placeholder="Ada Lovelace"/>
</label>
```

Two things to notice:

- `widget: text` had to be written to get braces, although `text` is the
  default for a `String`.
- The label TEXT (the `<span>`) cannot be reached; only the wrapper can. In a
  radio group the label text is a `<legend>`, which CAN be (example 5).

### 2c. `label` twice in one field (open point C)

```rust
name => {
    label: "Full name",                                  // the TEXT
    widget: text { label: { class_plus: [row] } },       // the ELEMENT
},
```

```html
<label class="fx-form-field row">
  <span class="fx-field-label">Full name<span class="fx-required" aria-hidden="true"> *</span></span>
  <input class="fx-control fx-input" type="text" name="name" value="" required=true/>
</label>
```

---

## 3. A textarea, and what is refused

```rust
bio => { widget: textarea, rows: 4, max_length: 500 },
```

Today, and with parts (identical):

```html
<label class="fx-form-field">
  <span class="fx-field-label">Bio</span>
  <textarea class="fx-control fx-textarea" name="bio" value="" rows=4 maxlength=500></textarea>
</label>
```

**Refused: a validated key inside a part** (decision 4):

```rust
bio => { widget: textarea { textarea: { max_length: 500 } } },
```
```text
error: `max_length` is checked against the whole field; write it beside `widget:`,
       not inside `textarea: {…}`
```

**Refused: an attribute the part's element cannot take** (open point D):

```rust
bio => { widget: textarea { label: { rows: 4 } } },
```
```text
error: `rows` is not valid on `<label>`; `textarea`'s `rows` goes on the
       `textarea` part, which is where it lands when you write it beside `widget:`
```

**Allowed** on a `label` part: the global attributes only (`class`,
`class_plus`, `style`, `style_plus`, `title`, `lang`, `dir`, `hidden`, quoted
`data-*`). `for` is formoxus's (issue #9).

---

## 4. A checkbox

```rust
agreed => { label: "I agree to the terms", required_true, class_plus: [switch] },
```

Today (with `class_plus` added to the real render by hand, on the input, where
the code puts it):

```html
<label class="fx-form-field">
  <input class="fx-control fx-checkbox switch" name="agreed" type="checkbox" required=true/>
  <span class="fx-field-label">I agree to the terms<span class="fx-required" aria-hidden="true"> *</span></span>
</label>
```

With parts: identical. `required_true` is validated, so it goes to the form
control; `class_plus` is unaddressed, and the form control is the only
candidate.

---

## 5. A radio group: where the options actually differ

```rust
state => {
    widget: radio_group { choices: STATES },
    class_plus: [inline],
    "data-group": "states",
},
```

Today (with `class_plus` added; today it is dropped, pinned by
`on_a_radio_group_an_author_class_goes_nowhere_yet`):

```html
<fieldset class="fx-form-field fx-radio-group" role="radiogroup" data-group="states">
  <legend class="fx-legend fx-field-label">State<span class="fx-required" aria-hidden="true"> *</span></legend>
  <label class="fx-choice">
    <input class="fx-control fx-radio" type="radio" name="state" value="AL" required=true/>Alabama
  </label>
  <label class="fx-choice">
    <input class="fx-control fx-radio" type="radio" name="state" value="AK" required=true/>Alaska
  </label>
</fieldset>
```

### 5a. Open point A, option A1: unaddressed goes to "the element that stands for the field"

That is the form control when there is one, the `<fieldset>` when there are
several. **Today's behaviour, plus the class:**

```html
<fieldset class="fx-form-field fx-radio-group inline" role="radiogroup" data-group="states">
  <legend class="fx-legend fx-field-label">State …</legend>
  <label class="fx-choice">
    <input class="fx-control fx-radio" type="radio" name="state" value="AL" required=true/>Alabama
  </label>
  …
</fieldset>
```

### 5b. Option A2: unaddressed always goes to the form control

```html
<fieldset class="fx-form-field fx-radio-group" role="radiogroup">
  <legend class="fx-legend fx-field-label">State …</legend>
  <label class="fx-choice">
    <input class="fx-control fx-radio inline" type="radio" name="state" value="AL"
           required=true data-group="states"/>Alabama
  </label>
  <label class="fx-choice">
    <input class="fx-control fx-radio inline" type="radio" name="state" value="AK"
           required=true data-group="states"/>Alaska
  </label>
</fieldset>
```

One rule everywhere ("unaddressed means the thing you type into"), but
`inline` is a layout class, and layout is the fieldset's job; and a `title`
would be read out on every radio. Under A2, putting it on the group means
`fieldset: { class_plus: [inline] }`.

### 5c. Every part addressed

```rust
state => {
    widget: radio_group {
        choices: STATES,
        fieldset: { class_plus: [inline] },
        legend: { class_plus: [visually_hidden] },
        label: { class_plus: [pill] },
        input: { class_plus: [big] },
    },
},
```

```html
<fieldset class="fx-form-field fx-radio-group inline" role="radiogroup">
  <legend class="fx-legend fx-field-label visually-hidden">State …</legend>
  <label class="fx-choice pill">
    <input class="fx-control fx-radio big" type="radio" name="state" value="AL" required=true/>Alabama
  </label>
  <label class="fx-choice pill">
    <input class="fx-control fx-radio big" type="radio" name="state" value="AK" required=true/>Alaska
  </label>
</fieldset>
```

This is the whole of the old `group_class`/`input_class` hold, with nothing
radio-group-specific in the grammar.

### 5d. Open point B: the same key, two different elements

Put example 2b and 5c side by side:

```rust
name  => { widget: text        {                     label: { class_plus: [row] } } },
state => { widget: radio_group { choices: STATES,    label: { class_plus: [row] } } },
```

```html
<label class="fx-form-field row">…the whole name field…</label>

<fieldset class="fx-form-field fx-radio-group">
  <legend …>State</legend>
  <label class="fx-choice row">…Alabama…</label>            <!-- each CHOICE -->
  <label class="fx-choice row">…Alaska…</label>
</fieldset>
```

`label` means "the field" in one and "each choice" in the other. True to the
markup, which is the point of element names; whether the docs' element trees
(section 1) are enough to make that unsurprising is the question.

### 5e. `required_true` on a radio group over a bool (D1, dissolved)

```rust
agreed => { widget: radio_group, required_true },
```

Step 4 as first planned would have REFUSED this: the group's one element was
`<fieldset>`, and the table allows `required_true` only on a checkbox, a
`<select>` or a radio. With parts it is validated, so it routes to the `input`
part, `<input type=radio>`, and is accepted. It is server-checked only (a
browser cannot require that the "Yes" radio be picked), so no new attribute
appears; each radio keeps the `required=true` it already has for presence.

---

## 6. An enum field: no parts to address

```rust
contact => { class_plus: [compact] },
```

Today, and with parts (identical):

```html
<fieldset class="fx-group fx-variant-set">
  <legend class="fx-legend fx-field-label">Contact<span class="fx-required" aria-hidden="true"> *</span></legend>
  <label class="fx-form-field">
    <select class="fx-control fx-select compact" required=true>
      <option value="" selected=true disabled=true hidden=true>Choose...</option>
      <option value="Email">Email</option>
      <option value="Phone">Phone</option>
    </select>
  </label>
</fieldset>
```

An enum field takes no `widget:`, so it has no braces to address parts in.
The `<fieldset>` and `<legend>` belong to the variant SET (the group the
variant's own fields render into), not to a widget. Unaddressed attributes go
to the `<select>`, as today, under either A.

---

## 7. A custom widget (open point E, new)

A hypothetical Markdown editor with a live preview:

```text
markdown (custom)
  label.fx-form-field
    *span.fx-field-label*
    div.md-editor
      textarea.fx-control.md-source      ← the form control
      div.md-preview
    *errors*
```

```rust
body => {
    widget: custom(MarkdownEditor) {
        textarea: { rows: 20 },
        div: { class_plus: [split] },      // which div?
    },
},
```

There are two `<div>`s with different roles, so an element name cannot
address either. Options: E1, a custom widget may only declare parts whose
element names are unique (here: none for the divs, so they get no author
attributes); E2, a custom widget names its parts freely (`editor`,
`preview`) and documents them, and only the built-ins promise element names;
E3, everyone may add a name where the element is ambiguous (`div.preview`
style), which is E2 with extra steps.

---

## The open points, with what the examples suggest

| | question | options | the examples suggest |
|---|---|---|---|
| **A** | where an unaddressed attribute goes | A1 the element that stands for the field (control, or fieldset when several); A2 always the form control | A1: same as today, and 5b shows A2 scattering a layout class and `data-*` over every radio |
| **B** | `label` is the field in one widget and a choice in another | B1 accept it, the element trees document it; B2 role names after all | B1, if section 1's trees ship in the docs; it is what "element names" means |
| **C** | `label:` the text vs `label: {…}` the element | accept; or rename the text key (`label_text:`) | accept: different brace levels, and the element form is rare |
| **D** | the table has no word for `<label>`/`<legend>` | add `Label`, `Legend` to `FieldControl` (and to every global row, or a `Global` shorthand); rename `FieldControl` | add them; a rename is cheap before publishing (the crate itself was renamed `formoxus-spec` 2026-10-10) |
| **E** | custom widgets with two of the same element | E1 unique element names only; E2 free names for custom widgets | open; E2 seems likelier to be needed |
