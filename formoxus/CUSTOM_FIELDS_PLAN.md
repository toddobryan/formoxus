# Custom fields (#28, FIELDS_PLAN Step 6)

Written 2026-11-23 by Claude. **Decisions A to I DECIDED by Todd on 2026-11-24, all
as recommended** (F's exact route is still worked out in box 2's tests, as F says).
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

A. **A proxied type is a built-in leaf.** Any shape with a `formoxus` proxy to
   `String` becomes a text leaf whose field carries the `String`, checked through
   `convert_in`. Recommended, because it is the only mechanism here that needs
   nothing from formoxus in the model's crate. Alternatives: no proxy support,
   leaving `field:` as the only route, so `TimeLimit` needs a `field:` on every
   path that holds one.

B. **Where a type → field default lives, when one is wanted at all.**
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

D. **The proxy's error text.** facet wraps the dev's message: "Custom
   deserialization of shape 'String' into 'TimeLimit' failed: "90": expected like
   1h15m at \<root>". Recommended: call `ProxyDef::convert_in` directly, which
   hands back the dev's own `String`, rather than going through `Partial`. That
   means a few lines of `unsafe` in formoxus around an uninitialised slot.
   The other route is to unwrap the error kind from `ReflectError`.

E. **Proxy types other than `String`.** A `u32`-proxied type could keep a number
   input. Recommended: `String` only at first, with the others refused by a clear
   panic naming the type. Everything apcsp needs is text.

F. **`choices:` on a proxied leaf** (`Ref<Question>` in `groups[].questions[]`).
   The field carries the proxy's `String`, so the `(id, label)` choices apcsp
   passes today (`COURSE_QUESTIONS` in `tests/suite/assignment_shape.rs`) would
   compare as text unchanged. But `form!`'s `choice_field_for` requires
   `P: DefaultField` of the MODEL type, and a storage-agnostic `Ref<Question>`
   cannot implement it without its crate depending on formoxus (orphan rule),
   which is the dependency the proxy exists to avoid. Options: `form!` routes a `choices:` on an
   unknown type to a text choice field keyed by the proxy, with a `const _` check
   that the shape has a proxy; or a `ChoiceField` typed by the proxy. To work out
   in box 2's tests.

G. **The newtype path.** `FIELDS_PLAN.md`'s decision E was to retire the runtime
   newtype unwrapping once `field:` exists. With A, a newtype could instead keep
   working as it does (`Markdown` needs nothing more than that), or become "a
   newtype is proxied by its inner value". Recommended: leave newtypes alone in
   this plan. One wrapper type in `FieldNode`, `enum Wrapper { Newtype(&Shape),
   Proxy(&ProxyDef) }`, is a fair place for them to meet.

H. **The default widget's key and holder.** `with_widget_for::<T>` keyed on
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
   shape: `Ref<T>` derives `Facet`, is proxied to `String`, and round-trips as
   `"question:abc"` or whatever the storage layer prints. Until then, the
   suite's tests use a stand-in `Ref<T>` of exactly that shape.

## Traps

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

- [ ] **5.** (Todd, with Claude on `form!`'s side) The proxied leaf:
      classification (trap 2), the node, the check through `convert_in` (D), the
      write-back, and the refill through `convert_out`. Expanded 2026-11-24 by
      Claude. Decide J and K first.

  **Where it stands.** With `mod custom_fields;` back in, the suite fails to
  compile only on box 6's `with_widget_for` (2) and on `choices:` over
  `Vec<Ref<Question>>` (1, decision J). Run against a copy with those stubbed, 21
  tests fail, and all of them are this box's or box 6's. The proxy tests that
  pass today pass by accident: `TimeLimit(u32)` is still read as a newtype over
  its number, which is one leaf, round-trips `4500`, and is required when empty;
  and `widget: textarea` compiles only because the `const _` checks cannot see
  inside a newtype.

  **What facet gives, concretely (facet-core and facet-reflect 0.46.5).**
  `shape.format_proxy("formoxus")` is `Option<&'static ProxyDef>`. A `ProxyDef`
  holds `shape` (the PROXY's shape, `String`'s here), `convert_in` and
  `convert_out`, both `unsafe fn` over raw pointers, returning `Result<PtrMut,
  String>`; the `String` is the dev's own `TryFrom` error, which is decision D.
  `Peek::custom_serialization_with_proxy(def)` runs `convert_out` and returns an
  `OwnedPeek` of the `String`. `Partial::begin_custom_deserialization_from_shape_with_format(Some("formoxus"))`
  is the container-level way in for the write-back (trap 1 names the wrong one).

  **New decisions (2026-11-24, Claude recommends; Todd decides).**

  J. **How `form!` reaches a proxied type that has no `DefaultField`.** Decision F
     left the route open. `choice_field_for` and `field_for` both need `P:
     DefaultField`, which `Ref<T>`'s crate cannot implement (orphan rule), so
     `questions[] => { choices: COURSE_QUESTIONS }` does not compile. Recommended:
     the autoref dispatch `validator:` already uses (`(&&validator_for(…)).validator(…)`,
     `field_kind::validators`). `form!` emits `(&&choices_for(|__m| &place)).choice_field(choices)`;
     the impl on the nearer reference is for `P: DefaultField` and builds today's
     `ChoiceField<ValueOf<P>>`, and the fallback, for any `P: Facet`, builds a
     `ChoiceField<String>` over the proxy's text. A `const _` beside it asserts the
     shape has a `String` proxy, so a struct with neither still gets
     `DefaultField`'s note. The same two impls serve settings (`max_length: 10` on a
     `TimeLimit` gets a `StringField`). This works because the projection makes
     `P` concrete at the call site. Claude's part (`form!` and the two impls).
     Alternative: a key of its own, which a dev would have to know to use.

  K. **Where a proxied leaf learns it is proxied.** Recommended: `LeafArgs` gains
     `proxy: Option<&'static ProxyDef>`, filled by `node_for_shape` from the path's
     shape, and `LeafArgs::node::<F>` decides, since only it knows `F`. If
     `F::Value` is the path's own type, the field holds it: no wrapper, the peek as
     is (a `field:` whose field reads a `TimeLimit` itself). If `F::Value` is the
     proxy's type, `String`, the leaf is proxied: `node` runs `convert_out` on the
     peek and populates from that. Anything else is 3.4's mismatch panic. So
     `StringField`, `ChoiceField<String>` and a dev's `Field<Value = String>` can
     all sit on a proxied path, which J needs. The alternative, deciding in
     `node_for_shape`, cannot see `F`, so a constructor's field would have to be
     guessed.

  - [ ] **5.1** (Todd) Classify a proxied type as text in `field_kind::kind`
    (trap 2).

    Why: `form!`'s `const _` checks call `kind`. Today a proxied `Ref<T>` is a
    struct, `Kind::Other`, so `widget: textarea` and every attribute on it are
    refused. A proxied `TimeLimit(u32)` is a newtype without `inner`, so
    `Kind::Unknown`, which lets everything through, `widget: number` included.

    1. After the `Option` peel and before the list, set, file and
       `module_path` tests, scan `shape.format_proxies` (a `&'static` slice, so a
       `while` loop works in a `const fn`) for an entry whose `format` is
       `"formoxus"` (`str_eq`). Proven in a `const _` on 2026-11-24 (trap 2).
    2. If its `proxy.shape` is `String` (the same `type_identifier` and
       `module_path` test `kind` already makes), return `Kind::Text`. Otherwise
       return `Kind::Unknown`, so the checks let it through and 5.3's runtime
       panic names the type (decision E).
    3. Before the file test is fine: a `FileUpload` has no formoxus proxy.

  - [ ] **5.2** (Todd) `enum Wrapper { Newtype(&'static Shape), Proxy { shape:
    &'static Shape, def: &'static ProxyDef } }` in `FieldNode`, replacing
    `wrapper: Option<&'static Shape>` (decision G). `shape` is the proxied type's
    own (`TimeLimit`), kept for box 6's lookup and for messages. `LeafArgs.wrapper`
    becomes `Option<Wrapper>`, and `LeafArgs` gains `proxy` (K).

  - [ ] **5.3** (Todd) The proxy arm in `node_for_shape`.

    1. Look the proxy up once, next to `spec`: `let proxy =
       shape.format_proxy("formoxus");`. It goes into `leaf(…)` for the
       constructor arm too, which is what lets K work on a `choices:` path.
    2. A new arm right AFTER the constructor arm and BEFORE the file, scalar and
       newtype arms (trap 4: `TimeLimit` is a newtype and `Ref` is a struct, so
       either later arm would claim it): when `proxy` is `Some`, build
       `StringField::default()` through `leaf(peek, None, shape)` with the proxy
       in it, and `LeafArgs::node` does the rest (K).
    3. A proxy whose `def.shape` is not `String` panics there, naming the type,
       its proxy type and the path. The test asks only that the message contain
       `Minutes` (decision E).
    4. `Option<TimeLimit>`, list items and variant fields all arrive through
       `node_for_shape`, so they need nothing more (trap 6, and the "met by
       `Shape` alone" tests).

  - [ ] **5.4** (Todd) The refill, in `LeafArgs::node` when it picks
    `Wrapper::Proxy`: `peek.custom_serialization_with_proxy(def)` gives an
    `OwnedPeek` of the `String`, and `populate` runs on its `as_peek()`.

    Trap: the `OwnedPeek` owns the `String` and the peek borrows it, so it must
    live until `populate` has cloned the value: a local in `node`, not
    something returned from a helper that drops it. A `convert_out` that fails
    (the test's are `Infallible`) has nothing sensible to show; `expect` with
    the type's name. Trap 3 holds unchanged: a `convert_out` of `""` is
    `Empty`, because `populate` now asks `format`, which for a `String` is the
    text itself.

  - [ ] **5.5** (Todd) The check through `convert_in`, in `FieldNode::validate`.

    Why there: the dev's `TryFrom` is the parse, and its error is the message
    the user must see (the tests compare it exactly), so it has to run where
    errors are collected, before anything is written. The `String` itself is
    always valid text, so `parse` cannot catch it.

    1. When `value` is `Valid(text)` and the wrapper is `Proxy`, run
       `convert_in`: allocate `wrapper.shape` (`shape.allocate()`), call
       `(def.convert_in)` with a `PtrConst` to the `String` and that slot, and on
       `Err(message)` push
       `ValidationMessage(message)`. Decision D, so no facet wording.
    2. **Trap: the scratch value must be dropped and freed on BOTH outcomes.** On
       `Ok` it is a live `TimeLimit` (a `Ref` owns a `String`): drop it in place,
       then deallocate. On `Err` nothing was built: deallocate only. Facet's own
       `custom_serialization_with_proxy` (`peek/value.rs:983`) is the pattern to
       copy for the `Err` side. Getting `Ok` wrong leaks, and dropping on `Err`
       is undefined behaviour.
    3. Run it after the field's own `check`, and only if that found nothing,
       so a `ChoiceField<String>` refusing a key outside the list says only that.
    4. `Submission::accept` validates the same nodes, so the server tests need
       nothing more.

  - [ ] **5.6** (Todd) The write-back, in `write_value_into`: for
    `Wrapper::Proxy`, `p.begin_custom_deserialization_from_shape_with_format(Some("formoxus"))?`,
    take `.0`, `set` the `String`, then `end()`. `end` runs `convert_in` again;
    5.5 has already shown it succeeds, so an error here is unreachable, as for
    the other arms. `Wrapper::Newtype` keeps today's `begin_nth_field(0)`.

  - [ ] **5.7** (Claude) J's `form!` side: `choices_for` with its two impls, the
    settings route the same way, the `const _` that the fallback needs a `String`
    proxy, and a golden for a struct with neither.

  - [ ] **5.8** (Claude) Tests the box needs beyond box 2's: `widget: textarea` on
    a `Ref<T>` path compiles (5.1 on a struct), `widget: number` on a `TimeLimit`
    is refused (a golden), a proxied `Ref` inside an `Option` round-trips, and a
    `field:` whose field reads `TimeLimit` itself gets no wrapper (K's first
    case). Then put `mod custom_fields;` back for good once box 6 lands.

- [ ] **6.** (Todd) `Formoxus::with_widget_for` and its place in the widget
      order (H), and `Formoxus: Send + Sync` for `with_context`. Expanded
      2026-11-24 by Claude. Decide L first.

  **Where it stands.** `Formoxus` is `Send + Sync` and `AnyWidget` is `Clone +
  PartialEq + Send + Sync` (checked 2026-11-24), so nothing blocks `with_context`.
  Today a leaf's widget is decided ONCE, in `LeafArgs::node` at build time
  (`spec.widget`, else `field.default_widget(optional)`), and stored in
  `FieldNode::widget`. The build runs on the server too (`Submission::accept`),
  where there is no Dioxus context, so the app's per-type widget cannot be
  chosen there. It has to be chosen at RENDER,
  from what `Form` resolved, which is the config cascade's rule anyway: resolve
  at the `Form` boundary, never call `defaults()` while rendering
  (`.claude/memory/config_cascade.md`).

  **New decision (2026-11-24, Claude recommends; Todd decides).**

  L. **What the build-time widget still decides.** Three things are read off
     `FieldNode::widget` outside rendering: `is_checkbox` (an unticked checkbox
     writes `false`, on the server too), `is_password` (a password is not
     trimmed), and `value_when_empty` (a range starts at its midpoint). None of
     them can see the app's per-type widget on the server. Recommended: the
     per-type widget changes RENDERING only, and `with_widget_for` refuses the
     four built-ins whose name changes what a value means (`CheckboxInput`,
     `PasswordInput`, `RangeInput`, `FileInput`) with a panic at launch. A dev's
     own widget, the real case (`MarkdownWidget`), is never one of them.
     Alternative: allow them, and document that the server reads the field's
     default.

  - [ ] **6.1** (Todd) `Formoxus` holds `widgets_for: Vec<(&'static Shape,
    AnyWidget)>`, and gains `with_widget_for::<T: Facet<'static>>(self, widget:
    impl LeafWidget) -> Self`, which replaces an existing entry for `T`.

    1. Keyed by `Shape`, whose `PartialEq` compares type ids, so `Formoxus`'s
       derived `PartialEq` still holds. A `Vec` is plenty: an app registers a
       handful of types.
    2. Check the widget against `T` there: `field_kind::renders::<W>(T::SHAPE,
       false)` is a `const fn`, so an inline `const { assert!(…) }` in the
       generic method fires when the app is BUILT (not under `cargo check`; see
       `.claude/memory/const_shape_walk_blocked.md`). L's refusal goes beside it.
    3. Its doc says the order (6.4) and that the per-type widget applies wherever
       a `T` is met by shape: a field, a list item, a variant's field, an
       `Option<T>`'s inside.

  - [ ] **6.2** (Todd) Resolve it at the `Form` boundary: a
    `Form::widgets_for()` reading `defaults()`, like `classes()` does, into a
    `FormWide` field. There is no per-form tier, so nothing to fold.

  - [ ] **6.3** (Todd) A leaf remembers what render needs to choose: whether its
    widget came from the spec (`widget:` or `with_widget`), and the TYPE to look
    up. The type is the path's own: `Wrapper::Newtype(s)` gives `s` (`Note`, not
    the `String` the field holds), `Wrapper::Proxy { shape, .. }` gives `shape`
    (`TimeLimit`), and no wrapper gives `F::Value::SHAPE` (`Note` for a
    `field: NoteField`). Box 5's `Wrapper` is what makes the first two possible,
    which is why 6 follows 5.

  - [ ] **6.4** (Todd) Choose at render, in `FieldNode::render`: the spec's widget
    if it came from the spec, else the `FormWide` entry for the leaf's type, else
    the build-time widget (the field's `default_widget`). That is decision H's
    order, and `the_apps_widget_for_a_type_beats_a_named_fields_default_widget`
    pins its middle step. Use the chosen widget for `extends()` and `render`;
    leave `is_checkbox`, `is_password` and `value_when_empty` on the build-time
    one (L).

    Trap: a set's draft input renders through the same leaf code, so a
    per-type widget for a set's item type renders in the draft too. That is
    probably right (the draft is where a `Note` is typed); a test would say.

  - [ ] **6.5** (Claude) Tests beyond box 2's: L's refusal, a per-type widget
    inside a variant, and a set draft over a registered type. Then `mod
    custom_fields;` goes back into `suite.rs` for good.

- [ ] **7.** (Claude) Everything compiles and passes: suite, examples, e2e. A
      gallery example and an e2e pair (a time limit typed as `1h15m` that
      survives a submit). Docs, memory, and a comment on #28.

- [ ] **8.** (Todd) apcsp: `Facet` (and `#[repr(u8)]` on the enums) for
      `AssignmentData`, `AssignmentSettings`, `AssignmentLatePolicy` and
      `QuestionGroup`; a `TimeLimit` newtype proxied to `String`;
      `MarkdownWidget` registered once at launch and the five
      `widget: MarkdownWidget` lines dropped; the hand-written converters
      deleted. `QuestionGroup`'s deferred `Random` and `Streak` variants would
      then show in the form: accept that, or keep a narrower form type.
      **The question ids wait on the storage repo:** `groups[].questions[]`
      becomes the storage-agnostic `Ref<Question>` (decision I) only once that
      repo's core crate exists. Until then apcsp keeps its ids as `String`s in
      that one place, which `choices:` already checks (`assignment_shape.rs`),
      and converts them where the model is saved. Do NOT add facet to today's
      surreal-table `Ref<T>` in the meantime: `table!` would only undo it.

- [ ] **9.** (Todd) Merge, and close #28.
