# Custom fields (#28, FIELDS_PLAN Step 6)

Written 2026-11-23 by Claude. **Decisions A to I DECIDED by Todd on 2026-11-24, all
as recommended** (F's exact route is still worked out in box 2's tests, as F says).
**2026-11-25: A, B, D, E, F, G and the proxy design REVERSED.** Todd: no
formoxus-specific attributes on models, not even `formoxus::proxy`. A type gets
its field from an app-wide registry installed in `main` (box 5, decisions M to
Q), proven to reach server functions. Sections below that describe the proxy are
kept as history and marked.
Also 2026-11-24: this lands before A1 on the road to publish
(`.claude/memory/mvp_scope.md`). Decided the same day: this comes BEFORE the storage rewrite
(`table!`, in a repo of its own), and assumes that rewrite's storage-agnostic
`Ref<T>`. It replaces `FIELDS_PLAN.md`'s Step 6, which now points here.

**Why now.** apcsp's `create_assignment` (ANSWERING_PLAN 2h, apcsp `51a055a`) needed
hand-written converters from the form's types to the model
(`api/src/assignment/form.rs`: `SettingsForm::into_settings`,
`GroupForm::into_question_group`, `From<LatePolicyForm>`), and editing an assignment
(2k) would need the same in reverse. Todd decided on 2026-11-23 to make custom
fields work in formoxus first, so that `AssignmentData` can itself be the form model
and neither direction has to be written. It adds `form!` grammar (`field:`), so by
`.claude/memory/mvp_scope.md`'s principle 2 it lands BEFORE A1 (the grammar docs),
on the road to publish.

Who does what: Todd writes the runtime and decides; Claude writes the tests (first,
with no implementation behind them), the `form!` side, the docs, the memory, and
the suite and examples follow-ups. This is planned as one change on a branch, with
the compiler errors as the to-do list.

## Why

1. **A type cannot bring its own behaviour.** apcsp's question form names
   `widget: MarkdownWidget` on five fields, and every future form with a `Markdown`
   field will need it again. Forgetting it silently gives a plain text input.
2. **A type cannot have its own text form.** A time limit wants to hold seconds and
   be typed as `1h15m`. A newtype's wire value is its inner value, parsed by the
   inner type, so `TimeLimit(u32)` expects `"4500"`. apcsp works around it with an
   `Option<String>` field, a `validator:` that parses with `jiff`, and a conversion
   in the server handler: three places that know the format, repeated for every
   such field.
3. **A leaf whose type formoxus does not know cannot be in a form at all.**
   `Ref<Question>` (surreal-table's typed record link) inside
   `groups[].questions[]` is the case apcsp needs. Today `node_for_shape` panics
   on it ("unsupported type"), or builds a fieldset around its private fields.
4. **`field:` does not exist in `form!`,** although `FIELDS_PLAN.md` built the
   `Field` trait (Steps 1 to 5) so that a dev could write one. A dev's own field,
   like the `ProgramSnippetField` from the design note, cannot be named yet.

## What facet 0.46.5 gives (probed 2026-11-23)

The full findings are in `.claude/memory/facet_proxy_hook.md`. In short:

1. **`#[facet(formoxus::proxy = String)]`**, with `TryFrom<String> for T` and
   `TryFrom<&T> for String`, gives a type its own text form. It is stored on the
   type's `Shape` as a format-specific proxy, readable as
   `shape.format_proxy("formoxus")`, including on a struct field's shape and a list
   item's shape. **Only formoxus reads it**: facet-json and other formats see no
   proxy. **The model's crate does not need to depend on formoxus**, because facet's
   derive handles every `ns::proxy` itself, and the namespace is only a string.
2. **A formoxus-defined attribute, `#[facet(formoxus::field = some_fn)]`**, works
   too (`facet::define_attr_grammar!` with an `fn_ptr` variant). It is found from
   the `Shape` the same way, on generic types too (`Ref::<T>::field`, resolved per
   `T`). But the model's crate must then depend on formoxus, because the attribute
   expands to `formoxus::__attr!`.
3. **facet's derive never wires `FromStr` into the vtable's `parse`.** A dev's
   `FromStr` alone is invisible to formoxus; only the proxy or a hand-written
   `Facet` impl changes how a type parses.
4. **Dioxus 0.7.10's launch contexts do not reach server functions.**
   `LaunchBuilder::with_context` feeds the client root and the server-side render,
   and nothing else. `Submission::accept` runs inside a server function, so
   anything it needs (parsing, `check`, `check_on_server`) cannot live only in an
   app-wide context.

## apcsp's crates decide a lot

| Crate | Depends on | Holds |
|---|---|---|
| `models` | facet, serde, surreal-table | `Markdown`, `AssignmentData`, the future `TimeLimit` |
| `surreal-table` | serde only (no facet) | `Ref<T>` |
| `ui` | formoxus, models | `MarkdownWidget` |
| `api` | formoxus, models, surreal-table, ui | the forms and server functions |

Three consequences:

1. **`Markdown` cannot name `MarkdownWidget`.** The widget lives in `ui`, which
   depends on `models`. Whatever says "a `Markdown` renders with `MarkdownWidget`"
   has to live in `ui` or `api`, not on the type. The app's formoxus configuration
   is the natural place, and since a widget matters only while rendering, a
   client-side context is enough for it.
2. **`formoxus::field` would make `models` depend on formoxus,** and so on Dioxus.
   It does not today. `formoxus::proxy` would not.
3. **Today's `Ref<T>` is the wrong one to make a form field.** It wraps a
   surrealdb `RecordId`, which has no `Facet` impl, and lives in a crate with no
   facet at all. Decided 2026-11-23: the SurrealDB work moves to its own repo, as
   formoxus did, and models become plain `Facet` structs with storage done by a
   separate `table!` (`.claude/memory/storage_separate_from_models.md`). So this
   plan assumes a **storage-agnostic `Ref<T>`**: a typed key that derives
   `Facet` with `#[facet(formoxus::proxy = String)]` and names no database, which
   the storage layer maps to a `RecordId`. Where it lives is decision I.

## The design (draft)

A custom type gets its behaviour from up to three places, chosen by who needs to
see it.

**SUPERSEDED 2026-11-25** by box 5's registry: points 1 and 2 now both live in
the app's registry, installed in `main`, and point 3 stands.

1. **Its text form, on the type: `formoxus::proxy`.** Server-reachable for free,
   with no dependency on formoxus. formoxus treats any shape with a formoxus proxy
   as ONE leaf, the way it treats a newtype today. The field carries the PROXY type
   (`String`), because a concrete `T` cannot be recovered from a runtime `Shape`
   (the comment on `scalar_node`'s `wrapper` parameter). The proxy's `convert_in`
   runs where the value is read, and a failure becomes the field's error message.
   Its `convert_out` refills the input from an existing value. This generalises
   `FieldNode::wrapper`, which already re-applies a newtype's wrapper when the
   value is written back.
2. **Its default widget, in the app's configuration:** something like
   `Formoxus::new().with_widget_for::<Markdown>(MarkdownWidget)`, keyed on the
   type's `Shape`. It is presentation only, so living in a Dioxus context the
   server never sees is fine. A per-field `widget:` still wins. This is what is
   left of `.claude/memory/widget_registry_idea.md`, and it rides the config
   cascade (`.claude/memory/config_cascade.md`).
3. **Anything more, for one path: `field: X` in `form!`.** A dev's own `Field`,
   such as `ProgramSnippetField`, with its own `parse`, `check`,
   `check_on_server` and default widget. The spec is built on both sides, so this
   reaches the server. This is the old Step 6's first three boxes.

**The init Todd asked for (2026-11-23)** follows Dioxus's app creation:
`dioxus::LaunchBuilder::new().with_context(Formoxus::new().with_classes(…).with_widget_for::<Markdown>(MarkdownWidget)).launch(App)`.
`provide_defaults` is already `provide_context(config)` and `defaults()` is
already `try_consume_context`, so this probably works today if `Formoxus` is
`Send + Sync`. A test will say. A type → FIELD map in the same config is what
point 4 above rules out, unless the dev also hands the config to
`Submission::accept` (decision B).

**`Field::format`** (the old Step 6's fourth box) comes along: the reverse of
`parse`, defaulting to facet's display, as `FieldNode::raw_value` does today. A
proxied leaf formats through `convert_out`. A dev's own field must be able to
refill its input from an existing value, which is the whole of apcsp's edit page.

## Decisions

A. **SUPERSEDED 2026-11-25 by M.** **A proxied type is a built-in leaf.** Any shape with a `formoxus` proxy to
   `String` becomes a text leaf whose field carries the `String`, checked through
   `convert_in`. Recommended, because it is the only mechanism here that needs
   nothing from formoxus in the model's crate. Alternatives: no proxy support,
   leaving `field:` as the only route, so `TimeLimit` needs a `field:` on every
   path that holds one.

B. **DECIDED 2026-11-25: (c), without the passing.** The app's registry,
   installed once in `main`, which the server sees because it runs the same
   `main` (box 5.1). **Where a type → field default lives, when one is wanted at all.**
   (a) Nowhere for now: the proxy covers text form, `field:` covers the rest.
   (b) `#[facet(formoxus::field = …)]` on the type, which is server-reachable but
   makes the model's crate depend on formoxus (apcsp's `models` does not).
   (c) The app configuration, with the dev passing it to `Submission::accept` as
   well, so the server sees it too.
   Recommended: (a), and revisit with a real case. apcsp has none that the proxy
   and `field:` do not cover. (b) is proven to work if a case appears.

C. **One registration or three (#28's question C).** With A and B(a), the answer
   is: the type declares its text form, the app declares its default widget, and a
   path declares its field. Each lives where the thing it needs is visible.
   Confirm, or pull them together.

D. **MOOT 2026-11-25 (no proxy).** **The proxy's error text.** facet wraps the dev's message: "Custom
   deserialization of shape 'String' into 'TimeLimit' failed: "90": expected like
   1h15m at \<root>". Recommended: call `ProxyDef::convert_in` directly, which
   hands back the dev's own `String`, rather than going through `Partial`. That
   means a few lines of `unsafe` in formoxus around an uninitialised slot.
   The other route is to unwrap the error kind from `ReflectError`.

E. **MOOT 2026-11-25 (no proxy).** **Proxy types other than `String`.** A `u32`-proxied type could keep a number
   input. Recommended: `String` only at first, with the others refused by a clear
   panic naming the type. Everything apcsp needs is text.

F. **SUPERSEDED 2026-11-25 by Q.** **`choices:` on a proxied leaf** (`Ref<Question>` in `groups[].questions[]`).
   The field carries the proxy's `String`, so the `(id, label)` choices apcsp
   passes today (`COURSE_QUESTIONS` in `tests/suite/assignment_shape.rs`) would
   compare as text unchanged. But `form!`'s `choice_field_for` requires
   `P: DefaultField` of the MODEL type, and a storage-agnostic `Ref<Question>`
   cannot implement it without its crate depending on formoxus (orphan rule),
   which is the dependency the proxy exists to avoid. Options: `form!` routes a `choices:` on an
   unknown type to a text choice field keyed by the proxy, with a `const _` check
   that the shape has a proxy; or a `ChoiceField` typed by the proxy. To work out
   in box 2's tests.

G. **SUPERSEDED 2026-11-25 by N (retire the unwrapping now).** **The newtype path.** `FIELDS_PLAN.md`'s decision E was to retire the runtime
   newtype unwrapping once `field:` exists. With A, a newtype could instead keep
   working as it does (`Markdown` needs nothing more than that), or become "a
   newtype is proxied by its inner value". Recommended: leave newtypes alone in
   this plan. One wrapper type in `FieldNode`, `enum Wrapper { Newtype(&Shape),
   Proxy(&ProxyDef) }`, is a fair place for them to meet.

H. **Holder changed 2026-11-25 by P:** the registry, not `Formoxus`; the
   order stands. **The default widget's key and holder.** `with_widget_for::<T>` keyed on
   `T::SHAPE`, held in `Formoxus`. `Formoxus` derives `Clone + PartialEq` and
   must be `Send + Sync` for `with_context`, so the stored widget must be too.
   Checked 2026-11-24: `Formoxus` is `Send + Sync` today, and `AnyWidget` is
   already `Clone + PartialEq + Send + Sync`, which is everything
   `with_context(impl Any + Clone + Send + Sync)` asks. Also decide the order: the per-field `widget:`,
   then the app's per-type widget, then the field's `default_widget`.

I. **Where the storage-agnostic `Ref<T>` lives, and what its key is.** It must
   be something `models` can depend on with no database behind it. Candidates:
   the core crate of the new storage repo (the one a SurrealDB backend and any
   other backend both build on), or a tiny crate of its own. The key could be a
   `String`, which is what the proxy and `choices:` see, or a small key type the
   storage layer interprets. Recommended: the storage repo's core crate with a
   `String` key, decided when that repo is planned. This plan needs only its
   shape: `Ref<T>` derives `Facet`, gets a field from the registry (since
   2026-11-25; it was a `String` proxy before), and round-trips as
   `"question:abc"` or whatever the storage layer prints. Until then, the
   suite's tests use a stand-in `Ref<T>` of exactly that shape.

J, K, L. **Decided 2026-11-25 as recommended, then overtaken the same day.**
   Their full text is in box 5 and 6 of `a8463e0`. J (`form!` reaches a type
   with no `DefaultField` by the autoref dispatch `validator:` uses): the
   dispatch stands for `choices:`, its fallback is now Q's, and its settings
   fallback is dropped (O.1). K (`LeafArgs` gains the proxy): moot. L (the
   per-type widget is render-only, four built-ins refused): superseded by P.

## Traps

**Traps 1 to 3 and 6's proxy half are moot since 2026-11-25** (no proxy). 4 and
5 still hold; 5 is why the registry is a process-wide static.

1. **`begin_custom_deserialization_with_format` is the wrong method.** It reads a
   FIELD-level proxy and fails at the root with "not currently processing a
   field". The container-level one is
   `begin_custom_deserialization_from_shape_with_format(Some("formoxus"))`.
2. **`field_kind::kind` is a `const fn`, and it decides what `form!`'s compile-time
   checks allow.** A proxied type must classify as text there, or every `min:` and
   `widget:` check will refuse it. `format_proxies` is a `&'static` slice on the
   shape, so a const scan using the crate's `str_eq` should work. PROVEN
   2026-11-24 (facet 0.46.5): a `const fn` scanning `shape.format_proxies` for
   `"formoxus"` is true for a proxied newtype, false for a plain one, and true
   through `Option<T>` once `shape.inner` is peeled, all in a `const _` assert.
3. **`""` is absence** (`.claude/memory/facet_form_design_decisions.md`). A
   `convert_out` that returns `""` must read as empty, as a `String` does, and
   `populate`'s check has to go through the new `format`.
4. **The named-field check has to move to the top of `node_for_shape`.** Today the
   spec's constructor is consulted only inside `scalar_node`, so a `field:` on a
   struct- or list-shaped path would build a fieldset and never reach the field.
   The proxy check has to come before the struct arm for the same reason: a
   proxied `Ref<T>` IS a struct.
5. **Dioxus contexts do not reach server functions** (see above). Nothing the
   server needs may live only in `Formoxus`.
6. **`Option<TimeLimit>`** must peel to the proxied leaf exactly as
   `Option<Markdown>` peels to a newtype, and the `optional` flag stays
   payload-only (`.claude/memory/optional_flag_is_payload_only.md`).

## Boxes

- [x] **1.** (Todd) Decide A to I. Done 2026-11-24: all as recommended.

- [x] **2.** (Claude) Tests first, on a branch, with no implementation:

  1. A `TimeLimit` proxied to `String` in the suite: it renders one text input,
     parses `1h15m`, refuses `90` with the dev's own message, refills from an
     existing value, and the same works inside a list item and an enum variant.
  2. `Option<TimeLimit>`: empty is `None`.
  3. `Submission::accept` parses and refuses the same values from raw wire text.
  4. A `field: ProgramSnippetField` on a path, from outside formoxus in the suite
     (a textarea whose text must "compile"), including on a struct-shaped path
     (trap 4).
  5. The app's per-type widget: a `Markdown`-like newtype rendering with a
     custom widget with no `widget:` on the path, a `widget:` overriding it, and
     `with_context` at launch reaching it (as far as a suite test can reach a
     launch).
  6. `choices:` on a proxied list item, once F is decided.
  7. Trybuild goldens for `field:`'s compile errors (an unknown field type, a
     field whose `KIND` does not fit the widget).

  **As built (2026-11-24, branch `custom-fields`).** `formoxus/tests/suite/custom_fields.rs`,
  48 tests in four sections (proxy, `field:`, the app's per-type widget, `choices:` on
  a proxied item), wired into `suite.rs`. The suite does NOT compile until boxes 3 to 6
  exist, by design: today exactly 15 errors, all missing API (7 `field:` keys, 3
  `Field::KIND`, 2 `Field::format`, 2 `with_widget_for`, 1 `choices:` on `Ref` needing
  `DefaultField`). To get the rest of the suite back as a regression net while building,
  comment out `mod custom_fields;` in `suite.rs`. Goldens 2.7 are three cases in
  `formoxus/tests/ui-pending/` with no `.stderr` (`compile_fail.rs`'s `ui_pending`):
  not a `Field`, a `Value` that is not the path's type, a `KIND` the widget cannot
  render. **API the tests chose, for Todd to accept or change:**

  1. `Field::KIND: FieldType`, a required associated const: what `form!` checks a
     `widget:` against when `field:` is given. Needed because a dev's field may read a
     struct (`Span`) from text, so the model type's own kind is the wrong one.
  2. `fn format(&self, value: &Self::Value) -> String` on `Field`, defaulting to
     facet's display.
  3. `field:` takes an EXPRESSION (`ProgramSnippetField { max_lines }`, a local
     captured), evaluated once where `form!` is written; `NodeCtor`'s closure clones it.
  4. `Formoxus::with_widget_for::<T>(impl LeafWidget)`.
  5. Decision H's order is pinned by
     `the_apps_widget_for_a_type_beats_a_named_fields_default_widget`: the app's
     per-type widget beats a `field:`'s `default_widget`. Flip that test if it reads wrong.
  6. A non-`String` proxy panics with a message containing the type's name
     (`should_panic(expected = "Minutes")`); wording left to box 5.

  Not covered: `check_on_server`, which nothing in formoxus calls yet. F's ROUTE is
  still open; the tests pin only what `form!` must do (`choices:` on
  `questions[]: Vec<Ref<Question>>` checks the proxy's text, in the page and in
  `Submission::accept`).

- [x] **3.** (Todd) `Field::KIND`, `Field::format`, and the named field winning at
      the top of `node_for_shape`. Expanded 2026-11-24 by Claude. Everything here
      is in `field.rs`, `fields/`, `nodes/field_node.rs` and `build.rs`; nothing in
      `form!` changes yet (that is box 4).

  **Done, 2026-11-24 evening.** 3.1 to 3.5 all built (Todd 3.1 to 3.4; Claude
  the docs on `KIND` and `format` and 3.5's 7 tests in
  `tests/suite/spec_constructors.rs`). `scalar_node` now takes `(scalar, args)`:
  `node_for_shape` looks the spec up once and builds every leaf's `LeafArgs`
  through one closure. The 3.4 assert reads "mismatched types at {path},
  expected {path's type}, given {field's Value}". Lib 276, suite 970, goldens,
  clippy and fmt all clean, with `mod custom_fields;` still commented out.

  **How to check it while the suite does not compile.** Comment out
  `mod custom_fields;` in `tests/suite.rs`. Then `cargo test -p formoxus --lib` and
  `cargo test -p formoxus --test suite` are the regression net, and the goldens
  (`--test compile_fail`) should not move. Put the line back at the end: the 3
  `KIND` and 2 `format` errors in `custom_fields.rs` should be gone, leaving the 7
  `field:` keys, the 2 `with_widget_for`s and the 1 `choices:` on `Ref`.

  - [x] **3.1** Add `const KIND: FieldType` to the `Field` trait, with no default.

    Why it is needed: `FieldNode::field_type()` (`field_node.rs`, around line 157)
    works out the field's kind from `F::Value`'s shape, and `expect`s that shape to
    be a scalar. A dev's field may hold a struct that it reads from text
    (`SpanField`, whose `Value` is `Span`), so that `expect` would panic the first
    time such a field rendered or checked a bound. The field has to say what kind
    of text it reads. `form!` also needs it, in box 4, to check a `widget:` written
    next to `field:`: a `const fn` can read an associated const, but it cannot call
    a method.

    Why it has no default: as with `default_widget`, a dev writing a `Field` should
    choose, not inherit `Text` without noticing. The tests already write it on
    every field they define.

    1. Built-in fields: `StringField` is `FieldType::Text` and `BoolField` is
       `FieldType::Bool`. `NumberField<T>` is `T::KIND`, which `Number` already has
       (`number_fields.rs:30`).
    2. `ChoiceField<T>` is generic over any `Facet` type, so its `KIND` has to be
       worked out from `T::SHAPE` at compile time. Add a `pub(crate) const fn
       field_type_of(shape: &Shape) -> FieldType` to `field_kind.rs` that maps
       `kind(shape)`'s `Text`, `Int`, `Float` and `Bool` across, and `panic!`s on
       anything else. That panic can only show up as a compile error, because
       `choice_field_for` requires `DefaultField`, so its `T` is always a scalar.
       Then `const KIND: FieldType = field_type_of(T::SHAPE);`.
    3. Make `FieldNode::field_type()` return `F::KIND`, and delete the scalar
       `match` and its `expect`. Fix the doc comment above it, which says the kind
       is "derived from `F::Value` on every read".
    4. Fix the one `Field` impl in the suite: `CapField` in
       `tests/suite/spec_constructors.rs` gets `const KIND: FieldType =
       FieldType::Int;`. (The ones in `custom_fields.rs` and `tests/ui-pending/`
       already have theirs.)
    5. Document `KIND` on the trait: what it is used for, and that it describes
       the field's TEXT, not its `Value` (a `SpanField` is `Text`, though `Span`
       is a struct).

  - [x] **3.2** Add `fn format(&self, value: &Self::Value) -> String` to `Field`,
    the reverse of `parse`.

    Why: a field that parses text its own way has to write the text back the same
    way, or editing an existing value refills the input with something the field
    would refuse. Today `FieldNode::raw_value` formats with
    `Peek::new(t).to_string()`, which for a `Span` would not be `3-7`. This is the
    whole of apcsp's edit page.

    1. The default body is `Peek::new(value).to_string()`, exactly what
       `raw_value` does today, so every built-in field keeps its current text with
       no override.
    2. `FieldNode::raw_value` (around line 266) calls `self.field.format(t)` for a
       `Valid` value. `shown()` and `collect_values` go through `raw_value`
       already, so they follow.
    3. Trap 3, `""` is absence: `populate` (`field.rs`, the end of the file) decides
       emptiness with `p.to_string().is_empty()`, the facet display, not the
       field's own text. Give it the field: `populate(&field, peek)`, do the
       `p.get::<F::Value>()` first, then return `Empty` when
       `field.format(&value).is_empty()`. In `LeafArgs::node` the field is in hand
       before `value` is built, so pass `&field` there. The comment above the `""`
       arm explains why the check compares the display string; change "display"
       to the field's own `format`, since that is now the string `raw_value`
       emits.
    4. `ChoiceField`'s private `raw()` (`choice_field.rs`, around line 101) also
       formats with `Peek::new(value).to_string()`, to turn each choice's value
       into the text a widget submits. It must agree with `raw_value`, because
       `shown()` finds a choice by comparing the two. Nothing overrides `format`
       on `ChoiceField`, so the two agree today. Either leave it, or make
       `as_choices` call `self.format(value)`, so they cannot drift apart. Claude
       recommends the second; it is a few lines.
    5. Document `format` on the trait, saying it must round-trip with `parse`:
       `parse(&format(v)) == Ok(v)`.

  - [x] **3.3** A named field wins over any shape: move the constructor check from
    `scalar_node` to `node_for_shape`.

    Why: today `scalar_node` (around line 535) is the only place that reads the
    spec's `node`, so a constructor named on a struct-, list- or newtype-shaped
    path is never seen. The walk builds a fieldset for `span` instead of handing
    it to `SpanField` (trap 4).

    1. In `node_for_shape` (around line 374), put the check right AFTER the
       `Option` arm and BEFORE the file arm. It must come after `Option`, because
       `option_node` peels the `Option` and calls `node_for_shape` again with the
       same path and `optional: true`. There, the constructor sees the inner
       shape and the inner peek, and its field's `Value` is `Span`, not
       `Option<Span>`, which is what `choice_field_for` on an `Option<u8>`
       already counts on. Checked before `Option`, it would be handed an
       `Option`'s peek.
    2. The check builds `LeafArgs { name, path: prefix, peek, optional, wrapper:
       None, spec }` and returns `ctor.build(args)`. `prefix` is already the leaf's
       full path, as the comment in `scalar_node` says. `wrapper` is `None`
       because the named field holds the path's own type: a `field:` on a `Note`
       path gets a `Note`, not the `String` inside it.
    3. Delete the check from `scalar_node`. `scalar_node` then only builds the
       built-in leaves, and its doc and the comment above the deleted lines, which
       describe the constructor, move to the new check.
    4. Nothing that works today changes. Every constructor `form!` emits comes from
       `field_for` or `choice_field_for`, and both require `DefaultField`, so they
       only ever sit on scalar paths, maybe inside an `Option`. A set's items go
       through `node_for_shape` too (`build.rs` around line 748), so `choices:` on
       set items still reaches them.

  - [x] **3.4** (Recommended, optional) Panic clearly when a named field's `Value`
    is not the path's type.

    Why: `form!` will refuse a mismatch at compile time (box 4, the
    `form_field_value_mismatch` golden), but a hand-built
    `FormSpec::with_node(path, ctor)` cannot be checked that way. Today such a
    mismatch shows up only when the form is FILLED, as `populate`'s `expect`
    ("scalar_type matched, so this get should be the right type"), which names
    neither type. `empty_form` never notices at all.

    1. Give `LeafArgs` a `pub shape: &'static Shape`, the shape the walk is
       building at that path: the path's own shape at the new check, and the INNER
       scalar's shape in `scalar_node`'s `dispatch!` for a newtype, since that is
       what those fields hold.
    2. In `LeafArgs::node`, `assert!` that `F::Value::SHAPE` matches `self.shape`
       (compare `.id`), with a message naming the path and both types.

  - [x] **3.5** (Claude, if wanted) A few `with_node` tests in
    `tests/suite/spec_constructors.rs` for 3.3, with no `form!`: a constructor on a
    struct-shaped path builds one leaf, on an `Option<Span>` path it sees the
    inner value, and on a newtype path it sees the newtype. They prove 3.3 now,
    in the half of the suite that compiles, instead of waiting for box 4's
    `field:`.

- [x] **4.** (Claude) `form!`'s side: a `field:` key, the `const _` checks
      against `<F as Field>::KIND`, and whatever F needs.

  **As built (2026-11-24 evening).**

  1. **`field:` takes a struct expression or a unit struct**
     (`ProgramSnippetField { max_lines }`, `SpanField`), and nothing else: the
     checks need the field's TYPE, which a free `const _` item can name but cannot
     reach through a local, so `form!` reads it off the expression. A call
     (`SnippetField::new(10)`) is refused at the expression
     (`form_field_not_a_struct_expression`). This narrows box 2's API item 3:
     still an expression evaluated at the call site, but only those two forms.
     Every test already wrote one of them.
  2. **It emits `with_node(key, field_kind::named_field(projection, expr))`.**
     `named_field<M, P, F>(_: fn(&M) -> &P, field: F) where F: Field<Value = P> +
     Send + Sync` clones the field into each leaf. The projection goes through the
     path's last `?`, so a `SpanField` fits `maybe?: Option<Span>`. A wrong `Value`
     is rustc's own "expected `&u32`, found `&String`" at the field
     (`form_field_value_mismatch`); something that is not a `Field` gets a new
     `#[diagnostic::on_unimplemented]` on the trait (`form_field_not_a_field`).
  3. **Refused beside `field:`, at the key** (`FieldBody::named_field_refusals`):
     `choices:`, every validated key (`min`, `max`, `min_length`, `max_length`,
     `pattern`, `required_true`: give the setting to the field itself), `no_trim`
     (the field's own `trims`), the list, set and file keys (`set`, `sort_by`,
     `adder`, `item`, `feedback`, `min_items`, `max_items`, `max_size`, `accept`:
     the path is one leaf), a set widget by name, `range` (its required `min:`/`max:`
     are default-field settings), and a path nested in the body. Kept: `label`,
     `help_text`, `validator`, `widget`, and attributes that only render.
  4. **The checks ask `<F as Field>::KIND` where the path's type is asked
     otherwise** (`FieldBody::named_field_checks`): whether each attribute applies
     (`Attr::applies_to(KIND)`), whether a `step` must be whole, and whether the
     widget's `ACCEPTS` holds the `KIND` (`WidgetRef::named_field_checks`,
     `form_field_kind_misfits_widget`). The path's own `Option` still decides
     `ACCEPTS_OPTIONAL`. A dev's widget beside `field:` always goes through
     `with_widget`, never `WidgetSlot`, since the path is one leaf.
  5. **Tests.** 12 macro unit tests (each refusal, the expansion, what is kept),
     `field` in `legal_keys`, 5 goldens (the 3 from `ui-pending/`, now empty, plus
     2). `DefaultField`'s note no longer says `field:` is "not built yet"; 4
     goldens moved by that line only. Box 2 had 5 tests reading errors without
     `validate()` first (Claude's mistake); fixed. With `mod custom_fields;` put
     back, the only errors are box 5's and 6's API (2 `with_widget_for`, 1
     `choices:` on `Ref`); run against a stubbed copy, all 16 `field:` tests pass.
     `just ci` green (1624), with the line still commented out.

- [ ] **5.** (Todd, with Claude on tests and `form!`'s side) The type registry:
      a field per TYPE, registered once in `main`, read by every build on both
      sides. Rewritten 2026-11-25 by Claude, replacing the proxied leaf (the
      2026-11-24 version is in `a8463e0`), after Todd ruled out
      formoxus-specific attributes on models. M, N, O and P DECIDED 2026-11-25,
      Q DEFERRED.

  **Why the change.** `#[facet(formoxus::proxy = String)]` needs no formoxus
  dependency, but it still writes formoxus into the model, and Todd wants the
  model to be just the model (2026-11-25): what formoxus or a storage layer
  needs is added in those layers. Facts about the model that any consumer
  cares about, like bounds and lengths, may still sit on it, in a neutral
  vocabulary (box 10). That leaves the app as the place a type gets its
  field, which the plan had ruled out only because Dioxus contexts do not
  reach server functions (trap 5). A process-wide registry does not need a
  context.

  **Proven 2026-11-25 (5.1).** In a scratch copy of `examples`, `main` set a
  `static OnceLock<&str>` before `launch(App)`, and a `#[post("/api/probe")]`
  server function read it back: the built server binary answered `POST
  /api/probe` with the value set in `main`. The fullstack server runs the same
  `main` in the same process, and the wasm client runs its own `main` before
  `launch` too, so a registry installed there is seen by `use_form` on the
  client, by server-side rendering, and by `Submission::accept`.

  **What goes away.** The proxy and everything built on it: `convert_in`,
  `convert_out`, the `unsafe` slot handling, the `OwnedPeek` lifetime, the
  `String`-only rule, and the `Wrapper` enum. A registered field is an
  ordinary `Field<Value = T>` whose own `parse` and `format` do the text form,
  which is box 3's machinery unchanged: the registry stores the same
  `NodeCtor` that `field:` builds (`field_kind::named_field`).

  **New decisions (2026-11-25, Claude recommends; Todd's answers the same day
  are marked).**

  M. **What the registry is and how it is installed.** Recommended: a type of
     its own, not part of `Formoxus`, built in
     `main` and installed once:
     `TypeRegistry::new().field_for::<TimeLimit>(TimeLimitField).widget_for::<Markdown>(MarkdownWidget).install();`
     then `launch` as today. `install` puts it in a `static OnceLock<TypeRegistry>`
     and panics if one is already installed. Why not inside `Formoxus`:
     `Formoxus` also goes into a subtree's context through `provide_defaults`,
     and a field registered there would reach the client's build but never the
     server's, so client and server would build different forms. Keeping the
     registry a separate type that can only be installed makes that mistake
     impossible rather than documented
     (`.claude/memory/feedback_hard_to_misuse.md`). `field_for::<T, F>` takes
     `F: Field<Value = T> + Send + Sync`, so a field for the wrong type does
     not compile. A generic type registers per concrete type
     (`Ref<Question>` and `Ref<Assignment>` separately), because a `Shape` is
     per instantiation. Alternative: fields inside `Formoxus`, with a rule
     that only the launch-level one counts. **DECIDED 2026-11-25: as
     recommended, named `formoxus::TypeRegistry`.**

  N. **Retire the newtype unwrapping now** (`FIELDS_PLAN.md` decision E,
     deferred by G). Recommended: yes, in this box. The registry is the
     declared way to say what a `Markdown(String)` is, so the guess is no
     longer needed, and `FieldNode::wrapper` disappears entirely, since there
     is no proxy either. apcsp registers two: `Markdown` (five places, inside
     variants and a list item) and `ChoiceId`; `Password` can then be a real
     field instead of a `String` converted by hand (`api/src/auth.rs`).
     `#[facet(transparent)]` is different and stays: it is facet's own,
     consumer-agnostic statement that the type IS its inner type, so honouring
     it is not a guess. An unregistered plain newtype panics at build, naming
     the type and saying how to register it. **DECIDED 2026-11-25: yes.**

  O. **What `form!` can still check on a registered type.** `form!` cannot see
     a registry filled at run time, so:

     1. **Settings stay compile-time.** `max_length: 10` on a path still needs
        `DefaultField`, so on a `TimeLimit` path it is a compile error, whose
        note now says to name a configured field with `field:` instead. The
        registered field is one instance, and `form!` has no way to configure
        it. This drops J's settings fallback.
     2. **`widget:` and table attributes become run-time checks on a struct or
        a plain newtype.** `kind` returns `Unknown` for both instead of
        `Other`, so the `const _` checks let them through, and the build
        asserts what the compiler no longer can: the widget's `ACCEPTS`
        contains the registered field's `KIND`, and a path with a widget or
        attribute but no field (an unregistered struct) panics naming the type.
        The cost is that `widget: textarea` on a plain struct is caught when
        the form is built instead of by the compiler; `form_constraint_on_a_struct`
        and `form_setting_on_a_newtype` change.
     Alternative: keep `Other` for structs and refuse `widget:` on any struct
     path, which forces a `field:` next to every `widget:` on a `Ref`.

     **Todd's two questions (2026-11-25), answered by Claude:**

     a. *Could a type like `Markdown` declare `KIND = Text`, so its text
        attributes are known?* Not on the type itself without writing formoxus
        into the model. Two routes that keep it plain: `#[facet(transparent)]`,
        which `kind` already follows through `shape.inner`, so a transparent
        `Markdown(String)` IS `Text` to every compile-time check today, and
        needs no registered field at all, only a registered widget; or a
        type-level `facetous::text`, which a `const fn` can read (proven in
        10.1). Recommended: `transparent` for `Markdown`, since "this type is
        its string" is a fact every consumer shares. Not `facetous::text` for
        `TimeLimit`: being typed as `1h15m` is the form's concern, and storage
        keeps a `u32`. So a type whose text form is its registered field's
        stays `Unknown` to the compiler.
     b. *Can the checks the compiler cannot make run when the registry is
        installed, so they fail at startup instead of at first use?* Partly.
        `install` sees everything the registry holds, so it can check each
        type that has both a field and a widget (`W::ACCEPTS` contains
        `F::KIND`), and 6.1's const check covers a widget against its type.
        But a per-path `widget:` lives in a form, and a form is built when it
        is first used: `install` never sees it. Reaching those would need
        `form!` to add every spec to a linker-collected list that `install`
        walks (the `inventory` crate), whose wasm support is unverified.
        Recommended: `install` checks the registry; a `FormSpec::check()`
        builds a spec against the installed registry without rendering, so
        each form is one line in a test; `inventory` only if proven on wasm.

     **DECIDED 2026-11-25: O as written (O.1, O.2, a, and `install` checking
     the registry).** `FormSpec::check()` is DEFERRED to #18 (check a model's
     whole shape up front), which is where checking every use early belongs.

  P. **The per-type widget moves into the registry and is chosen at build.**
     Recommended. With the registry seen on both sides, the server can pick
     the same widget the client does, so the widget is chosen once, in
     `LeafArgs::node`, in decision H's order: the path's `widget:`, then the
     registry's widget for the path's type, then the field's
     `default_widget`. This supersedes L: `is_checkbox`, `is_password` and
     `value_when_empty` read the chosen widget on both sides, so nothing has
     to be refused. Box 6 shrinks to two steps. **DECIDED 2026-11-25.**

  Q. **`choices:` on a registered type** (`Ref<Question>` in
     `groups[].questions[]`, replacing F and J's fallback). J's dispatch stays
     for `DefaultField` types. Recommended fallback, for any other `P`: a
     choice field over `P` whose text comes from `P`'s REGISTERED field, looked
     up at build. Its `format` is the registered field's, and its `parse`
     finds the choice whose text equals the input, so it never needs to parse
     `P` itself. The registry has to hand that text out typed: next to the
     `NodeCtor` it keeps an `Arc<dyn Any + Send + Sync>` holding an
     `Arc<dyn FieldText<P>>` (`format`, `parse`), which the choice field
     downcasts knowing only `P`. A `choices:` on an unregistered `P` panics at
     build. Alternative: defer, since apcsp's ids stay `String`s until the
     storage repo exists (box 8); box 2's `Ref` test would wait in
     `ui-pending` style. **DEFERRED 2026-11-25 (Todd):** 5.7 and Q's part of
     5.8 wait; 5.2 parks box 2's `choices:` over `Vec<Ref<Question>>` test.

  - [x] **5.1** (Claude) Prove a registry filled in `main` reaches a server
    function. Done 2026-11-25, above.

  - [x] **5.2** (Claude) Tests first, rewriting box 2's proxy tests against the
    registry: `TimeLimit` gets a `TimeLimitField` (`1h15m`), the stand-in
    `Ref<T>` a `RefField<T>`, and the `with_widget_for` tests become
    `widget_for` (P). One `fn registry()` in `tests/suite.rs` installs every
    test type behind a `std::sync::Once`, and each test that needs it calls it
    first: the suite is one binary, and `install` is once per process (M).
    Tests that only made sense for a proxy go (the non-`String` proxy panic).
    New: a registered type inside a variant, a list item and an `Option`;
    `field:` beating the registry; an unregistered newtype's panic message;
    a widget whose `ACCEPTS` misses the registered field's `KIND`. The suite
    will not compile until 5.3 exists, by design.

    **As written, 2026-11-26.** `custom_fields.rs` rewritten; `mod
    custom_fields;` is back in `suite.rs`, with `install_test_registry()` and
    a `panic_message()` helper there. The suite fails to compile on 5 errors,
    all the missing `TypeRegistry`; against a throwaway stub of it everything
    else type-checks. With `custom_fields` stubbed out, today's code passes
    970 and fails one: the plain-newtype panic below, which waits for 5.5.

    The API the tests chose, for 5.3 to build or change:

    1. `TypeRegistry` in the prelude, so `formoxus::TypeRegistry`.
    2. `field_for(field)` with no turbofish: the type is the field's `Value`.
       `widget_for::<T>(widget)` names the type, since a widget does not.
    3. A type whose registered widget cannot render its registered field's
       `KIND` panics in whichever of the two builder calls comes SECOND,
       naming the type and the widget's form name (`number`). That is still
       startup, since the registry is built in `main`, and it is testable,
       where a check inside `install` would not be: `install` runs once per
       process.
    4. A second `install` panics with a message containing "installed".
    5. The build-time check of O.2 (a `widget:` that cannot render the
       registered field's `KIND`) names the path and the widget's form name.
    6. 5.5's panic for an unregistered plain newtype names the type,
       `TypeRegistry::field_for`, and `field:`.

    Other changes:

    1. Every test newtype outside this module (`sets`, `trim`, `form_macro`,
       `field_validators`, `set_item_errors`) is now `#[facet(transparent)]`, and
       still passes today.
    2. `newtypes.rs` pins the transparent route, plus the new panic (`Headline`).
    3. Q's three `choices:` tests are parked in `#[cfg(any())] mod deferred_q`.
    4. Not covered: a registered field for a BUILT-IN type (5.4's last
       sentence). The suite is one process, so registering one for `String`
       would change every `String` in it.

    Two traps the tests expose, for 5.4 to 6.2:

    1. **A transparent type's widget is looked up by the OUTER type.** `Note`
       and `Secret` are transparent, so the field carries the inner `String`
       (3.4's assert compares `F::Value` with the inner shape), but the
       registered widget is keyed on `Note`. So the walk has to keep the outer
       shape for 6.2's lookup.
    2. **The transparent write-back needs no stored wrapper.** facet has
       `Partial::begin_inner()` and `Peek::innermost_peek()` for a transparent
       type, so `FieldNode::wrapper` can still go (N), and the write-back works
       it out from the shape.

  - [ ] **5.3** (Todd) The registry (M): its builder methods, `install`, and a
    `pub(crate) fn registry() -> Option<&'static TypeRegistry>` for the build. It
    holds `Vec<(&'static Shape, NodeCtor)>` (a `Vec`: an app registers a
    handful of types; `Shape`'s `PartialEq` compares type ids), and
    registering a type twice in one builder replaces the entry. Its doc says
    the order (P) and that a registered type is found by shape wherever it
    sits: a field, a list item, a variant's field, an `Option<T>`'s inside.

  - [ ] **5.4** (Todd) The registry arm in `node_for_shape`, right after the
    spec's constructor arm and before the file, scalar and newtype arms:
    look `shape` up in `registry()`, and on a hit `return
    ctor.build(leaf(peek, None, shape))`. After the `Option` arm for the same
    reason as the constructor (trap 6). After the constructor because a path
    beats a type (H). Before the scalar arm, so an app can register a field
    for a built-in type, a `String` that always renders as a `MarkdownWidget`.
    3.4's assert stays as a backstop; `field_for`'s bound makes it
    unreachable.

  - [ ] **5.5** (Todd) Retire the newtype unwrapping (N): delete the newtype
    arm, `newtype_inner`, `FieldNode::wrapper`, `LeafArgs.wrapper`, and the
    `begin_nth_field(0)` branch of the write-back. `scalar_node` loses its
    wrapper path. Keep `#[facet(transparent)]` working through `shape.inner`.
    Reword the "unsupported type" panic: name the type and say
    `TypeRegistry::field_for` or `field:`.

  - [ ] **5.6** (Todd) `kind` and the run-time checks (O.2): a user struct and
    a plain newtype give `Kind::Unknown`, with a doc saying the compiler
    cannot see the registry. In `LeafArgs::node`, assert the chosen widget
    accepts `F::KIND` (if `AnyWidget` cannot say, it needs `accepts()`, read
    from the widget's `ACCEPTS` when it is made). The old 5.1's proxy scan is
    not needed.

  - [ ] **5.7** (Todd, DEFERRED with Q) Q's run-time side: `FieldText<P>`, the registry storing
    it, and the choice field over `P` that reads it.

  - [ ] **5.8** (Claude) `form!`'s side: J's dispatch with Q's fallback, O.1's
    note on the settings route, and the goldens that change (O.2). Then the
    suite is green.

- [ ] **6.** (Todd) The per-type widget, chosen at build (P). Rewritten
      2026-11-25 by Claude; the render-time version (6.1 to 6.5 of
      2026-11-24, in `a8463e0`) is superseded with L.

  - [ ] **6.1** (Todd) `TypeRegistry::widget_for::<T: Facet<'static>>(self,
    widget: impl LeafWidget) -> Self`, replacing an existing entry for `T`.
    Check the widget against `T` there with an inline `const {
    assert!(field_kind::renders::<W>(T::SHAPE, false)) }`, which fires when the
    app is built (not under `cargo check`; see
    `.claude/memory/const_shape_walk_blocked.md`).

  - [ ] **6.2** (Todd) Choose in `LeafArgs::node`: `spec.widget`, else the
    registry's widget for `self.shape` (the path's own type: `Note` for a
    `field: NoteField` on a `Note`), else `field.default_widget(optional)`.
    `the_apps_widget_for_a_type_beats_a_named_fields_default_widget` pins the
    middle step. Nothing else changes: the chosen widget is stored in
    `FieldNode::widget` as today, on both sides.

    Trap: a set's draft input renders through the same leaf code, so a
    per-type widget for a set's item type renders in the draft too. That is
    probably right (the draft is where a `Note` is typed); 5.2 has a test.

- [ ] **7.** (Claude) Everything compiles and passes: suite, examples, e2e. A
      gallery example and an e2e pair (a time limit typed as `1h15m` that
      survives a submit). Docs, memory, and a comment on #28.

- [ ] **8.** (Todd) apcsp: `Facet` (and `#[repr(u8)]` on the enums) for
      `AssignmentData`, `AssignmentSettings`, `AssignmentLatePolicy` and
      `QuestionGroup`; a `TimeLimit` with a `TimeLimitField` (`1h15m`); a
      `TypeRegistry` installed in `main` with fields for `TimeLimit`, `Markdown`
      and `ChoiceId` (N) and `MarkdownWidget` for `Markdown`, and the five
      `widget: MarkdownWidget` lines dropped; `Password` as a real field in
      `ChangePasswordForm`; the hand-written converters deleted. `QuestionGroup`'s deferred `Random` and `Streak` variants would
      then show in the form: accept that, or keep a narrower form type.
      **The question ids wait on the storage repo:** `groups[].questions[]`
      becomes the storage-agnostic `Ref<Question>` (decision I) only once that
      repo's core crate exists, and it is registered with a field there (Q). Until then apcsp keeps its ids as `String`s in
      that one place, which `choices:` already checks (`assignment_shape.rs`),
      and converts them where the model is saved. Do NOT add facet to today's
      surreal-table `Ref<T>` in the meantime: `table!` would only undo it.

- [ ] **9.** (Todd) Merge, and close #28.

- [ ] **10.** (Claude, then Todd) **Model constraints in a neutral vocabulary.**
      Asked by Todd 2026-11-25: facts about the model that any consumer cares
      about may sit ON the model, as long as nothing formoxus-specific does.
      Probably its own plan after #28; recorded here because box 5's O.1 and
      the registry are shaped around it. R, S and T DECIDED 2026-11-25. 10.1
      done; 10.2 next.

  **What exists.** `facet-validate` (facet 0.50 only, still a release
  candidate) is facet's own grammar for this, on struct fields:
  `validate::min`/`max` (`i64` only), `min_length`/`max_length`, `regex`,
  `email`, `url`, `contains`, `custom = fn`. Todd found it too thin, and its
  `regex` matches anywhere in the value while HTML's `pattern` matches the
  whole, so browser and server would disagree
  (`.claude/memory/facet_validate_crate.md`).

  **What we would write.** Our own vocabulary in a small crate depending only
  on facet, so the model depends on it and not on formoxus. Not through
  `facet::define_attr_grammar!` but a hand-written `__attr!` (10.1 says why).
  The derive expands `#[facet(facetous::min = 0)]` on a field to
  `facetous::__attr!(@ns { facetous } min { size : u8 | = 0 })`, so the
  namespace is a path the model can name: the crate, or a `use … as …`
  alias. Coverage, from what formoxus validates
  today: `min`, `max`, `min_length`, `max_length`, `pattern` (anchored),
  `min_items`, `max_items`, and `check = fn(&T) -> Result<(), String>` (the
  grammar has a `validator` payload kind for exactly that). formoxus reads
  them at build and folds them into the leaf's field; `ithell` can read the
  same for column constraints.

  R. **Where the crate lives and its name.** Recommended: this workspace,
     published with formoxus, under a name that does not say formoxus.
     **DECIDED 2026-11-25: here for now, probably moved out later; named
     `facetous`** ("like facet, but with some more stuff").

  S. **A model and `form!` setting the same constraint.** Claude first
     recommended that both apply, so a form can only tighten the model.
     **Todd's option (2026-11-25): it is an error.** A constraint on the model
     is one every consumer accepts; if storage and the form need different
     ones, the model leaves it unstated and each layer declares its own. 10.1
     showed `form!` can refuse it AT COMPILE TIME, so Claude now recommends
     Todd's option: a `const _` reads the model field's `facetous` keys off
     the parent's `Field` entry. **DECIDED 2026-11-25: an error, at compile
     time.**
     
  T. **Whether formoxus also reads `facet-validate` once 0.50 is stable.**
     Recommended: no; one vocabulary. **DECIDED 2026-11-25: no** (too
     bare-bones).

  - [x] **10.1** (Claude) Probe. Done 2026-11-25, on facet 0.46.5, in a
    scratch workspace (`facetous`, a `model` crate depending only on facet and
    `facetous`, and a `reader`). Findings:

    1. **The derive hands a field-level attribute the field's TYPE**
       (`$field:tt : $ty:ty`). `define_attr_grammar!` uses it only for
       `predicate` and `validator`; its value kinds fix the payload type
       (`i64`, `usize`), and `make_t` writes the literal with an untyped
       `put`. A hand-written `__attr!` (about ten `macro_rules!` arms) can
       type every value BY THE FIELD instead.
    2. **Bounds are typed by the field.** `min = 1` on a `u8` is stored as a
       `u8`, `-0.5` on an `f64` as an `f64`, and `10_000_000_000` on an
       `Option<u64>` as a `u64`, through a `Numeric` trait whose `Bound` peels
       `Option`. The reader compares a value with its bound through facet's
       `partial_cmp`, knowing only the shapes.
    3. **Every misuse is a compile error at the value or key**, with no
       formoxus involved:
       1. `max = 256` on a `u8`: rustc's "literal out of range for `u8`
          … whose range is `0..=255`".
       2. `min = 0.5` on a `u32`: "expected `u32`, found floating-point number".
       3. `min` on a `String`: "`facetous::min` and `facetous::max` apply to
          a number, and `String` is not one" (`on_unimplemented`).
       4. `max_length` on a `u32`, `min_items` on a `String`: the same kind
          of message from `Text` and `Items`.
       5. `check = f` where `f` takes `&String` on a `u32`: rustc's "expected
          fn pointer `fn(&u32) -> Result<_, _>`".
       6. An unknown key: "facetous has no constraint `minimum`".
    4. **`check = fn(&T) -> Result<(), String>`** is stored type-erased as
       `unsafe fn(PtrConst) -> Result<(), String>`, so a reader holding only
       the shape calls it. It cannot capture the field type of a GENERIC
       struct (an inner `fn` cannot use the outer generics), as facet's own
       `validator` cannot either. Untested: bounds on a generic field.
    5. **The namespace resolves through an alias:** `use facetous as rules;`
       then `#[facet(rules::min = …)]` works; the stored `ns` is the crate's
       own string, `"facetous"`.
    6. **A `const fn` reads them.** A type-level `#[facet(facetous::text)]`
       is found in `shape.attributes`, and a field-level key on the parent's
       `Field` entry (`st.fields[i].attributes`, by name), both in a `const
       _` assert; a false assert fails the build. That is what lets `form!`
       refuse S's duplicate at compile time.
    7. **Not checked at compile time:** a `pattern`'s regex syntax (a `const`
       cannot compile a regex; a small proc macro could, as formoxus-macros
       does with `regress`), and `min` above `max` across two attributes,
       which a per-attribute macro never sees together. Both could be checked
       where formoxus reads them, at install or build.
  - [ ] **10.2** (Claude) Expand into steps (R to T are decided).
