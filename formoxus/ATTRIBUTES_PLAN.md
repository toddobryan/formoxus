# Attributes: the build order from here

A working checklist for the attribute-table work (C6, which absorbed issue #4).
**The reasoning behind every decision is in
`.claude/memory/attribute_rules_design.md`**, numbered; this file is just the
order to do things in and where to do them. Claude keeps it current when asked.

> **Status, 2026-10-11.** Step 3 is DONE, every box (3f's last box today).
> 3d's widget side finished today too (steps 4–10). Typed keys on an enum
> field are allowed where they make sense (3f, DONE). Deferred: the
> `RadioGroup` author-attribute grammar, and custom widgets getting author
> `class`. **Step 4 (emit only valid attributes, the `Widget` trait) is broken
> down below but not started; it opens with decisions D0–D4.** D0 (2026-10-10)
> (parts) is ADOPTED. **2026-10-10: three steps now come FIRST, in "Before
> Step 4": P1 labels beside their inputs (#9), P2 `BuiltinWidget`, P3
> configurable classes (#10).** **P1 DONE 2026-10-11 (#9 closed).** P2 is
> broken down; P2-a and P2-b DECIDED 2026-10-11 (`Element` + narrowed
> `FieldControl`; decision 5's `Input` router folded in). **2026-10-12: the
> first two P2 boxes DONE** (`Element`/`FieldControl` in
> `formoxus-spec/src/elements.rs`; the 73 rows sorted). **By the end of
> 2026-10-12, boxes 1–5 DONE** (also `RenderedInputType`, and `Input` routing
> every `<input>`). **2026-10-13: `BuiltinWidget` and its unit tests DONE.**
> **The macro side DONE the same day, and the runtime side: P2 IS DONE.**
> Next: P3 (configurable classes), BROKEN DOWN 2026-10-13; it opens with
> decisions P3-c to P3-h.
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

### 3a. The table grows (`formoxus-spec/src/attrs.rs`)

- [x] **Generate `variant_name()` and `from_variant_name()`** (done by Todd) in the
  `attributes!` macro ([attrs.rs:83](../formoxus-spec/src/attrs.rs#L83)), next
  to `from_name` ([:159](../formoxus-spec/src/attrs.rs#L159)).
  `variant_name` is `stringify!($variant)`; `form!` turns `max_length` into
  `MaxLength` with `heck` and looks it up here.
- [x] **New rows** (DONE 2026-10-04: 95 rows, 44 alphabetized then 51 ARIA
  alphabetized; see the design note, decision 15, for what was left out and
  why) in `attributes! { … }`
  ([:195](../formoxus-spec/src/attrs.rs#L195)):
  - `Class` (`owner: Merged`, a token list) and `ClassPlus` (emits HTML
    `class`, like `Required`/`RequiredTrue` share `required`).
  - `Style` and `StylePlus`, the same way, with a declaration list.
  - The ARIA attributes, about 48: `owner: Author`, valid on every control.
    **`aria-invalid` (already a row, [:320](../formoxus-spec/src/attrs.rs#L320))
    and `aria-describedby` are owned by formoxus.**
  - The common author attributes, at least `autocomplete`, `readonly`, `rows`,
    `cols`, `inputmode`, `title`, `autofocus`. The full list, with where each is
    valid, is in `.claude/memory/html_attributes_reference.md`.
- [x] ~~**REMINDER (Todd, 2026-10-04): add `TokenList` and `Declarations`
  variants to `AttrValue`**~~ CLOSED 2026-10-07: both are built as the one
  `AttrValue::List`, and `AttrType` keeps the two kinds apart; see 3d.

### 3b. The macro parses attributes generically (`formoxus-macros/src/form/field.rs`)

> **DONE 2026-10-06** (Claude, delegated). The macro crate builds and its 110
> unit tests pass; the workspace does not, until 3c gives the runtime
> `AttrKey` keys and an `AttrValue::String` arm (`fields.rs:271`). Beyond the
> plan: an identifier key spelled the HTML way (`maxlength:`) gets "write
> `max_length`"; `required:` (owned) points at `required_true`; `label` is a
> `LitStr` now (Todd's `Option<String>`), no longer any expression; the
> `takes_*` wrappers in `field_kind` stay until their unit tests move (3f).

- [x] **Replace the per-key fields** of `field_body!`
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
- [x] **Routing in `ParsedAttrs`' insert**, in this order:
  1. An identifier key → `from_variant_name` of its PascalCase form. Unknown
     → "unknown key", with a did-you-mean (`suggest.rs`) and the legal list.
  2. Owned (`owner: Formoxus`) → compile error naming it.
  3. Already present → "given twice", pointing at the second.
  4. `Class` with `ClassPlus` present, or the reverse (same for `Style`) →
     error: only one per field.
  5. A **string-literal** key (`"hx-get"`) → `NonStd`, passed through. If the
     table knows that HTML name (`"maxlength"`), refuse it: "write
     `max_length:`".
- [x] **Parse each value by the row's `attr_type()`**: `Int`/`Bound`/`String`
  → `Expr`; `Regex` → `LitStr` (and keep today's `regress` compile check);
  `Flag` → **no value at all** (a bare key); lists → the list form.
- [x] **`constraints_tokens`** ([:83](../formoxus-macros/src/form/field.rs#L83))
  becomes one loop over the map, emitting `(AttrKey, AttrValue)` per entry
  (decision 16): `Std(Attr::X)` with the value its row's type gives, or
  `NonStd("hx-get")` (the macro's `String` written out as a literal) with
  `AttrValue::String`. The loop collects `TokenStream`s, not runtime values:
  the values are the author's expressions, which only exist in the expansion.
- [x] **`type_checks`** ([:152](../formoxus-macros/src/form/field.rs#L152))
  becomes one loop emitting `assert!(applies(Attr::X, shape), msg)` per entry.
  Make `field_kind::applies` public for it
  ([field_kind.rs:42](../formoxus/src/field_kind.rs#L42)); the `takes_*`
  wrappers ([:54](../formoxus/src/field_kind.rs#L54)–[:75](../formoxus/src/field_kind.rs#L75))
  can then go. **Generate each message from the key and the row's `for:`**
  (`Text` → "a String field", `Int | Float` → "a number field") so the goldens
  stay byte-identical. The bound checks and min ≤ max stay, keyed on `Min`/`Max`.
- [x] **The `required: false` check** in `Parse`
  ([:319](../formoxus-macros/src/form/field.rs#L319)) goes: `required_true` is
  a bare flag, so there is no `false` to write.

### 3c. One runtime map (`formoxus/src/fields.rs`, `formoxus/src/form/spec.rs`)

> **DONE 2026-10-06** (Todd; Claude did the two builders → `with_attrs`, the
> macro's emitted names, and the tests). One `FieldAttrs(IndexMap<AttrKey,
> AttrValue>)`; `html_attributes` emits in the author's order.

- [x] **Merge `AllAttrs` ([fields.rs:56](../formoxus/src/fields.rs#L56)) and
  `FieldAttrs` ([:75](../formoxus/src/fields.rs#L75)) into one `FieldAttrs`**:
  ONE `IndexMap<AttrKey, AttrValue>` (decision 16), so the author's order holds
  across table and quoted keys. `get(Attr)`/`contains(Attr)` keep their
  signatures and wrap in `AttrKey::Std` inside; `check` still walks
  `Attr::ALL`, so it never sees a `NonStd` entry. `to_attributes` emits both
  kinds. (`AttrKey` and `AttrValue::String` already exist, Todd 2026-10-06.)
- [x] **`FieldSpec`**: `constraints` and `attrs`
  ([spec.rs:40](../formoxus/src/form/spec.rs#L40),
  [:54](../formoxus/src/form/spec.rs#L54)) become one field;
  `with_constraints` and `with_attrs`
  ([:144](../formoxus/src/form/spec.rs#L144), [:149](../formoxus/src/form/spec.rs#L149))
  become one builder.
- [x] **`distribute_specs`** copies the one map
  ([fields.rs:589](../formoxus/src/fields.rs#L589)–[:590](../formoxus/src/fields.rs#L590)).
- [x] **`render`** ([:449](../formoxus/src/fields.rs#L449),
  [:453](../formoxus/src/fields.rs#L453)) hands down table attributes plus
  extras; `ScalarWidget`'s merge
  ([scalar.rs:39](../formoxus/src/widgets/scalar.rs#L39)–[:46](../formoxus/src/widgets/scalar.rs#L46))
  keeps "constraint attributes first, caller's last".

### 3d. `class` and `class_plus`, `style` and `style_plus`

> **Decided 2026-10-07** (Todd; reasoning in the design note, open question 2):
>
> - **Attributes become `Vec<Attribute>` inside the widget**, not in
>   `FormField::render`. Only the widget knows its own classes, and a spread
>   `class` is *duplicated*, not merged. Probed 2026-10-07: `class_plus:
>   ["wide", "dark"]` rendered `<input class="fx-control fx-input" … class="wide dark"/>`.
> - **No separate `TokenList`/`Declarations` values.** Both are
>   `AttrValue::List(&'static [&'static str])`; the macro writes each style
>   declaration out whole (`"font-size: 20px"`), and the widget joins with `" "`
>   or `"; "` by the `Attr`. `AttrType::List`
>   ([attrs.rs:19](../formoxus-spec/src/attrs.rs#L19)) is unused and goes.
>   This closes the 3a reminder.
> - **`RadioGroup` is on hold** until Todd has looked at examples. Maybe
>   faux attributes `group_class`/`input_class`.

#### The grammar

```rust
class_plus: [dark, centered, my_class, "real_underscore"],
style: {
    color: red,
    font_size: 20px,
    justify_content: space_between,
    position: static,
    margin: -1px 0,
    font_family: "'Inter', sans-serif",
},
```

- **One rule everywhere:** a bare identifier turns `_` into `-`, in class names,
  style properties and style values alike (`my_class` → `my-class`,
  `space_between` → `space-between`). Quote to keep a real underscore. BEM's
  `card__title` has to be quoted.
- Keywords are allowed bare (`position: static`), and raw identifiers are unraw'd
  (`r#type` → `type`).
- **A style value** is a quoted string, used verbatim, or a run of bare items up to
  the next `,`, joined with single spaces. An item is an identifier or a number
  (Todd leaning yes on numbers, 2026-10-07):
  - `20px` is ONE token, a `LitInt` with suffix `px`; `1.5rem` is a `LitFloat`
    with suffix `rem`. `lit.to_string()` gives back `"20px"`.
  - `50%` is TWO tokens, `LitInt(50)` then `Punct('%')`. Glue a `%` onto the
    number before it. (`50 %` lexes identically, so it comes out `50%` too,
    which is harmless.)
  - `-1px` is `Punct('-')` then `LitInt`. Glue a `-` onto the number after it.
  - **`2em`, `1.5em` and `2ex` never reach the macro**: Rust's lexer reads the `e`
    as an exponent and fails with "expected at least one digit in exponent". They
    must be quoted. Put this in the grammar docs, because the error is Rust's.
  - Anything else gets an error pointing at that token: "quote the whole value:
    `font_size: \"2em\"`". That covers `#fff`, `rgb(…)`, `var(--x)`, `calc(…)`,
    `!important`, and anything with a comma in it.
- **Quotes inside a quoted value pass through untouched.** CSS accepts `'Inter'`
  as readily as `"Inter"`, and Dioxus SSR escapes both kinds inside an attribute
  value (`askama_escape::Html`), so nothing needs converting.

#### Hints for the parser (Todd is writing it)

1. **Where:** `parse_value`
   ([field.rs:427](../formoxus-macros/src/form/field.rs#L427)) picks the grammar
   by `attr_type()`. Split the shared list arm in two: `TokenList` uses
   `bracketed!`, `Declarations` uses `braced!`.
2. **Keep `entry_tokens` small:** normalize at parse time and store the results as
   `LitStr`s built with `LitStr::new(&text, span)`. `AttrSource::List(Vec<LitStr>)`
   ([:148](../formoxus-macros/src/form/field.rs#L148)) then stays as it is, and the
   `todo!` ([:133](../formoxus-macros/src/form/field.rs#L133)) becomes
   `quote! { #attrs::AttrValue::List(&[#(#items),*]) }`. For style, each `LitStr`
   is the finished `"font-size: 20px"`. Use the span of the item (or property) it
   came from, so a later error lands on the right token.
3. **The syn pieces:**
   - `use syn::ext::IdentExt;` gives `Ident::parse_any` (accepts keywords) and
     `input.peek(Ident::peek_any)`.
   - `ident.unraw().to_string().replace('_', "-")`.
   - `input.peek(LitStr)`, `input.peek(LitInt)`, `input.peek(LitFloat)`,
     `input.peek(Token![%])`, `input.peek(Token![-])` to tell items apart.
   - A run: `while !input.is_empty() && !input.peek(Token![,]) { … }`.
   - Each class name or declaration is one item of a
     `Punctuated::<_, Token![,]>::parse_terminated`, over a small type with its own
     `Parse` impl, so trailing commas come free.
4. **The test that will break:** `every_legal_key_parses`
   ([:659](../formoxus-macros/src/form/field.rs#L659)) writes `#ident: ["x"]` for
   every list type. `Declarations` needs `{ x: y }`.

#### The checklist

- [x] Parser: the grammar above. DONE 2026-10-07: class list (Todd), style
  block (Claude, delegated, with these calls made on Todd's behalf):
  - [x] `class_plus: []` and `style_plus: {}` are errors, "should be omitted"
    (Todd). `class: []` stays legal: it strips formoxus's classes.
  - [x] A hyphenated bare name (`text-center`, `font-size`, `space-between`)
    gets "write `text_center`, or quote it"; a class list adds a comma
    reminder, since `[text -mt-4]` lexes identically. A leading `-`
    (`-mt-4`, `--gap`, `-webkit-…`) gets "must be quoted".
  - [x] In a value, `-` before a NUMBER starts a negative one: `auto -1px` is
    two items. Only `-` before a name is a hyphen error.
  - [x] `!important` is accepted, last in a value only.
  - [x] A property given twice is an error, compared after normalizing, so
    `font_size` and `"font-size"` collide.
  - [x] A missing `:` after a later property says "if it belongs to the value
    before it, quote that whole value" (the `font_family: Inter, serif` trap).
  - [x] `;` between declarations gets "separate declarations with `,`".
  - [x] Anything else in a value (`#fff`, `rgb(…)`, `/`) gets "quote the whole
    value: `color: \"…\"`". The `…` is literal: rebuilding the author's text
    from tokens gets the spacing wrong (`# fff`).
- [x] `entry_tokens`: the `todo!` becomes `AttrValue::List` (Todd).
- [x] Drop `AttrType::List` (Todd).
- [x] **Widgets build their own attributes, and `required`/`aria_invalid` leave
  `FieldProps`** (Todd; folded together 2026-10-08, since both touch every
  widget's signature). Run `cargo test --workspace` after each step.

  **Part A: the typed map reaches the widgets. DONE `1d1722b`, no HTML change.**
  - [x] 1. `html_attributes` → `FieldAttrs::to_attributes(field_type)`;
    `int_bound` a free fn (its panic no longer names the field).
  - [x] 2. `ScalarWidget` takes `field_attrs: FieldAttrs`.
  - [x] 3. Every widget takes `field_type` + `field_attrs` and calls
    `FieldAttrs::merge_with_attrs(field_type, attrs)`; the `Custom` arm too
    (pinned by `a_custom_widget_receives_the_fields_attributes` in
    `tests/suite/author_attrs.rs`).

  **Part B: class and style.**
  - [x] 4. **One resolved `class`, written as a LITERAL.** DONE 2026-10-08 (Todd;
    tests Claude). Beyond the plan: `VariantSet` gained `field_attrs`, copied
    from the spec in `distribute_specs`, so an enum field's attributes now reach
    its `<select>` (they were parsed and dropped before); `VariantSelect`
    passes `FieldType::Text` inline rather than taking a `field_type` prop. No
    existing test changed, because none set an author `class`. New:
    `class_and_class_plus_on_the_{input,textarea,checkbox,select,variant_select}`
    and `on_a_radio_group_an_author_class_goes_nowhere_yet` in
    `tests/suite/author_attrs.rs`, and
    `a_class_plus_from_the_macro_appends_to_the_inputs_class` in
    `tests/suite/form_macro.rs`; all mutation-checked.
    1. `FieldAttrs::class(&self, base: &str) -> String`: `Class` replaces
       `base`, `ClassPlus` appends with a space, neither gives `base`.
    2. Each widget writes `class: field_attrs.class("fx-control fx-input")`
       where it has the fixed string now: `input.rs`, `textarea.rs`,
       `checkbox.rs`, `select.rs`, `variant_select.rs`. (`RadioGroup` on hold.)
    3. `to_attributes` skips `Class`/`ClassPlus`; `merge_with_attrs` calls it,
       so that is the one place.

    A literal rather than the spread because the spread always comes after the
    literals, so a spread `class` would move in every tag and break the suite's
    exact-markup tests. Expect a test or two to change where an author's
    `class` used to be a duplicate attribute: that duplication is the bug.
  - [x] 5. **Style.** `to_attributes` joins `Style`/`StylePlus` with `"; "`.
    Formoxus emits no style of its own, so `StylePlus` acts like `Style` for now.
    DONE 2026-10-08 (Todd; tests Claude, mutation-checked): four `style*` tests
    in `tests/suite/author_attrs.rs` (`;` join, one declaration, `style_plus`
    = `style`, a radio group's style on its `<fieldset>`) and
    `a_style_from_the_macro_reaches_the_input` in `tests/suite/form_macro.rs`.

  **Part C: `required` and `aria_invalid` into the map.**
  - [x] 6. **Build the map at render time.** DONE 2026-10-08 (Todd, as
    `FieldAttrs::with_owned_attrs(widget, required, invalid, attrs)`; 4 unit
    tests in `fields.rs`, Claude, mutation-checked). **Expected red until
    steps 7–8:** `choices::the_group_is_a_field_and_a_radio_group` fails
    because the spread now puts `required` on `RadioGroup`'s `<fieldset>`
    (step 8), and inputs render `required=true` twice (literal + spread) until
    step 7 deletes the literal. Something like
    `FieldAttrs::with_owned(required: bool, invalid: bool, rest: &FieldAttrs)`,
    inserting `Required` (`Flag`) and `AriaInvalid` (`String("true")`) FIRST,
    then copying `rest`. Call it in `FormField::render` instead of
    `self.attrs.clone()`. Both are known only at render time (`ctx.required`,
    the errors), so the macro's map never holds them. First keeps `required`
    where the suite pins it (`choices.rs:318`, `enums.rs:208`).
    **Leave `Required` out when the widget is `Checkbox`**: HTML `required` on
    a checkbox means "must be ticked".
  - [x] 7. **Widgets read the map, not the props.** DONE 2026-10-08 (Todd),
    including `RadioGroup` and `VariantSelect`, which take their literals out
    ahead of steps 8–9. **Expected red until then:**
    `choices::every_radio_is_required_and_nothing_offers_absence` and
    `choices::the_group_is_a_field_and_a_radio_group` (step 8: the radios have
    no `required` and the `<fieldset>` does); and
    `enums::an_unchosen_required_enum_offers_no_way_back_to_unchosen` (step 9:
    `VariantSet` still passes its raw `field_attrs`, so the variant `<select>`
    loses `required` and offers `--none--` on a required enum).
    1. Delete the literal `required,` and `aria_invalid,` in each widget; the
       spread emits them now.
    2. `field_attrs.contains(Attr::Required)` for the ` *` marker and
       `Select`'s placeholder choice.
    3. `Checkbox`: `required_true` → `field_attrs.contains(Attr::RequiredTrue)`;
       drop the prop from `Checkbox`, `ScalarWidget` and `render`.
    4. `reject_if_not_required` (`scalar.rs`) checks the map.
  - [x] 8. **`RadioGroup`: `required` on each radio, `aria-invalid` on the
    group.** DONE 2026-10-09 (Claude, delegated): `FieldAttrs::without(attr)`
    (unit-tested), `required` a literal on each radio, `role: "radiogroup"`
    on the fieldset. Mutation-checked; e2e 29 green, including
    `an_unanswered_radio_group_is_blocked`. Rewritten 2026-10-09 from the specs (both are judgments about the
    GROUP; they differ only in where each spec lets you say so):
    - HTML has `required` only on `<input>`, never `<fieldset>`, and applies it
      per group: one required radio makes every radio in the group "suffering
      from being missing" until any is checked. So it stays a literal on EACH
      radio (consistent, and each announces "required" on focus), read from
      `field_attrs.contains(Attr::Required)`, and leaves the fieldset's copy
      (`shift_remove`, which keeps the order).
    - WAI-ARIA 1.2 lists `aria-invalid` (and `aria-required`) for `radiogroup`
      and NOT for `radio`, whose only supported states are `aria-posinset` /
      `aria-setsize`; elsewhere use is "not defined". A bare `<fieldset>` is
      role `group`, also not listed. So the fieldset gets `role: "radiogroup"`
      as a literal right after `class`, and `AriaInvalid` simply stays in the
      spread, which already lands there.
    - The old reason for per-radio `aria-invalid` (the `input[aria-invalid] + *`
      test in `widgets.rs`) renders a TEXT input, not radios; it is unaffected.
    - Claude's tests:
      `the_group_is_a_field_and_a_radio_group` expects
      `<fieldset class="fx-form-field fx-radio-group" role="radiogroup">`, and
      `an_invalid_radio_group_marks_the_group_not_the_radios` (choices.rs).
  - [x] 9. DONE 2026-10-09 (Todd). `VariantSet::render` builds the map with
    `with_owned_attrs(&WidgetType::Select, ctx.required, self.has_errors_here(), …)`.
    The first try passed `has_errors()`, which counts the chosen variant's
    fields too, so a bad `radius` marked the variant `<select>` invalid;
    pinned by `enums::a_bad_field_in_the_variant_does_not_mark_the_choice_invalid`.
    That led to splitting `FormMember::has_errors` into `has_errors_within`
    and `has_errors_here` (Claude, delegated; see
    `.claude/memory/errors_here_vs_within.md`).
    **`VariantSelect`** gets `field_attrs: FieldAttrs`, built in
    `VariantSet` ([variant_set.rs:185](../formoxus/src/members/variant_set.rs#L185))
    as in step 6; apply step 7 to it. It passes `FieldType::Text` to
    `to_attributes` (no bounds). Its legend marker keeps reading `ctx.required`.
  - [x] 10. **Delete the fields.** DONE 2026-10-09 (Todd, including item 6;
    Claude wrote item 1's doc comment on `with_owned_attrs`). No HTML change:
    suite and e2e green unchanged. (Refreshed 2026-10-09, line numbers
    as of then.) `required` and `aria_invalid` leave `FieldProps`; what is
    left is `path`, `label`, `errors`.
    1. **The struct** ([types.rs:106](../formoxus/src/widgets/types.rs#L106)):
       delete both fields. **Keep the `aria_invalid` doc comment's reasoning**
       (absent is the only neutral state; `"false"` asserts "checked and
       passed"): move it onto `FieldAttrs::with_owned_attrs` in `fields.rs`,
       which is now the thing that only ever inserts `"true"`. The `required`
       comment's checkbox caveat already lives there as code.
    2. **`field_class()`** ([types.rs:141](../formoxus/src/widgets/types.rs#L141)):
       key off `!self.errors.is_empty()`. That removes the contradictory
       hand-built case (errors but no `aria_invalid`, or the reverse).
       `field_class_plus` follows for free.
    3. **The two builders:** `FormField::render`
       ([fields.rs:506–508](../formoxus/src/fields.rs#L506)) and
       `VariantSet::render`
       ([variant_set.rs:200–202](../formoxus/src/members/variant_set.rs#L200)):
       delete the `required:` and `aria_invalid:` lines. `RenderCtx::required`
       (`members.rs`) is a different thing and STAYS: it is where
       `with_owned_attrs` and `VariantSet`'s legend marker get it from.
    4. **The six destructures:** delete `required: _,` and `aria_invalid: _,` in
       `input.rs:30–32`, `textarea.rs:36–37`, `checkbox.rs:29–30`,
       `select.rs:45–47`, `radio_group.rs:53–55`, `variant_select.rs:40–42`.
    5. **The checkbox comment** at `checkbox.rs` ~32–36: it says `props.required`
       is "deliberately dropped" and that `required_true` "is a separate prop".
       Both stopped being true at step 7: there is no `props.required` to
       drop, `with_owned_attrs` keeps `Required` off a checkbox, and
       `required_true` is an entry in the map.
    6. **Claude, after:** the three `ScalarWidget` literals in
       `tests/suite/widgets.rs` (~185, ~211, ~547) lose `required: true,` and
       `aria_invalid: None,`. None of their assertions read either.
       (`examples/src/bin/widget_matrix.rs` needs nothing: it builds its own
       `OneFieldProps`, not a `FieldProps`.)

    Expect no HTML change: since step 7 every widget reads the map, so these
    fields are already dead weight. A clean `cargo build` after 1–5 is most
    of the proof.

  Deferred: custom widgets after step 10 see `required` only by name in the
  merged `Vec<Attribute>`; `field_attrs` in `WidgetProps` waits (Todd,
  2026-10-08). `form!`'s `custom(…)` closure drops `attrs` entirely
  ([widget.rs:159](../formoxus-macros/src/form/widget.rs#L159)); that is
  "custom widgets receive attributes" in **Step 4** below (not step 4 here).
  **Since step 4, an author `class` reaches neither a custom widget nor
  `RadioGroup`:** `to_attributes` skips it and neither calls
  `FieldAttrs::class`. That belongs with the Widget trait's declared default
  classes (design note) and the `RadioGroup` hold. Issue #7
  (`aria-invalid="false"`) is unaffected; later.
- [x] Tests (Claude): the parser's unit tests (DONE 2026-10-07: the class-list
  and style-block tests in `field.rs`, e.g. `a_class_list_takes_bare_and_quoted_names`,
  `a_style_block_takes_bare_and_quoted_values`), and a suite test pinning ONE
  `class` attribute per element (DONE with step 4, above).

### 3e. The grammar change: `required: true` → `required_true`

> **DONE 2026-10-06** (Claude). `form_required_false` deleted; the other two
> goldens differ only by the key name and the caret moving onto the bare flag.
> Every other golden is byte-identical. Suite 327, lib 86, macros 110, e2e 29.

Breaking, so it all moves together. Claude updates the tests side.

- [x] Library: the `RequiredTrue` row is a `Flag`, so 3b already parses it bare.
- [x] Goldens: `form_required_false` (delete: nothing to write),
  `form_required_on_a_string`, `form_required_on_an_optional_bool` (in
  `formoxus/tests/ui/`).
- [x] e2e forms: `examples/src/test_forms.rs` (`MustAgree` and
  `MustAgreeNoValidate`, around lines 242–287).
- [x] Suite: `submissions.rs` (223, 236), `constraint_attrs.rs` (250–288).
- [x] Macro unit tests in `field.rs` (around 490–512).

### 3f. Tests (Claude)

- [x] `every_legal_key_parses` and `an_unknown_key_lists_every_legal_one`
  ([field.rs:397](../formoxus-macros/src/form/field.rs#L397),
  [:418](../formoxus-macros/src/form/field.rs#L418)) move from `LEGAL_KEYS` to
  the table. DONE: `legal_keys()` reads `Attr::ALL`. The second test is gone
  with the list it checked (2026-10-07: unknown-key messages no longer list
  ~90 keys); `an_unknown_key_names_itself` and
  `an_unknown_key_gets_the_quoting_rule` cover the message now.
- [x] New goldens: an owned key, a duplicate key, `class` with `class_plus`, a
  quoted key the table knows, an unknown key with a suggestion. DONE:
  `form_owned_attribute`, `form_attribute_given_twice`,
  `form_class_with_class_plus`, `form_quoted_key_the_table_knows`,
  `form_unknown_key_near_miss` (plus `form_unknown_key_html_spelling` and
  `form_hyphenated_bare_class`).
- [x] `tests/suite/author_attrs.rs` switches from `with_attrs` to `form!`
  keys and quoted keys, and keeps pinning where attributes land per widget.
  DONE 2026-10-09 (Claude). Four tests stay hand-built, each saying why: the
  `From` impl, `custom(…)` (drops attrs), and the two raw-`maxlength` hazards
  `form!` refuses. (A fifth, `class` on an enum field, moved to `form!` once
  the finding below was fixed.) **Finding:** `form!` refuses
  `class`/`class_plus` (any typed key) on an ENUM field at compile time, since
  the table's `for:` column has no enum kind; only quoted keys get through to
  the variant `<select>`. **Todd, 2026-10-09: allow them where they make
  sense.** Proposed: on an enum field allow rows with `validated: false`
  (author + `class`/`style`), refuse the validated ones (nothing is
  submitted). Split the enum out of `field_kind`'s `Kind::Other`, which also
  covers struct and list, whose containers render nothing that takes
  attributes, so loosening `Other` wholesale would drop them silently. Then
  Claude moves `class_and_class_plus_on_the_variant_select` to `form!` and
  adds a golden for a constraint key on an enum field.
  **DONE 2026-10-09.** `Kind::Enum` (Todd): an enum takes
  `!attr.validated() && attr.is_valid_on(FieldControl::Select)`; struct and
  list stay `Kind::Other`, refused. `form!` emits a second assert per key,
  `!is_enum || applies`, with an enum-specific message from `enum_refusal`
  (validated → "submits no value to check"; not valid on `<select>` → "does
  not take it"). Trap: facet types `Option<T>` as `UserType::Enum`, so
  `is_enum` asks `kind` (which peels the `Option`), not `shape.ty`; caught by
  `form_min_on_a_string`. `multiple` needs no exclusion: it is owned, refused
  at parse. Goldens: `form_constraint_on_an_enum`,
  `form_constraint_on_an_optional_enum`, `form_placeholder_on_an_enum`.
- [x] Acceptance: every existing golden byte-identical except the deliberate
  grammar change; `just e2e` green. MET as of `534057b` (goldens pass; e2e 29).

---

## Before Step 4: the markup settles first (decided 2026-10-10)

Todd pulled two later items forward, because the parts that Step 4 names are
read off the markup, and both of these change the markup:

1. **P1. Labels beside their inputs** (`<label for>`, issue #9's real fix).
2. **P2. `BuiltinWidget` in `formoxus-spec`**, describing the widgets AFTER P1,
   so the parts are final when they are written down (moved here from 4a).
3. **P3. Configurable default classes** (issue #10), keyed by the parts P2
   defines.

Then Step 4, whose 4a–4d get rewritten for parts. Line numbers in this section
are from `28e8163`. Decision record: `.claude/memory/attribute_rules_design.md`
("Labels beside their inputs" and "Configurable classes").

### P1. Labels beside their inputs

**Why:** a wrapping `<label>` names its input from ALL its text, so the error
list inside it joins the accessible name ("Title This field is required.").
`aria-describedby` alone cannot fix that; it adds a description and removes
nothing. Todd's design is in his comment on issue #9.

**Target markup** (DECIDED: every `<label>` uses `for`, radios included, Todd
2026-10-10, "for consistency"):

```text
input / textarea / select / an enum's variant select
  div.fx-form-field[.fx-invalid]
    label.fx-field-label  for=ID          (text + span.fx-required)
    input#ID.fx-control.fx-input          aria-describedby=ID-errors, only while there are messages
    ul.fx-field-errors#ID-errors

checkbox (box, then text)
  div.fx-form-field
    input#ID.fx-control.fx-checkbox
    label.fx-field-label  for=ID
    ul.fx-field-errors#ID-errors

radio_group
  fieldset.fx-form-field.fx-radio-group  role=radiogroup
    legend.fx-legend.fx-field-label
    div.fx-choice                         one per choice, N = its index (P1-b, P1-c)
      input#ID-N.fx-control.fx-radio      name=PATH, required on each as today
      label.fx-choice-label  for=ID-N
    ul.fx-field-errors#ID-errors          (aria-invalid + aria-describedby land on
                                           the fieldset, through the spread)
```

The `span.fx-field-label` inside today's wrapping label goes away: the
`<label>` itself carries `fx-field-label`. The error list stays the input's
next sibling (only the checkbox puts its label between), which Bootstrap's
`.is-invalid ~ .invalid-feedback` needs.

**Ids** (DECIDED in principle, Todd 2026-10-10: "a unique form id, prefixed to
the path"): `ID = {form_id}-{path}`. Path characters `.`, `[`, `]`, `$` are
legal in an id (they need escaping only in a CSS selector, and tests select by
role, never `#id`). The form id must come out the same in the server render
and the client's hydration, so no runtime counter.

Decided (Todd, 2026-10-10):

- **P1-a. Each form gets a RANDOM id, picked in `use_form`** (Todd,
  2026-10-10; replaces a same-day title/model/instance scheme). 8 characters
  from `[A-Za-z0-9]`, so 62^8 ≈ 2.2 × 10^14 ids; 100 forms on one page
  collide with odds of about 1 in 40 billion. Element ids are
  `{form_id}-{path}`. Nothing in `form!`, nothing at render, no title rule,
  and the same form repeated per row just works.
  **Picked through `dioxus_fullstack_core::use_server_cached`**, so a
  fullstack page's hydration reuses the server's id instead of rolling its
  own. A NEW unconditional dependency, `dioxus-fullstack-core = "0.7.10"`
  (pin with `dioxus`), and NO formoxus feature: the hook switches on that
  crate's own `server`/`web` features, which `dioxus-server` and
  `dioxus-web/hydrate` turn on, so Cargo's feature unification picks the
  right behaviour; with neither (desktop, client-only, LiveView, the suite)
  it just runs the closure.
  **SPIKED 2026-10-10 (Claude, throwaway worktree, deleted):** the suite
  (343) passed with the hook in every `use_form`; `cargo tree` showed the
  plain build gets no feature and the examples' server build gets `server`;
  in a browser an element created AFTER hydration carried the server's id
  (`server-2`), and the control run with plain `use_hook` showed the
  mismatch (`server-2` in the DOM, `client-0` in the client). Notes for the
  real version: (1) start the id with a letter (or a fixed prefix) so `#id`
  needs no CSS escaping; (2) the random source must work on
  `wasm32-unknown-unknown` without `getrandom`'s `--cfg` setup, and need not
  be cryptographic; (3) suite assertions compare `for` with `id`, never a
  literal id; (4) a form rendered in pieces is fine, the id is on the `Form`.
- **P1-b. A radio's id is `ID-N`**, by choice index, not by value (a value
  can hold a space; an id cannot).
- **P1-c. Each radio and its label get a `div.fx-choice` wrapper**; the
  `fx-choice` class moves there from today's `<label>`, and the label gets
  `fx-choice-label`. `radio_group` gains a `div` part.

Checklist:

- [x] **The form id** (Todd, DONE 2026-10-11): `use_server_cached` in
  `use_form`, kept on `Form` as a `Signal<String>` (so `Form` stays `Copy`),
  handed to `RenderCtx::root` as a plain `String` (resolved once, like
  `label_case`), and `RenderCtx::id(name)` = `{form_id}-{path}` builds the
  element id in ONE place. `FieldProps` carries both: `path` is the field's
  name in the DATA (`name=`, the value store, `Edit`s, the server; identical
  for every copy of a form), `id` its name in the PAGE (different per copy).
  The build is red on purpose until the widget rewrite: the six widgets'
  exhaustive `let FieldProps { … }` patterns do not mention `id` yet.
- [x] **The five single-control widgets** (Todd, DONE 2026-10-11): `input.rs:73`,
  `textarea.rs:41`, `select.rs:52`, `checkbox.rs:36`, `variant_select.rs:46`
  move from `label { class: field_class, … }` to the target above.
  `FieldProps::field_class` keeps naming the wrapper, now a `div`.
- [x] **`FieldErrors`** (Todd, DONE 2026-10-11; `errors_id` is the one place the `-errors` format lives) ([errors.rs:37](src/widgets/errors.rs#L37)) takes the
  id; the control gets `aria-describedby` only when `errors` is non-empty (a
  dangling one is itself a defect). `Id` and `AriaDescribedby` are already
  `owner: Formoxus` rows, so an author still cannot set either.
- [x] **`RadioGroup`** (Todd, DONE 2026-10-11), per P1-b and P1-c: `div.fx-choice`
  around radio-then-label, ids by index, `FieldErrors` with the field id.
- [x] **Docs** (DONE 2026-10-11): `main.css`'s vocabulary comment gains
  `.fx-choice-label` and a "how they nest" section; `examples/README.md`;
  `FieldProps::field_class` (Todd) and the `id`/`path` field docs.
- [x] **Tests that flip** (Claude, DONE 2026-10-11): 9 suite tests moved to
  the new markup, through `contains_regex` and a shared `FORM_ID` pattern in
  `tests/suite.rs` (an id differs per render, its FORMAT should not); the radio
  test now also pins index ids, `for` pairing, and radio-before-label order.
  The 29 e2e tests needed nothing: they find fields by role and name.
- [x] **New tests** (Claude, DONE 2026-10-11): `with_owned_attrs` unit tests
  for `aria-describedby` (mutation-checked); e2e
  `an_error_describes_the_field_without_renaming_it` (name stays "Name" after
  an error; `aria-describedby` absent while clean and resolving to the message
  after, in the live DOM, which also proves the random id survives hydration;
  mutation-checked); suite
  `two_copies_of_a_form_get_different_ids_and_the_same_names` (mutation-checked
  against a fixed id).
- [x] **An enum's variant `<select>` is named by its legend** (DONE
  2026-10-11, Claude at Todd's request): the legend gets `id="{id}-legend"`
  (`legend_id`, both ends in `variant_set.rs`) and the select
  `aria-labelledby` through `FieldAttrs::with_default`, so an author's own
  `aria_labelledby` replaces it instead of duplicating it. It had no
  accessible name before (a legend names its fieldset only; axe
  `select-name`). Suite + unit tests mutation-checked; e2e
  `an_enums_variant_select_is_named_by_its_legend` on `/t/enum` green.
- [x] **Close #9** (closed 2026-10-11).

### P2. `BuiltinWidget` in `formoxus-spec`

Broken down 2026-10-11 (Claude); line numbers from `ea76eed`. One
dependency-free description of every built-in widget, so the facts the two
crates now keep in parallel live in one place. The components and
`ScalarWidget`'s dispatch stay in `formoxus`; the facts move.

**What it replaces** (each a hand-kept copy today):

| fact | lives today in |
|---|---|
| `form!` name → widget | `widgets!` table, [widget.rs:63](../formoxus-macros/src/form/widget.rs#L63), mapping names to variant STRINGS (`widget_variant`, :54) |
| which field types it renders | `rule()` + `Rule` ([widget.rs:172](../formoxus-macros/src/form/widget.rs#L172)), `field_kind::WidgetClass` + `renders` ([field_kind.rs:173](src/field_kind.rs#L173)), and `ScalarWidget`'s match arms ([scalar.rs:38](src/widgets/scalar.rs#L38)) |
| takes `choices` | `CHOOSERS` ([widget.rs:104](../formoxus-macros/src/form/widget.rs#L104)) |
| refuses an optional field | the `refusal` match in `WidgetRef::checks` ([widget.rs:230](../formoxus-macros/src/form/widget.rs#L230)), mirrored by `field_kind::is_optional` and `ScalarWidget`'s `reject_if_not_required` |
| not rendered yet | `Rule::NotYet`: `select_multiple`, `checkbox_multiple`, `file` |
| default per field type | `FormField::default_widget` ([fields.rs:406](src/fields.rs#L406)) |
| its parts | nowhere; read off P1's markup (below) |

**Parts, from the markup P1 shipped:**

| widget | parts (element) | form control |
|---|---|---|
| every `<input>` widget | `div`, `label`, `input` | `input` |
| `textarea` | `div`, `label`, `textarea` | `textarea` |
| `select` (and an enum's variant select) | `div`, `label`, `select` | `select` |
| `checkbox` | `div`, `input`, `label` | `input` |
| `radio_group` | `fieldset`, `legend`, `div` (per choice), `input`, `label` | `input` |
| `hidden` | `input` | `input` |

#### Decisions first (Todd)

- **P2-a. DECIDED (Todd, 2026-10-11): option C, two enums, superset and
  subset.** A flat **`Element`**, every element a part can be
  (`Input(InputType)`, `Textarea`, `Select`, `Fieldset`, `Legend`, `Label`,
  `Div`), and a narrowed **`FieldControl`**, the form controls only
  (`Input(InputType)`, `Textarea`, `Select`), with
  `impl From<FieldControl> for Element`. The table's `on:` column and the
  generated `is_valid_on` are keyed on `Element`, so one column answers for
  every part, labels and divs included. What must be a control is typed as
  one: `true_and_on(…)` and a widget's form-control part take `FieldControl`,
  so `true_and_on(Label)` does not compile. `Fieldset` leaves `FieldControl`
  (the "stretch" ends: a radio group's validated attributes go to its
  inputs now). Same shape as decision 6 (all the HTML facts in one enum, what
  formoxus acts on in a smaller one). Rejected: A (one grown, renamed enum;
  loses "is this a control?" from the type) and B (a separate part enum; would
  need a second validity concept beside `on:`). **Consequence: the 73 rows
  written `on: Input(_) | Textarea | Select | Fieldset` must be sorted into
  truly global (also valid on `Label`/`Legend`/`Div`: `class`, `title`,
  `style`, `lang`, `dir`, `hidden`, `data-*`, most `aria-*`) and form-only
  (`disabled`, `name`, `form`, `readonly`, …); the global list is in
  `.claude/memory/html_attributes_reference.md`. A `Global` shorthand in the
  table macro keeps those rows short.** **AS BUILT (Todd, 2026-10-12):
  `Element` is NESTED, not flat: `Div | Fieldset | Label | Legend |
  Control(FieldControl)`, so the `From` is `Element::Control`, and a row
  reads `on: Control(Input(Color))`, `on: Control(_) | Fieldset`, or `on: _`
  for a global. `_` made the `Global` shorthand unnecessary. In a `const fn`
  write `Element::Control(c)`, not `.into()`.**
- **P2-b. DECIDED (Todd, 2026-10-11): fold decision 5 in now.** `Input` becomes
  the router for every `<input>` formoxus renders: it matches on its input
  type and renders the checkbox code for `Checkbox`, the text code for the
  text-like types. `WidgetType::Checkbox` goes; `form!`'s `checkbox` becomes
  `Input(Checkbox)`. So decision 6's rendered-input enum is `Text` … `Week`
  PLUS `Checkbox` (not `Radio`, which only `RadioGroup` renders, nor `File`,
  `Range`, `Submit`, `Image`, `Reset`, `Button`), and `BuiltinWidget` is
  roughly `Input(RenderedInput) | Textarea | Select | RadioGroup`, plus the
  not-yet `SelectMultiple`, `CheckboxMultiple`, `File`. A checkbox's parts
  differ from a text input's (`div`, `input`, `label`: box first), so
  `parts()` matches on the input type too. About 46 `WidgetType::`
  references move (23 `src`, 15 `tests`, 8 `examples`).

#### Checklist

- [x] **Rename `formoxus-attrs` → `formoxus-spec`** (DONE 2026-10-10, Claude).
- [x] **`Element` and the narrowed `FieldControl`** (Todd), per P2-a: the two
  enums and the `From`; the table's `on:` keyed on `Element`;
  `true_and_on(…)` on `FieldControl`; ~~a `Global` shorthand~~ (`on: _`
  does it). DONE 2026-10-12; Claude rewrote the 95 `on:` rows into
  `Control(…)` and moved the callers and tests.
- [x] **Sort the 73 all-four rows** (DONE 2026-10-12, Todd reviewed): 70 are
  global (`on: _`: the spec globals, `role`, every `aria-*`); exactly three
  are form-only, `disabled`, `form`, `name`, which keep
  `Control(_) | Fieldset` because those are `<fieldset>`'s own attributes in
  the spec. `every_row_is_valid_exactly_where_the_spec_says` now checks
  `Label`/`Legend`/`Div` too (mutation-checked both ways).
- [x] **The rendered-input-type enum** (Todd; decision 6; DONE 2026-10-12 as
  `RenderedInputType` in `formoxus-spec/src/input_types.rs`, with
  `to_input_type()` and a same-name conversion test): the subset of
  `InputType` that `Input` renders (`Text` … `Week`, plus `Checkbox` per
  P2-b; not `Radio`, `File`, `Range`, `Submit`, `Image`, `Reset`, `Button`),
  with a one-line `const fn` conversion onto `InputType` for table lookups.
  It lives in `formoxus-spec` beside `InputType` (`BuiltinWidget` holds it),
  so `InputType`'s doc comment saying "on the `formoxus` side" goes too.
- [x] **`Input` routes every rendered `<input>`** (Todd; decision 5, P2-b;
  DONE 2026-10-12: `Input` matches into private `hidden`/`checkbox`/
  `text_like` bodies sharing `field_label`; `checkbox.rs` folded into
  `input.rs`, `widgets::Checkbox` no longer exported; `rule()` maps
  `checkbox` by name until `BuiltinWidget`; e2e 31 green):
  `Input` matches on its input type and renders `Checkbox` for a checkbox;
  `WidgetType::Checkbox` and its `ScalarWidget` arm go; `form!`'s `checkbox`
  maps to `Input(Checkbox)`; `with_owned_attrs`'s `!matches!(widget,
  WidgetType::Checkbox)` becomes a check on the input type.
- [x] **`BuiltinWidget` in a new `formoxus-spec/src/widgets.rs`** (Todd;
  DONE 2026-10-13, with `ALL`, `is_rendered()` for the not-yet three, and
  `parts()` built from `const { }` slices since a `const fn` call is not
  promoted to `'static`; each text-like `Input` has its own arm).
  `const fn`s, so `form!`'s `const _` asserts can call them:
  `from_form_name(&str) -> Option<Self>` / `form_name()` (as `Attr` has
  `from_variant_name`/`variant_name`); `parts() -> &'static [Part]`, with
  `Part { name, element }`; `form_control()` (the part attributes are
  validated against); `takes_choices()`; `accepts_optional()`;
  `renders(FieldType, has_choices) -> bool`;
  `default_for(FieldType, optional) -> Self`; and a way to say "not rendered
  yet". The refusal and rule MESSAGES can move too, so the macro stops
  holding them.
- [x] **Unit tests** (Claude; DONE 2026-10-13, 13 in `widgets.rs`, each
  mutation-checked) in `formoxus-spec`: every widget has exactly one
  form-control part; part names are unique within a widget;
  `from_form_name(form_name(w)) == Some(w)` for every widget; `default_for`
  covers every `FieldType`, optional and not; `renders` agrees with
  `default_for` (a default always renders).
- [x] **The macro side** (Claude, delegated by Todd; DONE 2026-10-13: the
  string tables are gone, `WidgetKind::Named { name, widget }` holds the
  resolved `BuiltinWidget`, messages stay in the macro as matches on it;
  pulled in from the runtime box: `field_kind::renders` takes a
  `BuiltinWidget`, `WidgetClass` gone. Goldens unchanged, byte for byte): `widgets!` maps names to `BuiltinWidget`
  instead of strings; `rule()`, `Rule`, `CHOOSERS` and the `refusal` match go,
  replaced by `BuiltinWidget` calls; the emitted widget path names the enum.
  Goldens (Claude): regenerate with `TRYBUILD=overwrite` and READ the diff;
  only message wording should move, if anything.
- [x] **The runtime side** (DONE 2026-10-13; broken down 2026-10-13, line numbers from
  `d3b4ae5`). Already done in the box before: `field_kind::renders` takes a
  `BuiltinWidget` and `WidgetClass` is gone.
  - [x] **R1. `WidgetType` = `Builtin(BuiltinWidget) | Custom { … }`**
    ([types.rs](src/widgets/types.rs)). DONE (Todd), with R-a, R-b, R-c
    as proposed.
    `PartialEq`: `(Builtin(a), Builtin(b)) => a == b`, `Custom` by name,
    `_ => false` (the `discriminant` fallback goes). `Debug`: see R-a. Also
    R-b (`From`), R-c (`Copy`).
  - [x] **R2. Exports.** DONE (Todd, per R-e). `widgets.rs:73` re-exports `WidgetType` but nothing
    re-exports `BuiltinWidget` or `RenderedInputType` except the
    `formoxus::attrs` glob (`lib.rs:49`). Anyone calling
    `with_custom_widget` now has to name `BuiltinWidget`; see R-e.
  - [x] **R3. `ScalarWidget`** (DONE, Claude, per R-d; the not-yet three
    get their own `is_rendered` assert before the `renders` one) ([scalar.rs:37](src/widgets/scalar.rs#L37)).
    Today it matches `(field_type, widget)` pairs and repeats the pair rules
    that `BuiltinWidget::renders` now owns, with a `_ => panic!`. Proposed
    shape (R-d): `Custom` first, unchanged; then for `Builtin(w)`, ONE
    `assert!(w.renders(field_type, choices.is_some()), …)` as the gate, and
    an inner `match w` exhaustive over `BuiltinWidget` deciding only WHICH
    component: `Input(Checkbox)` / `Input(t)` / `Textarea` / `Select` /
    `RadioGroup`, and `SelectMultiple | CheckboxMultiple | File` panicking
    "not supported yet" (still reachable through `with_custom_widget`, which
    bypasses `form!`). The bool-without-choices fallback (`bool_choices`)
    stays where `Select`/`RadioGroup` take their choices.
    `reject_if_not_required` (:118) stays as is: it reads the `required`
    attribute, which `with_owned_attrs` never sets for a checkbox, so it
    cannot be widened to "any widget whose `accepts_optional()` is false".
    The doc comment at :17–20 ("`form!` accepts every widget name against
    every field") is stale since `form!` checks pairs; fix it while here.
  - [x] **R4. `fields.rs`.** (DONE, Todd) `default_widget` (:406) becomes
    `WidgetType::Builtin(BuiltinWidget::default_for(self.field_type(),
    self.optional))`; its comments move to `default_for`, which already
    carries them. `with_owned_attrs` (:169) and `is_unticked_checkbox` (:434)
    match `Builtin(Input(Checkbox))`.
  - [x] **R5. `VariantSet`** (DONE, Claude) ([variant_set.rs:196](src/members/variant_set.rs#L196)):
    `&WidgetType::Builtin(BuiltinWidget::Select)`.
  - [x] **R6. The macro** (DONE, Claude): `widget_type_tokens` becomes
    `WidgetType::Builtin(#builtin_tokens)`, so the two token functions
    collapse to one; its tests that look for `WidgetType :: Input` look
    for `WidgetType :: Builtin` and `BuiltinWidget :: Input`.
  - [x] **R7. Doc links** (DONE, Claude): `widgets.rs:22–23` (`WidgetType::RadioGroup`
    → `BuiltinWidget::RadioGroup`); the `field_kind` comment saying "until
    that match is exhaustive over `BuiltinWidget`, keep the two in step"
    goes once R3 lands.

  Decisions (Todd):
  - **R-a. `Debug` for `Builtin`.** `Custom` prints `custom(Name)`, `form!`'s
    spelling, so panics quote what the author wrote. Proposed: `Builtin(w)`
    prints `w.form_name()` (`select`, `tel`) for the same reason. No test
    asserts the old `Input(Text)` form.
  - **R-b. `impl From<BuiltinWidget> for WidgetType`.** Proposed yes; tests
    and `with_custom_widget` callers then write `BuiltinWidget::Select.into()`.
    Whether `with_custom_widget` should take `impl Into<WidgetType>` is a
    further, optional step.
  - **R-c. `WidgetType: Copy`.** Both variants are now `Copy` inside (a fn
    pointer and a `&'static str`), so it can be; that removes the
    `custom_widget.clone()`s (`fields.rs:425`, `:657`). Proposed yes.
  - **R-d. `ScalarWidget`'s shape**, as in R3: gate on `renders`, then an
    exhaustive match. The two specific panics ("`select` needs choices",
    "`radio_group` needs choices") become the one gate's message, naming the
    widget by `form_name` and the field's type and path. Only reachable
    without `form!`, which reports the specific message at compile time.
  - **R-e. Where `BuiltinWidget` is exported.** Proposed: `formoxus::widgets`
    beside `WidgetType` (with `RenderedInputType`, which it holds), not the
    prelude. Someone writing `form!` never names either; someone building a
    `FormSpec` by hand or writing a custom widget does.
- [x] **Tests that move** (DONE 2026-10-13, Claude; also the five
  `.clone()`s on the now-`Copy` `WidgetType` that clippy flagged; `just ci`,
  `just e2e` (31) and `widget_matrix` green): `fields.rs` lib tests (:1390–1642, 7
  references); suite `specs.rs` (7), `widgets.rs` (7), `author_attrs.rs` (1,
  a `Custom`, likely untouched); `examples/src/bin/widget_matrix.rs`, whose
  hand-kept list of 21 `(name, WidgetType)` pairs (:56–87) becomes
  `BuiltinWidget::ALL` mapped through `form_name`. Then the full `just ci`
  and **`just e2e`**, since R3 changes the dispatch every field goes
  through.

### P3. Configurable default classes

DECIDED (Todd, 2026-10-10):

- **A system is MVP; presets beyond two are not.** MVP ships `Classes::fx()`
  (the default, today's names) and `Classes::bare()` (no classes). Later:
  `pico()`, `bootstrap5()`, `tailwind()`, and a Bulma prototype.
- **Keyed by (widget, part)** for the built-ins, plus fixed slots for what no
  widget renders: `<form>`, its title, form-level errors, the button row and
  buttons, the nested-struct/list/variant fieldsets and their legends, the row
  buttons, the required marker, the error list and items. A slot holds the
  whole string, so today's composed names (`fx-form-field fx-radio-group`)
  are just one slot's value; #10's "not a flat map" worry goes away.
- **Two override levels:** the app (`Formoxus::with_classes`, through
  context) and the form (a `form!` key). Per field is `class`/`class_plus`,
  which replace or append to the configured base of the part they land on.
  Resolved once at the `Form` boundary (the rule in `.claude/memory/config_cascade.md`), then
  provided to the widgets as #10 describes.

Also decided (Todd, 2026-10-10):

- **P3-a. Each slot is a base class plus an optional invalid class.** Every
  framework checked marks the INPUT when invalid (Bootstrap `is-invalid`,
  Bulma `is-danger`, Foundation `is-invalid-input`, daisyUI `input-error`;
  Pico uses `aria-invalid`, already emitted); Foundation also marks the label
  and the message, Bulma the message. `fx()` keeps `fx-invalid` on the
  wrapper only.
- **P3-b. A custom widget's class function receives the resolved `Classes`**,
  so a custom text-like widget can borrow the text input's slot and follow
  the preset.

Broken down 2026-10-13 (Claude), line numbers from `800817f`.

#### What `fx()` has to reproduce

45 literals across 14 files (`grep '"[^"]*fx-' formoxus/src`). **Widget
parts**, keyed by P2's `BuiltinWidget::parts()`:

| widget | part | base | invalid | emitted at |
|---|---|---|---|---|
| text-like `Input` | `div` | `fx-form-field` | `fx-invalid` | `FieldProps::field_class` ([types.rs:113](src/widgets/types.rs#L113)), used at `input.rs:108` |
| | `label` | `fx-field-label` | | `field_label`, `input.rs:191` |
| | `input` | `fx-control fx-input` | | `input.rs:92` |
| `Input(Hidden)` | `input` | (none) | | `input.rs` `hidden` |
| `Input(Checkbox)` | `div` / `input` / `label` | `fx-form-field` / `fx-control fx-checkbox` / `fx-field-label` | `fx-invalid` on `div` | `input.rs:155`, `:136`, `:191` |
| `Textarea` | `div` / `label` / `textarea` | `fx-form-field` / `fx-field-label` / `fx-control fx-textarea` | `fx-invalid` on `div` | `textarea.rs:27`, `:53` |
| `Select` | `div` / `label` / `select` | `fx-form-field` / `fx-field-label` / `fx-control fx-select` | `fx-invalid` on `div` | `select.rs:37`, `:64` |
| `RadioGroup` | `fieldset` | `fx-form-field fx-radio-group` | `fx-invalid` | `radio_group.rs:48` (`field_class_plus`) |
| | `legend` | `fx-legend fx-field-label` | | `radio_group.rs:107` |
| | `div` (per choice) | `fx-choice` | | `radio_group.rs:70` |
| | `input` / `label` | `fx-control fx-radio` / `fx-choice-label` | | `radio_group.rs:72`, `:89` |

An enum field's `VariantSelect` renders `Select`'s three parts with
`Select`'s classes (`variant_select.rs:34`, and its wrapper `div` through
`field_class`).

**Fixed slots**, which no widget renders:

| slot | base | emitted at |
|---|---|---|
| form | `fx-form` | `form.rs:341` |
| form title | `fx-form-title` | `state.rs:276` |
| form errors list / item | `fx-form-errors` / `fx-form-error` | `state.rs:295`, `:298` |
| button-problems list / item | `fx-form-errors fx-problems` / `fx-form-error fx-problem` (Todd, 2026-10-14; was `fx-form-error`) | `form.rs:650`, `:652` |
| button row | `fx-buttons` | `form.rs:413` |
| a button, per `ButtonType` | `fx-button fx-button-{submit,reset,cancel,destructive,action}` | `buttons.rs:110–114` |
| add-row / remove-row button | `fx-button fx-button-add-row` / `…-remove-row` | `row_buttons.rs:23`, `:49` |
| nested struct fieldset / legend | `fx-group fx-field-set` / `fx-legend fx-field-label` | `field_set.rs:38`, `:40` |
| list fieldset / legend / row | `fx-group fx-list-set` / `fx-legend fx-field-label` / `fx-form-row` | `list_set.rs:111`, `:113`, `:122` |
| enum fieldset / legend | `fx-group fx-variant-set` / `fx-legend fx-field-label` | `variant_set.rs:202`, `:205` |
| required marker | `fx-required` | SIX copies: `input.rs:195`, `textarea.rs:57`, `select.rs:68`, `variant_select.rs:64`, `radio_group.rs:115`, `variant_set.rs:209` |
| field error list / item | `fx-field-errors` / `fx-field-error` | `errors.rs:41`, `:44` |

#### Decisions first (Todd)

- **P3-c. How slots are keyed.** DECIDED (Todd, 2026-10-14) as proposed;
  naming a part a widget does not have PANICS (it is probably a typo). one map,
  `SlotKey = Part(BuiltinWidget, &'static str) | Fixed(FixedKey)`, where
  `FixedKey` is a `#[non_exhaustive]` enum of the rows above. `bare()` is
  then the EMPTY map (a missing key renders no `class` at all), `fx()` fills
  it with loops over `BuiltinWidget::ALL` and `parts()`, and an override is
  one insert. A builder that sets one part on many widgets at once (every
  text-like `input`, say) keeps presets short. Rejected alternative: a struct
  with a named field per role (`wrapper`, `label`, `control`, …), which
  cannot give `color` its own class (Bootstrap's `form-control-color`) and
  needs a new field per new widget.
  **REVISED 2026-10-14 (Todd):** `SlotKey` lives in `formoxus-spec`, public
  there, so `form!`'s `classes_plus:` and `Classes` key slots one way. It is
  kept out of `formoxus`'s surface not by being private but by
  `formoxus::attrs` becoming an explicit re-export list that omits it (it was
  a glob of `formoxus_spec::*`). Its variant fields cannot be private, so
  `SlotKey::Part(w, "lable")` is writable; every checked route
  (`SlotKey::part`, `with_part`, `ClassEdit::part`, `form!`) rejects one.
- **P3-d. The slot type.** DECIDED as proposed: `Slot { base: Cow<'static, str>,
  invalid: Option<Cow<'static, str>> }`: presets are `&'static str`, while
  an app may `format!` its own.
- **P3-e. How the resolved `Classes` reaches the widgets.** DECIDED: (2). Issue #10
  proposed context: `Form` resolves and provides it. **Catch: `Form` is not
  a component.** Its render methods run inside the consumer's component, so
  a bare `provide_context` there would land in the CONSUMER's scope, shared
  by everything that component renders, and two forms in one component
  would overwrite each other's classes. Two ways that work:
  (1) context, provided by a small private component `Form` wraps its
  output in; (2) threaded: `Rc<Classes>` on `RenderCtx` (which already
  carries `label_case`, the precedent) and on to the widgets as a prop.
  #10's objection to threading was cloning ~15 strings per field; an `Rc`
  answers that. Proposed: (2). It needs no hidden scope, and it hands a
  custom widget the resolved `Classes` through `WidgetProps`, which is
  exactly P3-b.
- **P3-f. The form level LAYERS on the app's classes.** REVISED (Todd,
  2026-10-14), reversing the first answer, which replaced the app's
  `Classes` whole: that was verbose (`Classes::fx().with_part(…)` in every
  form), and it hard-coded the app's base into each form, so changing the
  base meant finding every form. Now, mirroring the field-level
  `class:`/`class_plus:` pair:
  - **`classes_plus: { … }`**, the common case: slot edits applied ON TOP of
    whatever the form would otherwise get (the app's `Classes`, through
    `Formoxus`). A form never restates the base.
  - **`classes: EXPR`** still replaces the whole thing, for "this one form
    is `Classes::bare()`".
  - **Both may be given** (unlike the field pair): `classes:` first, then
    `classes_plus:` on top of it ("start bare, then add these").
  - **The block's grammar, not a Rust expression** (decided: the grammar
    alone; a `with_parts`-style filter is not expressible in it, and that is
    accepted for now):
    ```rust
    classes_plus: {
        select.select: [form_select],                                // replace the slot
        text.input: { base: [form_control], invalid: [is_invalid] }, // replace both
        textarea.textarea: + [mine],                                 // APPEND to the base
        text.input: + { base: [x], invalid: [y] },                   // append to both
        button_row: [actions],                                       // a FixedKey, snake_case
        button.submit: [btn, btn_primary],                           // FixedKey::Button(Submit)
    }
    ```
    **Class values use the field-level `class:` list syntax** (Todd,
    2026-10-14: one way to write classes in `form!`): `[a, "b"]`, a bare name
    with `_` → `-`, a quoted one verbatim, the same targeted errors. Shared,
    not copied: the bracketed `CssClassName` parse at `field.rs:468–475`
    becomes a `pub(crate)` `class_list(input)` both call.
    A plain entry REPLACES the slot (what a slot is); `+ "…"` appends to the
    slot's base, keeping its invalid class (decided). A widget part is
    `form_name.part`; a fixed key is its variant in snake_case, and
    `button.<type>` names a button by its `form!` type keyword. Every name is
    checked at COMPILE time with a caret on it, which is why `FixedKey` and
    `ButtonType` move to `formoxus-spec` (P3-4b): the macro can then see
    them, as it sees `BuiltinWidget`.
- **P3-g. `class:` and the invalid class.** DECIDED as proposed. An author's `class:` replaces
  the control part's BASE. Proposed: the part's INVALID class is still added
  when the field has errors, because it marks state, not style (Bootstrap's
  `is-invalid` is what draws the red border). `fx()` puts no invalid class on
  a control, so this changes nothing today.
- **P3-h. `VariantSelect`** (DECIDED as proposed) reuses `Select`'s `label` and `select` slots
  (proposed), rather than getting fixed slots of its own.

#### Checklist

- [x] **P3-1. Prep (Claude, DONE 2026-10-14):** `widgets/label.rs`'s
  `field_label` and `required_marker` are now the only copies; the
  snapshot is unchanged. Was: the required marker is
  written out six times, and the label beside it four (`input.rs`'s
  `field_label`, `textarea.rs`, `select.rs`, `variant_select.rs`). Folding
  them into the one `field_label` first means each slot is looked up in one
  place. Behaviour-neutral; the suite's exact-markup tests pin it.
- [x] **P3-2. A snapshot (Claude, DONE 2026-10-14):** NOT the `/t/` forms
  and the gallery after all (the examples crate is a binary, and it cannot
  render error states without a browser), but `tests/suite/snapshot.rs`:
  one model touching every slot above, rendered clean, with field errors,
  and with form errors plus the button-problems list (a misdeclared button
  fn renders it). Goldens in `tests/snapshots/`, one tag per line,
  regenerated with `SNAPSHOT=overwrite`; the form id normalized to `FORMID`.
  Mutation-checked. Was: the SSR
  HTML of every `/t/` test form and the gallery, clean and with errors,
  written to a golden file. The suite's exact-markup tests cover a lot but
  not every element; this makes "byte-identical" checkable rather than
  hoped for.
- [x] **P3-3. `Classes`, `Slot`, `FixedKey`** (DONE 2026-10-14) in `formoxus/src/classes.rs`
  (Todd; the file exists, empty). Fleshed out 2026-10-14 (Claude); a
  proposal throughout, so change what reads wrong.
  - [x] **P3-3a. `Slot`** (DONE 2026-10-14, Todd; docs and 6 tests, Claude), per P3-d:
    ```rust
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Slot {
        pub base: Cow<'static, str>,
        pub invalid: Option<Cow<'static, str>>,
    }
    impl Slot {
        pub const fn new(base: &'static str) -> Self;   // Cow::Borrowed is const
        pub fn with_invalid(self, invalid: impl Into<Cow<'static, str>>) -> Self;
        /// The `class` value for this state: `base`, then `invalid` when
        /// `invalid` is true, space-joined; `None` when that is empty, so a
        /// slot of `""` renders no `class` attribute rather than `class=""`.
        pub fn value(&self, invalid: bool) -> Option<String>;
    }
    ```
    `pub` fields, like `Formoxus`'s, since a `Slot` is plain data with
    nothing to protect; `new` plus `with_invalid` is for presets.
  - [x] **P3-3b. `FixedKey`** (DONE 2026-10-14, Claude; `Hash` on `ButtonType`, Todd): `#[derive(Clone, Copy, Debug, PartialEq, Eq,
    Hash)] #[non_exhaustive] pub enum`, one variant per fixed-slot row:
    `Form`, `FormTitle`, `FormErrors`, `FormError`, `ButtonProblems`,
    `ButtonProblem`, `ButtonRow`, `Button(ButtonType)`, `AddRowButton`,
    `RemoveRowButton`, `FieldSet`, `FieldSetLegend`, `ListSet`,
    `ListSetLegend`, `ListRow`, `VariantSet`, `VariantSetLegend`,
    `RequiredMarker`, `FieldErrors`, `FieldError`. Two notes:
    `ButtonProblem` is its own variant although `fx()` gives it the same
    `fx-form-error` as `FormError` (a preset may want them apart); and
    `Button(ButtonType)` needs `Hash` on `ButtonType`
    ([buttons.rs:74](src/buttons.rs#L74) derives `Eq` but not `Hash`).
  - [x] **P3-3c. `Classes`** (DONE 2026-10-14, Todd), per P3-c:
    ```rust
    #[derive(Clone, Debug, PartialEq)]
    #[non_exhaustive]
    pub struct Classes { slots: IndexMap<SlotKey, Slot> }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum SlotKey { Part(BuiltinWidget, &'static str), Fixed(FixedKey) }   // private
    ```
    The map is private; `SlotKey` is `formoxus-spec`'s but not re-exported
    (P3-c, revised), so the public surface is still the methods below.
    `IndexMap` (already a dependency) rather than `HashMap` so `Debug`
    prints in a stable order; its `PartialEq` ignores order either way.
  - [x] **P3-3d. The presets** (DONE 2026-10-14, Claude: `fx()` walks every
    RENDERED widget's `parts()` through a private `fx_part(widget, part)`
    match that panics on a pair it does not know; the not-yet widgets get
    no classes until they ship; 4 preset tests, mutation-checked; the
    text-like row is a match arm, so no `is_text_input` was needed yet). `pub fn fx() -> Self`, the two tables
    above, written as loops over `BuiltinWidget::ALL` and `parts()` so a new
    widget cannot be forgotten silently; `pub fn bare() -> Self`, the empty
    map; `impl Default for Classes` is `fx()`, so `Formoxus::default()`
    keeps rendering today's names. The text-like `input` row wants a way to
    say "every `Input` but `Checkbox` and `Hidden`": either a match in
    `fx()`, or a `const fn is_text_input()` on `BuiltinWidget` in
    `formoxus-spec`, where the same set already appears in `renders()`.
  - [x] **P3-3e. Builders** (DONE 2026-10-14: `with_part` and `with_fixed`,
    Todd; `with_parts`, docs and 7 tests, Claude), each consuming and returning `Self`:
    - `with_part(widget, part: &str, slot)`: one widget's part. PANICS when
      `part` is not one of `widget.parts()` (P3-c: probably a typo), naming
      the widget and the parts it has. Looks the name up in `parts()` to
      get the `&'static str` the key stores.
    - `with_parts(filter: impl Fn(BuiltinWidget) -> bool, part, slot)`: that
      part on every widget the filter keeps that HAS the part; panics if
      none does. Presets need it, e.g. Bootstrap's
      `with_parts(|w| w.is_text_input(), "input", Slot::new("form-control"))`.
    - `with_fixed(slot: FixedKey, value: Slot)`.
    - Removing a slot (`without_part`/`without_fixed`) is left out until
      something needs it; inserting `Slot::new("")` already renders nothing.
  - [x] **P3-3f. Lookups** (DONE 2026-10-14, Todd; a shared private
    `part_name` gives lookup and `with_part` the same panic), what P3-7's call sites use:
    `part(&self, widget, part: &str) -> Option<&Slot>` (same panic as
    `with_part` on a name the widget lacks, which at a lookup is a formoxus
    bug) and `fixed(&self, slot) -> Option<&Slot>`. Plus the one-step form
    the call sites actually want: `part_class(widget, part, invalid) ->
    Option<String>` and `fixed_class(slot, invalid) -> Option<String>`,
    each `Slot::value` on the lookup. An `Option` so `rsx!` can write
    `class: classes.part_class(…)`, which omits the attribute on `None`.
  - [x] **P3-3g. Exports** (DONE 2026-10-14, Claude; a suite test,
    `tests/suite/classes.rs`, names each from outside the crate), per the prelude rule: `Classes` in the prelude
    (a `form!` author writes `classes: Classes::bare()`); `Slot` and
    `FixedKey` from `formoxus::classes`, with `pub mod classes` in
    `lib.rs`. Inside the crate, full paths (`crate::classes::Classes`).
  - [ ] **P3-3h. Not in this step:** wiring `Classes` into `Formoxus`,
    `FormSpec` and `form!` (P3-5), delivery (P3-6), and any call site
    (P3-7). After P3-3 nothing renders differently, and the snapshot proves
    it.
- [x] **P3-4. Unit tests (Claude; DONE 2026-10-14, 23 in `classes.rs`, each
  group mutation-checked, plus 2 suite tests of the export paths), inline in `classes.rs`:** every
  `(widget, part)` in `BuiltinWidget::parts()` has an `fx()` slot except
  `hidden`'s `input`; `fx()`'s values are the tables above; `bare()` is empty;
  a builder override replaces exactly one slot.
- [x] **P3-4b. `ButtonType`, `Invocation` and `FixedKey` move to
  `formoxus-spec`** (decided by Todd 2026-10-14; DONE the same day, Claude:
  `formoxus-spec/src/{buttons,fixed_keys}.rs`, 8 spec tests; the macro's
  copies of both enums are gone, parsed by free fns since the orphan rule
  forbids `impl Parse` on a spec type; `button_classes(ty)` is the interim
  fn; goldens unchanged), so
  `form!` can check `classes_plus:` names at compile time. Like
  `BuiltinWidget`: dependency-free, `const fn`s, an `ALL`.
  - `ButtonType` moves with `html_type()` and `default_invocation()`, so
    `Invocation` moves too (a plain enum; an inherent method cannot stay
    behind in `formoxus`). It gains `form_name()`/`from_form_name()`
    (`submit` … `action`). `classes()` holds `fx-` strings and cannot move;
    it becomes a private fn in `buttons.rs` until P3-7 deletes it.
  - The macro's own copy goes: `formoxus-macros/src/form/button.rs:146`'s
    `ButtonType` and its `button_types!` table, replaced by the spec's, as
    `widgets!` was by `BuiltinWidget`.
  - `FixedKey` moves with a `form_name()`/`from_form_name()` pair over the
    snake_case names, `Button(t)` spelled `button.<t>`.
  - The paths stay: `formoxus::buttons::{ButtonType, Invocation}` and
    `formoxus::classes::FixedKey` re-export them. Internal code uses the
    full module path, per the prelude rule.
- [x] **P3-5. The cascade** (DONE 2026-10-14: `Formoxus`/`FormSpec`/wiring,
  Todd; `ClassEdit` reshaped per piece, `with_edit`, the fold, the
  `classes_plus:` parser and expansion in `formoxus-macros/src/form/classes.rs`,
  8 goldens and 6 cascade suite tests, Claude), per P3-f as revised:
  - `Formoxus::classes: Classes` (default `fx()`) + `with_classes`.
  - `FormSpec` gets `classes: Option<Classes>` (the replace) AND an ordered
    list of slot edits (the layer), with `with_classes` and one builder per
    edit. BUILT (Todd; made opaque by Claude 2026-10-14): `ClassEdit` in
    `formoxus::classes`, a struct with PRIVATE `key: SlotKey`, `slot: Slot`,
    `append: bool`, built only by `ClassEdit::part(widget, part, slot)`,
    `::part_plus(…)`, `::fixed(key, slot)`, `::fixed_plus(…)` (the part ones
    check the name, panicking as `with_part` does), and
    `FormSpec::with_class_edit(edit)`; `form!` emits one call per entry.
    An append joins the edit's base onto the slot's base and its invalid
    class onto the slot's invalid class; onto an absent slot it gives just
    the edit's. `Classes::with_edit` applies one; `Form::classes()` folds
    `FormState::class_edits()` over the base. REVISED the same day: `+` is
    PER PIECE, so an edit is `{ key, base: Option<Change>, invalid:
    Option<Change> }`, built `ClassEdit::part(w, p).replace_base(…)
    .append_invalid(…)`; `part_plus`/`fixed_plus` are gone.
  - `Form::classes()` resolves: `spec.classes`, else the app's
    (`defaults().classes`), then every edit in order. Like
    `Form::label_case()` (`form.rs:284`), resolved once at the `Form`
    boundary.
  - `form!` parses `classes:` and `classes_plus:`, checking each name
    against the spec's `BuiltinWidget::parts()`, `FixedKey` and
    `ButtonType`. Goldens (Claude): a misspelt widget, part, fixed key and
    button type; `+` on a malformed value; a duplicate entry.
- [x] **P3-6. Delivery** (DONE 2026-10-14, Todd and Claude), per P3-e: every element that P3-7 will
  class can REACH the form's resolved `Classes`. Nothing renders differently
  after this step: the literals stay until P3-7, so the snapshot must not
  move. Broken down 2026-10-14 (Claude), line numbers from `f36669f`.
  **The rule (Todd, 2026-10-14):** a render FUNCTION takes `&RenderCtx`; a
  COMPONENT takes props. A widget is a component and never sees a
  `RenderCtx` (it carries `prefix`, `required`, `values`, `on_edit`, which a
  leaf must not depend on, and a component's props need `PartialEq`, which
  `RenderCtx` lacks), so a widget's classes reach it inside `FieldProps`, and
  the helpers inside a widget take the `&Classes` from there.
  - [x] **P3-6a. `RenderCtx` carries it.** DONE in part (Todd):
    `classes: Rc<Classes>` ([members.rs:259](src/members.rs#L259)), and
    `root` takes a `Classes` and wraps it itself, so callers never see the
    `Rc` and cannot pass one shared with anything else; `nested` and
    `optional` clone the `Rc` (a pointer copy). Those two list every field
    by hand, so a new field means editing both; `Self { prefix: …,
    ..self.clone() }` would not. Still to do: a doc comment on the field.
    **`Rc`, not `Arc`, and only on the render side** (Todd, 2026-10-14).
    Dioxus renders a `VirtualDom` on one thread (it is not `Send`, and
    neither props nor `provide_context` ask for `Send`), so `Arc` would only
    add an atomic count. The rule that matters: **never an `Rc` inside
    `FormSpec` or `FormState`.** Both reach the server, where `Submission`
    rebuilds a `FormState` with no renderer and a handler's future must be
    `Send`; one `Rc` there would break that path. They hold `Classes` and
    `ClassEdit`s by value (plain data, so `Send`); the `Rc` lives only in
    `RenderCtx` and `FieldProps`, which exist only while rendering. Worth
    saying in the doc comment on `RenderCtx::classes` too.
  - [x] **P3-6b. One context, built at the top of `render`.** DONE in part
    (Todd): a private `Form::root_ctx()` (id, store, callback,
    `label_case()`, `classes()`). **`render` is now the ONLY public way to
    draw a form** (Todd, 2026-10-14): `Form::render_fragment`,
    `render_title`, `render_fields` and `render_errors` are DELETED, and
    `FormState`'s pieces are `pub(crate)`. Unlike Django, whose template
    owns the `<form>`, formoxus owns the `<form>`, its `onsubmit` and the
    buttons, so a piece rendered alone could not be submitted; and adding a
    public method later is not breaking, while removing one after publishing
    is. The suite's ~99 `render_fragment()` calls became
    `render(using_fns! {})`; one test of `render_fragment` itself and three
    of the pieces' slicing were deleted, one rewritten through `render`.
    So the context is built once, in `render`, which passes it on to
    `render_buttons` and the free `render_problems` as `&RenderCtx`, for
    the form wrapper, the button row, the buttons and the problems list.
  - [x] **P3-6c. `FormState`'s pieces all take `&RenderCtx`.**
    `render_title` and `render_errors` take none today; they join
    `render_fields`, which does. All three are `pub(crate)`, called only
    from `render`.
  - [x] **P3-6d. The members** (`FieldSet`, `ListSet`, `VariantSet`)
    already render from `&RenderCtx`, so their own fieldsets and legends
    need nothing new. `AddRowButton`/`RemoveRowButton` (`list_set.rs:124`,
    `:127`) are components, so `ListSet` passes them `ctx.classes.clone()`
    as a new `classes: Rc<Classes>` prop (P3-6-C).
  - [x] **P3-6e. The widgets, through `FieldProps`.** (Todd also dropped
    every `let FieldProps { … }` for `props.x`; Claude added the doc
    comments on both `classes` fields.) `FieldProps` gains
    `classes: Rc<Classes>` ([types.rs](src/widgets/types.rs)). It is
    already what every built-in widget receives, and what a custom widget
    receives inside `WidgetProps`, so that one field is P3-b's "a custom
    widget's class function receives the resolved `Classes`". Built in two
    places (`fields.rs:529`, `variant_set.rs:220`), both from
    `ctx.classes.clone()`; destructured by six `let FieldProps { … }`, each
    gaining `classes`. A widget hands it on to `FieldErrors` (a component:
    a `classes: Rc<Classes>` prop) and to `field_label`/`required_marker`
    (functions inside a widget, with no context: a `&Classes` parameter).
  - [x] **P3-6f. Tests (Claude; DONE 2026-10-14: the literals by Todd;
    `a_custom_widget_receives_the_forms_resolved_classes` in
    `tests/suite/classes.rs`, mutation-checked):** the three suite `FieldProps { … }`
    literals (`tests/suite/widgets.rs:185`, `:210`, `:586`) gain
    `classes: Rc::new(Classes::fx())`; the snapshot unchanged; a suite test
    that a custom widget's `WidgetProps.props.classes` is the FORM's
    resolved `Classes` (`classes_plus:` included), which is P3-b and the
    only part of P3-6 observable before P3-7.

  Decisions (Todd):
  - **P3-6-A. `FieldProps` as the route to the widgets** (P3-6e), rather
    than a separate `classes` prop on `ScalarWidget` and each widget.
    DECIDED (Todd, 2026-10-14) as part of the rule above.
  - **P3-6-B. `Rc<Classes>`'s `PartialEq` compares contents.** Dioxus
    compares a component's props to decide whether to re-render, so every
    widget compares ~80 slots each time. Proposed: accept it for now (a map
    compare is cheap next to a re-render). The alternative is a newtype
    whose `PartialEq` is `Rc::ptr_eq`, which is right only if a form never
    rebuilds an equal `Classes` in a new `Rc`, and P3-6b does exactly that
    on every render, so pointer equality would re-render every widget every
    time.
  - **P3-6-C. One rule for components: take `classes: Rc<Classes>` and look
    up their own slots**, even where only one is needed (the row buttons).
    Proposed, for uniformity; the alternative is passing a resolved
    `class: Option<String>` to the one-element components.
- [ ] **P3-7. Replace the literals** (Todd), file by file, running
  `cargo test -p formoxus --test suite snapshot` after each: it must stay
  green throughout, since `fx()` holds exactly today's literals. Broken down
  2026-10-14 (Claude); 37 literals in 14 files, line numbers from the
  working tree after P3-6.

  **Decisions first (Todd), each with a proposal:**
  - **P3-7-A. How a widget part asks for its class:**
    `props.classes.part_class(WIDGET, "part", invalid)`, where `invalid` is
    `!props.errors.is_empty()` (the field's own errors, what `fx-invalid`
    and `aria-invalid` already follow). Each widget names its own
    `BuiltinWidget`: `Input(input_type)` (so a checkbox is
    `Input(Checkbox)` and a hidden input `Input(Hidden)`, whose `input` has
    no slot and so no `class`), `Textarea`, `Select`, `RadioGroup`, and
    `VariantSelect` borrows `Select`'s (P3-h). The radio group's per-choice
    `div`, `input` and `label` pass the same `invalid`.
  - **P3-7-B. `field_label` needs the widget.** It is shared, but the
    `label` slot is per widget, and Foundation-style presets give the label
    an invalid class. Proposed: `field_label(widget, id, text, marked,
    invalid, classes)`, so it looks the slot up itself; the alternative is
    each caller passing a resolved `class: Option<String>`.
  - **P3-7-C. `FieldAttrs::class(base)` takes the slot.** Today
    `class(&self, base: &str) -> String` ([fields.rs:136](src/fields.rs#L136)).
    It becomes `class(&self, slot: Option<&Slot>, invalid: bool) ->
    Option<String>`: an author's `class:` replaces the slot's BASE,
    `class_plus:` appends to it, and either way the slot's INVALID class is
    still added when `invalid` (P3-g); `None` when the result is empty, so a
    `bare()` control renders no `class`. (`Slot::value` already does the
    no-author-classes case.)
  - **P3-7-D. `invalid` for slots that only exist when there ARE errors**
    (`FixedKey::FieldErrors`/`FieldError`, `FormErrors`/`FormError`,
    `ButtonProblems`/`ButtonProblem`): proposed `false`, so a preset styles
    them with their base, and the invalid class stays the mark of an errored
    FIELD. The groups (`FieldSet`, `ListSet`, `VariantSet` and legends) pass
    `false` too: `fx()` gives them no invalid class, and only `VariantSet`
    tracks errors of its own.

  **One `rsx!` fact the whole step relies on:** `class: x` where `x:
  Option<String>` omits the attribute on `None`. CONFIRMED 2026-10-14 by a
  throwaway SSR probe: `div { class: None::<String> }` renders `<div>`, and
  `Some("a b")` renders `class="a b"`. P3-8's `bare()` test pins it.

  **The files, in an order where each builds on the last:**
  - [ ] **P3-7a. `FieldAttrs::class`** to the P3-7-C signature; its five
    callers (`input.rs:93`, `:130`, `textarea.rs:28`, `select.rs:38`,
    `variant_select.rs:35`) pass `props.classes.part(WIDGET, "input" /
    "textarea" / "select")` and `invalid`.
  - [ ] **P3-7b. The wrapper `div`.** `FieldProps::field_class()` and
    `field_class_plus()` ([types.rs:123](src/widgets/types.rs#L123), 7
    callers) are deleted; each widget uses `part_class(WIDGET, "div",
    invalid)`, and the radio group `part_class(RadioGroup, "fieldset",
    invalid)` (today's `field_class_plus("fx-radio-group")`). The doc
    comment on `field_class` explaining why it picks whole strings (rsx!
    joins two `class:`es with a stray space) is worth keeping on
    `Slot::value`, which now does that job.
  - [ ] **P3-7c. `label.rs`:** `field_label` per P3-7-B (`label.rs:24`);
    `required_marker` uses `fixed_class(RequiredMarker, false)`
    (`label.rs:42`), its `_classes` parameter losing the underscore.
  - [ ] **P3-7d. The radio group** (`radio_group.rs:64` `div`, `:66`
    `input`, `:83` `label`, `:101` `legend`).
  - [ ] **P3-7e. `FieldErrors`** (`errors.rs:47` list, `:50` item) from its
    `classes` prop.
  - [ ] **P3-7f. The members:** `field_set.rs:38`/`:40`,
    `list_set.rs:111`/`:113`/`:122`, `variant_set.rs:203`/`:206`, each a
    `ctx.classes.fixed_class(…, false)`; the row buttons
    (`row_buttons.rs:25`, `:56`) from their `classes` prop.
  - [ ] **P3-7g. The form:** `state.rs:277` (title), `:296`/`:299` (form
    errors); `form.rs:351` (`fx-form`), `:423` (button row), the buttons
    (`form.rs`'s `button_classes(b.ty)` call →
    `fixed_class(Button(b.ty), false)`), `:644`/`:646` (problems). Then
    `buttons::button_classes` (`buttons.rs:90–94`) is deleted.
  - [ ] **P3-7h. Sweep:** `grep '"[^"]*fx-' formoxus/src` finds nothing
    outside `classes.rs` (`fx()` itself) and comments; the snapshot is
    unchanged; `just ci` and `just e2e` (the e2e selectors use these
    classes) green.

- [ ] **P3-8. Tests (Claude):** the snapshot is unchanged; a whole form under
  `bare()` contains no `class=` at all; a form-level `classes:` replaces the
  app's; `classes_plus:` changes only the slots it names, on top of an app's
  CUSTOM `Classes` (not just `fx()`), and `+` appends; both keys together
  apply in order; `class`/`class_plus` replace and append to a CONFIGURED base, not
  just `fx()`'s; P3-g's invalid class survives an author `class:`; a custom
  widget receives the resolved `Classes` (P3-b).
- [ ] **P3-9. Wrap-up (Claude):** a `/t/` test form under `bare()` with its
  e2e test (they come in pairs); `examples/assets/main.css`'s vocabulary
  comment checked against `fx()`; `css_class_vocabulary.md` updated; close
  #10.

## Step 4: emit only valid attributes, and the custom-widget trait

Closes issue #4. Decisions 6, 10, and the custom-widget section of the note.
Broken down 2026-10-09; line numbers are from `12cb3c3`.

**The idea in one line:** every field already knows its `FieldType`; Step 4
gives it a `FieldControl` too (the element its attributes land on), at compile
time, and then asks the full rule `Attr::is_allowed(field_type, control)`
([attrs.rs:215](../formoxus-spec/src/attrs.rs#L215)), which is built and
unit-tested but has no caller yet. Once nothing invalid can get PAST `form!`,
the runtime can safely drop the one kind that is accepted but not emitted
(`true_and_on`: `min`/`max` on a number rendered as text, `required_true` on a
`<select>` or radios).

**Order matters: 4b before 4c.** If the runtime filtered first, an attribute
`form!` still accepts would vanish from the DOM silently, which is exactly the
failure the compile-time checks exist to prevent.

### Decisions to make first (Todd)

- **D0. Widget PARTS: ADOPTED 2026-10-10.** A widget declares parts named by
  their HTML elements (`label`, `input`, `fieldset`, `legend`, …), every
  widget has a `label` part, parts are addressed inside the widget's braces
  (`radio_group { choices, input: {...} }`), validated keys cannot be
  addressed, and unaddressed keys go where a convention says. Recorded in
  `.claude/memory/attribute_rules_design.md`, "DECIDED 2026-10-10: widgets have
  PARTS". **Open there, A–D:** the convention for unaddressed keys; `label` vs
  `legend` in a radio group; `label` one brace level apart; and a table word
  for `<label>`/`<legend>` (rename `FieldControl`?). D1 dissolves; D4 becomes
  per part. The checklist below is still written for one control per field
  and gets rewritten once A–D are settled.
- **D1. `RadioGroup`'s control.** Its spread lands on the `<fieldset>`, so
  `Fieldset`, as `FieldControl`'s doc says. But `required_true` is
  `true_and_on(Select | Input(Radio))`, so with `Fieldset` a required-true
  radio group over bools would be refused. Either the row adds `Fieldset`, or
  `RadioGroup` maps to `Input(Radio)` for VALIDATED attributes and `Fieldset`
  for the rest. The first is simpler.
- **D2. The message when the control comes from the DEFAULT widget.** A
  const panic's message is fixed when the macro expands, and the macro does not
  know the field's type, so for a field with no `widget:` it cannot say "on a
  checkbox". Proposed: "`rows` is not valid on the element this field renders
  by default; name a widget that takes it, like `widget: textarea`". For a
  named widget the message can name it: "`rows` is not valid on `widget: text`".
- **D3. `max_length` on a chooser.** `a_chooser_carries_them_onto_a_select_or_fieldset`
  ([constraint_attrs.rs:164](tests/suite/constraint_attrs.rs#L164)) puts
  `max_length: 10` on a `select`/`radio_group` over `String`. `MaxLength` is
  `validated: true` and not valid on `Select`/`Fieldset`, with no
  `true_and_on`, so 4b refuses it. Probably right (the choices are fixed, so
  the author controls their length), but it is a behaviour change. The
  alternative is `true_and_on(Select | Fieldset)` on the length and `pattern`
  rows, keeping them server-checked.
- **D4. The custom-widget trait's open questions** (design note): default
  classes as a required `fn` or a `const`; what `AttrSet` is (a `&'static [Attr]`
  plus an `ANY`, or a generated bitset); and whether built-ins implement the
  trait too (note's (b)) now, or later.

### 4a. Every field gets a `FieldControl` at compile time

> **2026-10-10:** the shared widget table and the crate rename moved to
> P2 above. These boxes get rewritten for parts once P1–P2 land and A–E are
> settled (examples: `WIDGET_PARTS_EXAMPLES.md`).

- [ ] **Named widgets, in the macro.** `widgets!`
  ([widget.rs:63](../formoxus-macros/src/form/widget.rs#L63)) gains the
  control per name: `text => Input(Text)` gives `Input(InputType::Text)`,
  `textarea` gives `Textarea`, `select` gives `Select`, `checkbox` gives
  `Input(Checkbox)`, `radio_group` gives D1's answer. `select_multiple`,
  `checkbox_multiple` and `file` are refused at parse already, and
  `custom(…)` has none (4d).
- [ ] **No `widget:`, in const code.** `field_kind::default_control(shape) ->
  Option<FieldControl>`: `Text`/`Int`/`Float` give `Input(Text)`, `bool`
  gives `Input(Checkbox)`, `Option<bool>` gives `Select`, an enum gives
  `Select`; `Unknown` and `Other` give `None` (nothing to check). It MIRRORS
  `FormField::default_widget` ([fields.rs:390](src/fields.rs#L390)), so it
  gets a "keep in step" note on both, like `renders` and `ScalarWidget`.
- [ ] Unit tests (Claude) for `default_control`, one per row above, beside the
  `field_kind` tests.

### 4b. `form!` refuses an attribute the control cannot take

- [ ] **`field_kind::allowed(attr, shape, control) -> bool`**: `applies(attr,
  shape)` (the `for:` half, already emitted) AND `is_valid_on(control) ||
  is_also_validated_on(control)`. `true` when `applies` is false, so a wrong
  field type reports once, from there, as `required_is_not_optional` does.
- [ ] **The macro emits one more assert per table key**, in
  `FieldBody::type_checks` ([field.rs:226](../formoxus-macros/src/form/field.rs#L226)):
  the control as tokens for a named widget, `default_control(#shape)` for
  none (`None` passes), nothing for `custom(…)` until 4d. Messages per D2.
- [ ] **The enum arm of `applies` simplifies.** An enum's control is `Select`
  (from `default_control`), so its `is_valid_on(FieldControl::Select)` half
  moves into `allowed` and the arm becomes `!attr.validated()`. `enum_refusal`'s
  "does not take it" message moves with it.
- [ ] **Goldens** (Claude): `rows` on a text field (default control);
  `rows` on `widget: email` (named); `pattern` on a textarea (replaces the
  suite test `a_textarea_gets_pattern_although_html_has_no_such_attribute`,
  [constraint_attrs.rs:143](tests/suite/constraint_attrs.rs#L143), as the
  build order says); `placeholder` on a bool (checkbox); `min` on a textarea
  (validated, not on it, no `true_and_on`); plus D3's case, whichever way it
  goes. And one that must still COMPILE: `min` on an `i32` with no widget
  (`true_and_on(Input(Text))`).

### 4c. The runtime emits only what is valid on the element

- [ ] **`FieldAttrs::to_attributes` and `merge_with_attrs`** ([fields.rs:75](src/fields.rs#L75),
  [:107](src/fields.rs#L107)) take an `Option<FieldControl>` and skip an
  `AttrKey::Std` whose `is_valid_on(control)` is false. `NonStd` (quoted)
  keys are never filtered; `None` (custom widgets) filters nothing. After 4b
  the only things skipped are `true_and_on` entries and formoxus's own.
- [ ] **The seven call sites pass their control:** `input.rs:23` (its
  `Input(type)`), `textarea.rs:28`, `select.rs:38`, `input.rs`'s `checkbox`
  body (was `checkbox.rs:21`), `radio_group.rs:46` (`Fieldset`), `variant_select.rs:71` (`Select`),
  `scalar.rs:102` (custom: `None`).
- [ ] **`RadioGroup`'s `FieldAttrs::without(Attr::Required)` may go.**
  `Required` is not valid on `Fieldset`, so the filter drops it from the
  spread on its own; `required` stays a literal on each radio.
- [ ] **Tests that flip, on purpose** (Claude):
  `a_numeric_bound_reaches_the_input` ([constraint_attrs.rs:73](tests/suite/constraint_attrs.rs#L73))
  stops seeing `min=1` on `type="text"` (still server-checked; give it a
  `widget: number` twin that DOES see it); `no_step_is_emitted_for_a_float`
  moves to `widget: number` for the same reason; D3's chooser test; the
  "Emitted even where HTML says they do not belong" comment block above them
  is rewritten, since issue #4 is what it describes.
- [ ] **New tests** (Claude): a required-true radio group emits no `required`
  on the `<fieldset>`; a quoted key is never filtered; a `with_attrs` spec
  with an invalid key loses it at render (pins the filter itself, since
  `form!` can no longer produce one).
- [ ] **Acceptance:** the gallery's rendered HTML through the W3C validator
  with no attribute errors (it flagged 50 of 70 pairs when #4 was filed);
  `just e2e` green; close #4.

### 4d. The `Widget` trait for custom widgets

- [ ] **The trait**, per the design note and D4: `trait Widget { const ATTRS:
  AttrSet; fn class() -> …; fn render(props: WidgetProps) -> Element; }`, with
  `AttrSet::ANY`. BREAKING: `custom(MarkdownWidget)` names a type
  implementing `Widget`, not a component fn.
- [ ] **`form!` passes attributes.** The closure in `WidgetRef::path`
  ([widget.rs:154](../formoxus-macros/src/form/widget.rs#L154)) passes
  `values` and `props` only; `WidgetProps.attrs` is already filled by
  `ScalarWidget`'s `Custom` arm ([scalar.rs:101](src/widgets/scalar.rs#L101)),
  so this is the gap.
- [ ] **`form!` checks keys against `ATTRS`**: `const _: () =
  assert!(<W as Widget>::ATTRS.contains(Attr::X), …)`, evaluated in the
  consumer's crate where the impl is visible.
- [ ] Tests (Claude): `spreading_widget` in `author_attrs.rs`
  ([:152](tests/suite/author_attrs.rs#L152)) moves to `form!`; the
  `custom(…)` hand-built test goes; goldens for a key outside `ATTRS` and for
  `ANY` letting one through; the custom widget in the examples crate updated.

### 4e. The dioxus-html cross-check test

- [ ] Per the design note ("TODO: cross-check"): every (attribute, element) the
  table allows has a matching `dioxus::html::elements::<el>::<attr>` const;
  known gaps listed as exceptions. In `formoxus`, not `formoxus-spec`.
  Independent of 4a–4d; can go any time.

## Then, in order

1. **Two enums for input types** (decision 6): `WidgetType::Input` takes a
   smaller enum of what `Input` renders; `Input` becomes the router for every
   `<input>` (decision 5), and `WidgetType::Checkbox` goes away.
2. **Reshape `FieldProps`**: `required` and `aria_invalid` MOVED INTO 3d
   (2026-10-08). What is left here: leave room for a blur-validation callback.
3. **`form!` is the only door** (decision 7): builders, `FormField`'s fields and
   `ScalarWidget` behind `#[doc(hidden)] __private`; about 210 suite call
   sites move to `form!` or `__private`. Last, because it touches the most.
4. **Split `fields.rs`**, if the value-kind half is still large (it mostly
   left in step 2).
5. Then back to the MVP list: #9 `aria-describedby` (with `<label for>`),
   help text, per-field validators, and onward
   (`.claude/memory/mvp_scope.md`).
