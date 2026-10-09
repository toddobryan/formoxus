# Attributes: the build order from here

A working checklist for the attribute-table work (C6, which absorbed issue #4).
**The reasoning behind every decision is in
`.claude/memory/attribute_rules_design.md`**, numbered; this file is just the
order to do things in and where to do them. Claude keeps it current when asked.

> **Status, 2026-10-09.** Step 3 is DONE, every box (3f's last box today).
> 3d's widget side finished today too (steps 4–10). Open from 3f: `form!`
> refuses typed keys on an enum field; Todd wants them allowed (see 3f). Deferred: the
> `RadioGroup` author-attribute grammar, and custom widgets getting author
> `class`. **Step 4 (emit only valid attributes, the `Widget` trait) has not
> started.**
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
>   ([attrs.rs:19](../formoxus-attrs/src/attrs.rs#L19)) is unused and goes.
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
  DONE 2026-10-09 (Claude). Five tests stay hand-built, each saying why: the
  `From` impl, `custom(…)` (drops attrs), the two raw-`maxlength` hazards
  `form!` refuses, and `class` on an enum field. **Finding:** `form!` refuses
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
- [x] Acceptance: every existing golden byte-identical except the deliberate
  grammar change; `just e2e` green. MET as of `534057b` (goldens pass; e2e 29).

---

## Step 4: emit only valid attributes, and the custom-widget trait

Closes issue #4. Decisions 6, 10, and the custom-widget section of the note.

- [ ] Map each widget (and `InputType`) to its `FieldControl`, at expansion time
  for named widgets.
- [ ] **Compile-time**: `form!` asserts `Attr::is_allowed(field_type, control)`
  per attribute. Refused → error naming the attribute and the widget.
- [ ] **Runtime**: `constraint_attributes` emits only where `is_valid_on`
  (so `min` stops appearing on `type="text"`; it is still checked).
- [ ] ~~`RadioGroup` routes each attribute to the radios or the `<fieldset>` by
  asking `is_valid_on`.~~ SUPERSEDED 2026-10-09: `required`/`aria-invalid`
  are placed by the specs (3d step 8), and validity cannot decide `class`,
  `style`, `disabled` or `data-*`, which are valid on both. Author attributes
  wait on the deferred `form!` grammar (`radio_group { choices, each: {…} }`).
- [ ] The **`Widget` trait** for custom widgets (`const ATTRS: AttrSet`,
  `AttrSet::ANY` allowed); custom widgets receive attributes.
- [ ] The **dioxus-html cross-check test** (design note, "TODO: cross-check").

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
