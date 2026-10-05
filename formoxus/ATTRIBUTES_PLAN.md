# Attributes: the build order from here

A working checklist for the attribute-table work (C6, which absorbed issue #4).
**The reasoning behind every decision is in
`.claude/memory/attribute_rules_design.md`**, numbered; this file is just the
order to do things in and where to do them. Claude keeps it current when asked.

> **Status, 2026-10-04.** Steps 1–2 are done and pushed (`69a9225`): the table
> lives in `formoxus-attrs`, and the constraints formoxus already had are driven
> from it, with no behaviour change. **Next is step 3.**
>
> Line numbers below are from `69a9225`. They drift as the code changes; ask for
> a refresh rather than trusting an old one.

Who does what: Todd writes the library code; Claude writes and updates tests,
goldens and the e2e forms, and does mechanical fixes.

---

## Step 3: `form!` takes any table attribute, and refuses the ones formoxus owns

Decisions 13–14 in the design note. Done when everything a field body says
goes through one table lookup, owned attributes are compile errors, quoted keys
pass through, and the grammar change below has landed.

### 3a. The table grows (`formoxus-attrs/src/attrs.rs`)

- [x] **Generate `variant_name()` and `from_variant_name()`** (done by Todd) in the
  `attributes!` macro ([attrs.rs:83](../formoxus-attrs/src/attrs.rs#L83)), next
  to `from_name` ([:159](../formoxus-attrs/src/attrs.rs#L159)).
  `variant_name` is `stringify!($variant)`; `form!` turns `max_length` into
  `MaxLength` with `heck` and looks it up here.
- [x] **New rows** (DONE 2026-10-04: 95 rows, 44 alphabetized then 51 ARIA
  alphabetized; see the design note, decision 15, for what was left out and
  why) in `attributes! { … }`
  ([:195](../formoxus-attrs/src/attrs.rs#L195)):
  - `Class` (`owner: Merged`, a token list) and `ClassPlus` (emits HTML
    `class`, like `Required`/`RequiredTrue` share `required`).
  - `Style` and `StylePlus`, the same way, with a declaration list.
  - The ARIA attributes, about 48: `owner: Author`, valid on every control.
    **`aria-invalid` (already a row, [:320](../formoxus-attrs/src/attrs.rs#L320))
    and `aria-describedby` are owned by formoxus.**
  - The common author attributes, at least `autocomplete`, `readonly`, `rows`,
    `cols`, `inputmode`, `title`, `autofocus`. The full list, with where each is
    valid, is in `.claude/memory/html_attributes_reference.md`.
- [ ] **REMINDER (Todd, 2026-10-04): add `TokenList` and `Declarations`
  variants to `AttrValue`** ([:22](../formoxus-attrs/src/attrs.rs#L22)),
  *deferred until 3d shows how they get used*. `AttrType`
  ([:11](../formoxus-attrs/src/attrs.rs#L11)) already has both, and the
  `Class`/`ClassPlus`/`Style`/`StylePlus` rows use them, but nothing builds an
  `AttrValue` of either yet. What they wrap depends on how the widget resolves
  its one `class` value. Candidates: a list of class names for `TokenList`;
  either a list of `(property, value)` pairs or one string for `Declarations`.
  Claude raises this when 3d starts.

### 3b. The macro parses attributes generically (`formoxus-macros/src/form/field.rs`)

- [ ] **Replace the per-key fields** of `field_body!`
  ([field.rs:33](../formoxus-macros/src/form/field.rs#L33), invoked at
  [:64](../formoxus-macros/src/form/field.rs#L64)) with:
  - `widget` and `label` kept as named fields (they are not attributes);
  - `attrs: ParsedAttrs`, where
    ```rust
    struct ParsedAttrs(IndexMap<AttrId, ParsedAttr>);
    enum AttrId { Std(Attr), NonStd(String) }
    struct ParsedAttr { key_span: Span, source: AttrSource }
    enum AttrSource { Expr(Expr), Regex(LitStr), Flag, List(…) }
    ```
- [ ] **Routing in `ParsedAttrs`' insert**, in this order:
  1. An identifier key → `from_variant_name` of its PascalCase form. Unknown
     → "unknown key", with a did-you-mean (`suggest.rs`) and the legal list.
  2. Owned (`owner: Formoxus`) → compile error naming it.
  3. Already present → "given twice", pointing at the second.
  4. `Class` with `ClassPlus` present, or the reverse (same for `Style`) →
     error: only one per field.
  5. A **string-literal** key (`"hx-get"`) → `NonStd`, passed through. If the
     table knows that HTML name (`"maxlength"`), refuse it: "write
     `max_length:`".
- [ ] **Parse each value by the row's `attr_type()`**: `Int`/`Bound`/`String`
  → `Expr`; `Regex` → `LitStr` (and keep today's `regress` compile check);
  `Flag` → **no value at all** (a bare key); lists → the list form.
- [ ] **`constraints_tokens`** ([:83](../formoxus-macros/src/form/field.rs#L83))
  becomes one loop over the map, emitting `(Attr::X, AttrValue::…(…))` per
  entry. `NonStd` entries need a home in the runtime type (3c).
- [ ] **`type_checks`** ([:152](../formoxus-macros/src/form/field.rs#L152))
  becomes one loop emitting `assert!(applies(Attr::X, shape), msg)` per entry.
  Make `field_kind::applies` public for it
  ([field_kind.rs:42](../formoxus/src/field_kind.rs#L42)); the `takes_*`
  wrappers ([:54](../formoxus/src/field_kind.rs#L54)–[:75](../formoxus/src/field_kind.rs#L75))
  can then go. **Generate each message from the key and the row's `for:`**
  (`Text` → "a String field", `Int | Float` → "a number field") so the goldens
  stay byte-identical. The bound checks and min ≤ max stay, keyed on `Min`/`Max`.
- [ ] **The `required: false` check** in `Parse`
  ([:319](../formoxus-macros/src/form/field.rs#L319)) goes: `required_true` is
  a bare flag, so there is no `false` to write.

### 3c. One runtime map (`formoxus/src/fields.rs`, `formoxus/src/form/spec.rs`)

- [ ] **Merge `AllAttrs` ([fields.rs:56](../formoxus/src/fields.rs#L56)) and
  `FieldAttrs` ([:75](../formoxus/src/fields.rs#L75)) into one `FieldAttrs`**,
  keyed by `Attr` for table attributes, plus the pass-through extras keyed by
  their `&'static str` name.
- [ ] **`FieldSpec`**: `constraints` and `attrs`
  ([spec.rs:40](../formoxus/src/form/spec.rs#L40),
  [:54](../formoxus/src/form/spec.rs#L54)) become one field;
  `with_constraints` and `with_attrs`
  ([:144](../formoxus/src/form/spec.rs#L144), [:149](../formoxus/src/form/spec.rs#L149))
  become one builder.
- [ ] **`distribute_specs`** copies the one map
  ([fields.rs:589](../formoxus/src/fields.rs#L589)–[:590](../formoxus/src/fields.rs#L590)).
- [ ] **`render`** ([:449](../formoxus/src/fields.rs#L449),
  [:453](../formoxus/src/fields.rs#L453)) hands down table attributes plus
  extras; `ScalarWidget`'s merge
  ([scalar.rs:39](../formoxus/src/widgets/scalar.rs#L39)–[:46](../formoxus/src/widgets/scalar.rs#L46))
  keeps "constraint attributes first, caller's last".

### 3d. `class` and `class_plus`

> **First:** decide what `AttrValue::TokenList` and `AttrValue::Declarations`
> wrap (the reminder in 3a), now that this step shows how they are used.

- [ ] Resolve them into **the widget's one `class` value**, never a spread
  attribute (a spread `class` is duplicated, not merged). The places each widget
  sets its own class today:
  [input.rs:85](../formoxus/src/widgets/input.rs#L85),
  [textarea.rs:53](../formoxus/src/widgets/textarea.rs#L53),
  [checkbox.rs:48](../formoxus/src/widgets/checkbox.rs#L48),
  [select.rs:63](../formoxus/src/widgets/select.rs#L63),
  [variant_select.rs:61](../formoxus/src/widgets/variant_select.rs#L61),
  [radio_group.rs:56](../formoxus/src/widgets/radio_group.rs#L56) (per radio;
  the fieldset's class is the wrapper's).
- [ ] `class:` replaces formoxus's classes (including `fx-control`);
  `class_plus:` appends. `style`/`style_plus` the same.

### 3e. The grammar change: `required: true` → `required_true`

Breaking, so it all moves together. Claude updates the tests side.

- [ ] Library: the `RequiredTrue` row is a `Flag`, so 3b already parses it bare.
- [ ] Goldens: `form_required_false` (delete: nothing to write),
  `form_required_on_a_string`, `form_required_on_an_optional_bool` (in
  `formoxus/tests/ui/`).
- [ ] e2e forms: `examples/src/test_forms.rs` (`MustAgree` and
  `MustAgreeNoValidate`, around lines 242–287).
- [ ] Suite: `submissions.rs` (223, 236), `constraint_attrs.rs` (250–288).
- [ ] Macro unit tests in `field.rs` (around 490–512).

### 3f. Tests (Claude)

- [ ] `every_legal_key_parses` and `an_unknown_key_lists_every_legal_one`
  ([field.rs:397](../formoxus-macros/src/form/field.rs#L397),
  [:418](../formoxus-macros/src/form/field.rs#L418)) move from `LEGAL_KEYS` to
  the table.
- [ ] New goldens: an owned key, a duplicate key, `class` with `class_plus`, a
  quoted key the table knows, an unknown key with a suggestion.
- [ ] `tests/suite/author_attrs.rs` switches from `with_attrs` to `form!`
  keys and quoted keys, and keeps pinning where attributes land per widget.
- [ ] Acceptance: every existing golden byte-identical except the deliberate
  grammar change; `just e2e` green.

---

## Step 4: emit only valid attributes, and the custom-widget trait

Closes issue #4. Decisions 6, 10, and the custom-widget section of the note.

- [ ] Map each widget (and `InputType`) to its `FieldControl`, at expansion time
  for named widgets.
- [ ] **Compile-time**: `form!` asserts `Attr::is_allowed(field_type, control)`
  per attribute. Refused → error naming the attribute and the widget.
- [ ] **Runtime**: `constraint_attributes` emits only where `is_valid_on`
  (so `min` stops appearing on `type="text"`; it is still checked).
- [ ] `RadioGroup` routes each attribute to the radios or the `<fieldset>` by
  asking `is_valid_on`.
- [ ] The **`Widget` trait** for custom widgets (`const ATTRS: AttrSet`,
  `AttrSet::ANY` allowed); custom widgets receive attributes.
- [ ] The **dioxus-html cross-check test** (design note, "TODO: cross-check").

## Then, in order

1. **Two enums for input types** (decision 6): `WidgetType::Input` takes a
   smaller enum of what `Input` renders; `Input` becomes the router for every
   `<input>` (decision 5), and `WidgetType::Checkbox` goes away.
2. **Reshape `FieldProps`**: `required` and `aria_invalid` become attributes,
   kept typed until emission; leave room for a blur-validation callback.
3. **`form!` is the only door** (decision 7): builders, `FormField`'s fields and
   `ScalarWidget` behind `#[doc(hidden)] __private`; about 210 suite call
   sites move to `form!` or `__private`. Last, because it touches the most.
4. **Split `fields.rs`**, if the value-kind half is still large (it mostly
   left in step 2).
5. Then back to the MVP list: #9 `aria-describedby` (with `<label for>`),
   help text, per-field validators, and onward
   (`.claude/memory/mvp_scope.md`).
