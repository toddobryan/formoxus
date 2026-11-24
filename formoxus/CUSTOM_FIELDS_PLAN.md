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

- [ ] **3.** (Todd) `Field::format`, and the named field winning at the top of
      `node_for_shape`.

- [ ] **4.** (Claude) `form!`'s side: a `field:` key, the `const _` checks
      against `<F as Field>::KIND`, and whatever F needs.

- [ ] **5.** (Todd) The proxied leaf: classification (trap 2), the node, the
      check through `convert_in` (D), the write-back, and the refill through
      `convert_out`.

- [ ] **6.** (Todd) `Formoxus::with_widget_for` and its place in the widget
      order (H), and `Formoxus: Send + Sync` for `with_context`.

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
