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

AB. **DECIDED 2026-11-26 (Todd): one list of the app's types, known at
   compile time, carried by each form, with the app's list installed as
   overrides.** Todd asked whether the `TypeRegistry` could exist at compile
   time, then whether it could work for libraries that have types and forms
   of their own, then how an app overrides a library's choices. Claude
   probed each in a throwaway workspace (a plain models crate, a stand-in
   for formoxus, two libraries, an app, and a working-tree copy of formoxus
   changed as in 5 below); the details are in
   `.claude/memory/compile_time_type_registry_probe.md`. Everything marked
   "probed" compiled and ran there; everything under 11 did not get that far.

   1. **The trait (probed).** formoxus gets
      `trait FieldFor<V> { type Settings: Default; type Field: Field; fn build(Settings) -> Field; }`,
      and an app implements it for a unit struct of its own:
      `impl FieldFor<models::Markdown> for AppTypes`. That is legal even
      though the app owns neither the trait nor the model: the orphan rule
      needs one local type in the impl, and `AppTypes` is one. So models stay
      plain and depend on nothing but `facet`. `Settings` is what `form!`'s
      settings configure: the field itself, or for a newtype its inner field,
      which `build` wraps in a `Newtype`. That is how `bio => { max_length: 280 }`
      on a registered `Markdown` is typed at compile time with no extra key.
   2. **`types!` declares a list once (probed).** In formoxus-macros, a
      function-like proc macro:

      ```rust
      formoxus::types! {
          pub struct AppTypes;
          include libt::LibTypes;
          Markdown => newtype String,
          TimeLimit => field TimeLimitField,
          replace libt::Badge => field LoudBadgeField,
      }
      ```

      (`include` and `replace` as amended by AF and AI.) It writes the
      struct; one `FieldFor` impl per entry, per built-in, and one for
      `Option<T>` that reuses `T`'s; a
      `registry()` that builds the list's run-time `TypeRegistry` once (a
      `LazyLock` static); and, for a library, the replay macro of 4. The
      built-ins must be listed by the macro itself: a blanket
      `impl<V: DefaultField> FieldFor<V> for AppTypes` beside the dev's
      entries is refused (E0119, "upstream crates may add a new impl of
      `DefaultField` for `Markdown`"). Widget entries (box 6) are not
      designed yet; see 11.
   3. **`form!` finds its list by name (probed).** It writes a bare
      `FormoxusTypes`, which rustc resolves at the call site like code typed
      there, so a `use` points every form in a module at a list. The app
      writes `use AppTypes as FormoxusTypes;`. A library writes
      `type FormoxusTypes = crate::LibTypes;`, a type alias rather than a `use`:
      a `use` would also import the macro of the same name (4), and rustc
      denies a crate reaching its own macro-generated `#[macro_export]` by an
      absolute path (`macro_expanded_macro_exports_accessed_by_absolute_paths`).
      A proc macro cannot learn the module it expands in, but it does not
      need to.
   4. **Libraries are included by replay (probed with two).** A library's
      `types!` also writes an exported `macro_rules!` with the same name as
      its struct (macros and types are separate namespaces, so one
      `use libt::LibTypes;` imports both). It replays the library's entries,
      `$crate`-qualified, back into `types!`. The app writes
      `include LibTypes;` once per library and gets a real impl for every
      type in every list. Four things found doing it:
      1. The replay macro must be written by the proc macro. Written by
         `macro_rules!`, its `$crate` names the crate the tokens came from
         (formoxus), not the library, even when the library passes in its
         own `$`.
      2. A library's entries must be reachable from its root
         (`pub use models::Markdown;`), because the replay writes
         `$crate::Markdown`. `types!` can say so when it is not.
      3. Including by delegation
         (`impl<T> FieldFor<T> for AppTypes where LibTypes: FieldFor<T>`)
         fails as soon as the app lists a type from a third crate (E0119),
         which is the common case.
      4. Listing a type an included list already has is E0119 on that type.
         `replace` is the way to mean it.
   5. **Each form carries its own list (probed).** `form!` puts
      `FormoxusTypes::registry()` in its `FormSpec` (a `with_types` builder), the
      walk carries it (`Walk.types`), and the walk's registry arm asks it.
      `Items` and `VariantNode` keep the list from the walk that built them,
      because they walk again later: when the server fills a form from the
      wire, when an item is added, when a variant is chosen. Without their
      own copy those later walks would quietly miss the form's types. In the
      probe, with nothing installed: two lists in one process gave the same
      type different fields and each form got its own; a server-style
      rebuild (`empty_form`, then `distribute_values`) built a list item
      through the kept list and validated; a library's form worked over a
      type the app never listed; and, as a control, the same form with no
      list panicked naming the type. So a library's forms bring their own
      types, and client and server agree because both run the same form.
   6. **The app's list is installed as overrides.** `main` calls
      `AppTypes::install()`, as it calls `TypeRegistry::install` today, and
      for the same reason (5.1): client and server both run `main`. A leaf's
      field and widget are then looked up in this order:
      1. the path's own settings, `field:` or `widget:` in its form;
      2. the installed list;
      3. the form's own list;
      4. the built-ins.

      For the app's own forms the installed list IS the list they compiled
      against, so 2 and 3 agree. For a library's forms, 2 is how the app's
      `replace`s reach them. Forgetting `install` now costs only the
      overrides: every form still builds from its own list.
   7. **An override never reaches a path the form configured.** A setting,
      `field:` or `widget:` that a library wrote in its form was checked
      against the library's list when the library compiled, and the app's
      field for that type may not even have that setting. So a configured
      path keeps what its form says (decision H: a path beats a type), and
      an app that wants it changed overrides that form path by path.
   8. **The app's `replace`s are checked when the app compiles.** Through the
      replay, `types!` sees the library's entry for each type it replaces.
      A replacement field whose `KIND` differs from the library's is
      refused: a library form that renders `Headline` as text and suddenly
      gets an `Int` field is almost certainly a mistake, and a dev who means
      it overrides that library form path by path. A replacement widget must
      accept the `KIND` of the replacement field and of the library's field.
      (Decided 2026-11-26, Todd agreeing with Claude.)
   9. **What is checked when.**

      | What | When |
      | --- | --- |
      | The app's own forms, against `AppTypes` | compile time, all of it |
      | A library's forms, against its own list | when the library compiles |
      | The app's `replace`s, against the library's entries | when the app compiles |
      | An installed widget meeting a library form's own `field:` (a field type the library's list does not hold) | when that form is built, as V's checks are now |
      | A hand-built `FormSpec` with no list | when the form is built, against the built-ins and the installed list |

   10. **What this replaces or changes, once built.**
       1. `TypeRegistry::install` and the process-wide registry stay, but
          hold the app's overrides rather than the only copy of its types.
          `TypeRegistry::new().field_for(…)` in `main` becomes `types!`, and
          `TypeRegistry::newtype::<T, V>()` (Z) goes with `DefaultField`: a
          `T => newtype V` entry does its job.
       2. Y's `newtype` key is no longer needed: a registered type's
          settings go through its `Settings`. An unregistered newtype with
          settings is refused, as one with none is (5.5.4).
       3. Most of V's run-time checks become compile-time ones; what is left
          is the second-to-last row of 9.
       4. `DefaultField` goes (decided 2026-11-26, after Todd asked why it
          was still needed). Its two remaining jobs both belong to the list:
          `form!` asks `<FormoxusTypes as FieldFor<P>>` instead, and a
          `newtype V` entry takes its inner field from the same list's own
          entry for `V`, `<AppTypes as FieldFor<V>>`, so a list that
          replaces `String` gets its newtypes over `String` through the
          replacement too (probed: a list whose `String` field brackets its
          text showed `Markdown("hi")` as `[hi]`).
       5. The built-ins sit in every list (2), so the walk's scalar arm and
          `scalar_node`'s dispatch go. The built-in types are named once, in
          `types!` itself, instead of three times (`scalar_node`, the
          `DefaultField` impls, `Number`'s impls, of which only `Number`'s
          stay, because `NumberField<T>` is generic over them).
       6. 5.5.5's set-item key formats through the list: `path!(…[= v])`
          takes `FormoxusTypes` from scope, as `form!` does.
       7. Box 6's per-type widget becomes an entry in `types!`.
       8. AA still stands: `path!` sees the list, but not a form's
          per-path `field:` on a set's items.
   11. **Not probed, to settle while building.**
       1. A library whose list includes another library's, with the app
          including both, replays the shared entries twice (E0119). Likely
          fix: each replay is tagged with its crate and list name (the proc
          macro reads `CARGO_CRATE_NAME`), and `types!` skips a tag it has
          seen. Matching on the type's text would be wrong, since
          `$crate::Markdown` reads the same from two crates.
       2. `path!(…[= v])` with `FormoxusTypes` from scope.
       3. Widget entries, and how `replace` names a widget rather than a
          field.
       4. What `form!` says when no `FormoxusTypes` is in scope. rustc's own
          "cannot find type `FormoxusTypes`" is a poor message; `form!` may want
          a `types:` key for a form that names its list directly.
       5. A hand-built `FormSpec`: the built-ins and the installed list, or
          nothing.
   12. **When: now (Todd, 2026-11-26), in place of 5.5.5 as written.** The
       build is box 5.9.

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
     **AMENDED 2026-11-26 by W:** `#[facet(transparent)]` is not special
     either.

  W. **No newtype is special, `#[facet(transparent)]` included.** DECIDED
     2026-11-26 (Todd, amending N): explicit is better than implicit, so it
     is less surprising. A transparent `Markdown(String)` is registered like
     any newtype, through a small adapter formoxus ships,
     `formoxus::fields::Newtype<T, F>`. It implements `Field<Value = T>` by
     passing everything to an inner field `F` and wrapping and unwrapping
     through facet reflection, so the model needs no `From` impls and stays
     plain. (`Newtype`, one word, as Rust spells the idiom.)

     What it costs a dev: one line per type,
     `.newtype::<Markdown, String>()` on the registry (Z), instead of one
     attribute; and the type's widget and attribute checks
     move from compile time to the build (V). What it buys:

     1. One rule: a type the compiler cannot classify needs a field.
     2. No `begin_inner` write-back, and no outer shape threaded through
        `LeafArgs` for 6.2: a leaf's shape is always its path's own type.
     3. Settings on a newtype path, which transparent never gave
        (`form_setting_on_a_newtype`): `bio => { newtype,
        max_length: 280 }` (Y), with every compile-time check back, judged
        against the inner type, which rustc infers through `.0`.
     4. No `try_from` trap. facet lets a transparent type validate itself
        through `try_from`, and formoxus would build one through
        `begin_inner`, so a value the type refuses would fail inside
        `write_value_into`, after `validate()` passed, with no field error
        for the user. An explicit field refuses it in `parse`.

     **Revisit if it proves obnoxious in use (Todd):** handle a transparent
     wrapper over a `String`, `bool` or number automatically, AFTER the
     registry check, so a registered field still wins.

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
        **SUPERSEDED 2026-11-26 by W:** `kind` no longer follows
        `transparent`, so `Markdown` is `Unknown` too and is registered
        through `fields::Newtype`.
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

    1. `TypeRegistry` imported from `formoxus::type_registry`, not the
       prelude (no new re-exports until publish; Todd's edit of the tests,
       2026-11-26).
    2. `field_for(field)` with no turbofish: the type is the field's `Value`.
       `widget_for::<T>(widget)` names the type, since a widget does not.
    3. A type whose registered widget cannot render its registered field's
       `KIND` panics in whichever of the two builder calls comes SECOND,
       naming the type and the widget's form name (`int_only`, a test widget
       that accepts only `Int`; `number` was wrong here, since every
       text-like input accepts text). That is still
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

    **Superseded 2026-11-26 by W:** both traps, and "Other changes" 1 and 2,
    assumed `transparent` is followed. It is not, so 5.5.1 turns those test
    newtypes back into plain ones registered through `fields::Newtype`.

  - [x] **5.3** (Todd) The registry (M), in `formoxus/src/type_registry.rs`.

    **Done 2026-11-26** (Todd; docs by Claude). As listed below, with these
    differences. A field is stored as `TypeAndCtor { field_type, node_ctor }`.
    `NodeCtor::for_field(field)` is the one place a field becomes a
    constructor, shared with `field_kind::named_field`; it takes the field by
    value, which is what cleared clippy's `needless_pass_by_value` on both
    callers. B DECIDED (Todd): a second field or a second widget for one type
    panics. Claude's tests follow: three duplicate tests, and the mismatch
    tests use a test widget `IntOnly`, since no built-in refuses text except
    by needing `min:`/`max:`. Suite 1000 passed, 22 failing as expected: 21
    wait on 5.4 to 6.2 and one on 5.5. `just clippy` is red only on
    `registry()` and `node_ctor`, which 5.4 reads.
    Expanded 2026-11-26 by Claude from Todd's questions that day; this
    replaces a one-sentence version that said `Vec` and "a second
    registration replaces the first", both now superseded. What 5.2's tests
    need from it, in the order they are worth building:

    1. **Storage.** `TypeRegistry` holds a private
       `HashMap<&'static Shape, Registered>`. A map rather than a `Vec`
       because the walk looks a type up at every leaf it builds. It works
       as a key because facet's `Shape` implements `Eq` and `Hash` by its
       type id, not its address (`facet-core` `shape.rs:284-299`), and
       `&'static Shape` hashes through to that. `Registered` holds the
       type's field and its widget SEPARATELY: an
       `Option<(FieldType, NodeCtor)>` and an `Option<AnyWidget>`. They
       cannot be merged into one `NodeCtor`, for three reasons the tests
       pin. `Topic` registers both. `Note` registers only a widget, which
       goes on the inner `String`'s default field. And a named `field:`
       beats the registered field while the registered widget still beats
       that field's default (`NoteField` on `Memo`). A built `NodeCtor` is
       a closure, so the widget could never be taken back out of it. The
       field is erased as a `NodeCtor` because `Field` is not
       dyn-compatible: it has an associated type, an associated const and
       an `impl Future` return. `KIND` is copied out beside it because the
       closure hides it and the mismatch check in 3 and 4 needs it. Both
       `TypeRegistry` and `Registered` derive `Debug` and `Default` and
       nothing else. `Send` and `Sync` are auto traits, which cannot be
       derived and are not needed: `NodeCtor` and `AnyWidget` are both
       already `Send + Sync`. Leave out `Clone`, because nothing copies a
       registry once it is installed. Leave out `PartialEq`, because
       `NodeCtor`'s equality is `Arc` identity, so two registries built
       the same way would compare unequal. `Registered` and its fields can
       be `pub(crate)`, since nothing outside the crate reads them.

    2. **`new()`.** Returns `Self::default()`. Both stay, because clippy's
       `new_without_default` fires on a `new()` with no `Default`, and the
       tests call both.

    3. **`field_for<F: Field + Send + Sync>(self, field: F) -> Self`.** No
       turbofish: the key is `F::Value::SHAPE`, so a field for the wrong
       type cannot be registered at all. It needs `Send + Sync` because the
       field is captured in a `NodeCtor`. Build the constructor exactly as
       `field_kind::named_field` does,
       `NodeCtor::new(move |args| Box::new(args.node(field.clone())))`, so
       a registered field and a `field:` field go through the same
       `LeafArgs::node`; consider factoring that out of `named_field` so it
       exists once. If the type already has a widget, check
       `widget.accepts().contains(F::KIND)` and panic if not, naming the
       type (`Shape`'s `Display` gives `TimeLimit`) and `widget.name()`
       (the form name, `number`).
       `registering_the_field_after_a_widget_it_cannot_use_panics_too` pins
       this order.

    4. **`widget_for::<T: Facet<'static>>(self, widget: impl LeafWidget) ->
       Self`.** Moved here from 6.1, because `test_types()` calls it, so
       the suite cannot compile without it. The key is `T::SHAPE`, and the
       widget is stored as `AnyWidget::new(widget)`. If the type already
       has a field, make the same check as 3 the other way round, with the
       same message. `registering_a_widget_that_cannot_render_the_types_field_panics`
       pins this order. The check runs in the builder call, not in
       `install`, so that both tests can make it: `install` runs once per
       process. It is still startup for an app, because the registry is
       built in `main`.

    5. **Registering a type twice.** DECIDED 2026-11-26 (Todd): panic, as
       recommended, for a field and for a widget.
       Recommended: panic, naming the type, through the map's entry API
       rather than an `insert` that silently drops the first registration.
       Two registrations of one type are a mistake, and without a panic
       nothing would show which one won (the hard-to-misuse rule). A type
       with one field and one widget is not a duplicate. There is no test
       yet; Claude adds one once this is decided.

    6. **`install(self)`.** A private
       `static REGISTRY: OnceLock<TypeRegistry> = OnceLock::new();` in this
       module, which is a plain `static` because `OnceLock::new` is a
       `const fn`. The map is built at runtime, in `main`, before `install`
       is called. `install` calls `REGISTRY.set(self)` and panics if it
       returns `Err`, with a message containing "installed"
       (`installing_a_second_registry_panics`). A `static` needs a `Sync`
       type, and `OnceLock<T>` is `Sync` only when `T` is `Send + Sync`, so
       a non-`Send` thing in `Registered` would fail to compile on this
       line.

    7. **`registry() -> Option<&'static TypeRegistry>`, `pub(crate)`.** It
       is `REGISTRY.get()`, with `None` meaning an empty registry. It must
       NOT be `get_or_init(TypeRegistry::default)`. The first form built
       before `install` would lock in the empty registry, and the real
       `install` in `main` would then panic with "already installed". The
       suite would hit this, because tests that never call
       `install_test_registry()` build forms in a random order alongside
       the ones that do. A form over built-in types has to work with no
       registry at all, so `None` is not an error either. Put the lookups
       on the registry, for example a field and a widget by `&Shape`, so
       5.4 and 6.2 never touch the map or the `Option` directly.

    8. **Exports.** None new. The tests import
       `formoxus::type_registry::TypeRegistry` from its own module, under
       the rule against new re-exports until publish
       (`.claude/memory/prelude_is_the_one_list.md`). Whether it joins the
       prelude is a question for the publish-time API pass.

    9. **Doc.** The order (P), and that a registered type is found by shape
       wherever it sits: a field, a list item, a variant's field, an
       `Option<T>`'s inside.

  - [x] **5.4** (Todd) The registry arm in `node_for_shape`, right after the
    spec's constructor arm and before the file, scalar and newtype arms:
    look `shape` up in `registry()`, and on a hit `return
    ctor.build(leaf(peek, None, shape))`. After the `Option` arm for the same
    reason as the constructor (trap 6). After the constructor because a path
    beats a type (H). Before the scalar arm, so an app can register a field
    for a built-in type, a `String` that always renders as a `MarkdownWidget`.
    3.4's assert stays as a backstop; `field_for`'s bound makes it
    unreachable.

    **Done 2026-11-26** (Todd; comment by Claude). Not a separate arm: the
    spec's constructor arm became
    `spec.and_then(|s| s.node.as_ref()).or_else(|| TypeRegistry::field_ctor(shape))`,
    so H reads as the `or_else`, and the two share one body and one comment.
    `TypeRegistry::field_ctor(shape) -> Option<&'static NodeCtor>` is the
    walk's only way in, and borrows rather than clones. Suite 1013 passed, 9
    failing as expected: 7 on 6.2 (the registered widget), 1 on 5.6 (the
    build-time widget check), 1 on 5.5 (the plain-newtype panic). Clippy and
    fmt clean.

  - [ ] **5.5** (Todd, after Claude's tests) No newtype is special (N as
    amended by W). Rewritten 2026-11-26 by Claude; it replaces a version from
    earlier the same day that narrowed the newtype arm to transparent types
    (in `15f19fb`). The test that waits on it today is
    `newtypes::a_plain_newtype_without_a_registered_field_panics_naming_the_type`.

    1. **(Claude) Tests first.** The suite will not compile until 2 exists,
       by design.
       1. **One registry for the whole suite.** `test_types()` moves from
          `custom_fields.rs` to `suite.rs`, and each module that has
          newtypes contributes a `register(TypeRegistry) -> TypeRegistry`.
          Every test that builds a form over one calls
          `install_test_registry()`.
       2. **`newtypes.rs` is rewritten around `Newtype`.** The same
          behaviours, now registered: one leaf at the field's own path,
          round trips, refill shows the inner text, a bad inner value is
          refused with the inner field's message, empty is required,
          `Option` in both states. Plus: a `#[facet(transparent)]` type with
          nothing registered panics like a plain one, and its message
          says that formoxus ignores `transparent`; `Pair(Config)` still
          walks as a struct.
       3. **`Newtype`'s own tests.** It takes `KIND`, `trims`, `check`,
          `check_on_server` and `default_widget` from the inner field; a
          setting reaches the leaf through a configured inner field given to
          `field_for`; `new` panics for an inner field whose `Value` is not
          the type's field's type, naming both types (the one-field check
          is a build-time `const` assert, so it has no runtime test);
          `TypeRegistry::newtype` (Z) registers the default inner field.
          Tests of Y's `newtype` key belong to 5.8, with the macro, so this
          step's tests compile once 5.5.2 exists.
       4. **The other modules' newtypes** (`Tag` in `sets` and
          `set_item_errors`, `Bio` in `trim` and `field_validators`, `Blurb`
          in `form_macro`) go back to plain and are registered through
          `Newtype`, or become a `String` where the newtype is not what the
          test is about. `set_item_errors`' `Tag` stays a newtype: its
          `path!(Shelf.labels[= &old])` is exactly 5's case.
       5. **`custom_fields.rs` section 3.** `Note` and `Secret` become plain,
          registered with `Newtype` plus their widgets. `Remark`, the
          unregistered control, gets a `Newtype` too, since unregistered it
          would now panic.

       **Done 2026-11-26 (Claude), uncommitted.** As listed, with these
       choices for 5.5.2 to build or change: `Newtype::new(field)`;
       `TypeRegistry::newtype::<T, V>()`; `new`'s mismatch panic names the
       newtype and the inner field's `Value` (`Note`, `u32`); 4's panic
       names the type, `TypeRegistry::field_for`, `field:` and `Newtype`,
       plus "transparent" for a transparent type. `newtypes.rs` has 28
       tests, nine of them one per delegated method through a test field
       `Loud` that overrides every default, so a forgotten delegation fails
       by name. `custom_fields.rs` gains
       `a_path_into_a_registered_types_fields_is_refused` (6). The suite
       fails to compile only on the missing `fields::Newtype` and
       `TypeRegistry::newtype` (10 errors); with `todo!()` stubs in a
       scratch copy it compiles with no warnings.

    2. **(Todd) `formoxus::fields::Newtype<T, F>`.**
       1. **Shape of the type.** It holds the inner field and a
          `PhantomData<fn() -> T>`, which keeps it `Send + Sync` and
          `'static` whatever `T` is. `Clone`, `Debug` and `PartialEq` can be
          derived, because `T` is a `Value` and has them anyway. `Default`
          must be written by hand, bounded on `F: Default` only: a derive
          would also require `T: Default`, which `Markdown` need not have.
       2. **`new(field: F) -> Self` checks the pairing.** That `T` is a
          struct with exactly one field can be checked at build time with an
          inline `const { assert!(…) }` on `T::SHAPE`, which fires when the
          app is built (not under `cargo check`, see
          `.claude/memory/const_shape_walk_blocked.md`). That the field's
          type is `F::Value` has to be checked at run time, since a field's
          shape sits behind a `ShapeRef` const code cannot call: panic
          naming both types. One-field named structs are allowed as well as
          tuple structs. The old argument for tuple-only was about formoxus
          flattening a struct on its own; here the dev asked for it.
       3. **The `Field` impl delegates everything.** `Value = T`,
          `Context = F::Context`, `KIND = F::KIND`. `parse` parses with `F`
          and wraps. `format`, `check` and `check_on_server` unwrap and call
          `F`. `trims`, `widget_attrs`, `default_widget` and `choices` pass
          straight through. **Trap:** every `Field` method with a default
          body has to be listed, because one that is left out silently uses
          the default instead of the inner field's (`trims` would quietly
          stop trimming). Say so in a comment, so a method added to `Field`
          later gets added here too.
       4. **Wrapping and unwrapping.** Wrap by building `T` with facet's
          `Partial`: `alloc::<T>()`, `begin_nth_field(0)`, `set`, `end`,
          `build`, `materialize::<T>()` (all in facet-reflect 0.46.5).
          Unwrap with `Peek::new(t).into_struct()`, `field(0)`, then a typed
          borrow of `F::Value`. After `new`'s check, failure in either is a
          bug, so `expect` with both types in the message.
       5. **`TypeRegistry::newtype::<T, V: DefaultField>(self) -> Self`**
          (Z), in `type_registry.rs`: `self.field_for(Newtype::<T,
          V::Field>::new(V::Field::default()))`. It inherits `field_for`'s
          duplicate and mismatch panics.
       6. **Docs**, for the module and the type: the registered route
          (`newtype::<T, V>()`, or `field_for` with a configured or custom
          inner field), the per-path route (`newtype` in `form!`, Y), and
          why formoxus ignores `transparent` (W).

    3. **(Todd) Delete the special-casing.** The newtype arm in
       `node_for_shape`; `newtype_inner` and its long doc; `FieldNode::wrapper`
       and its doc; `LeafArgs.wrapper`; the `begin_nth_field(0)` branch of
       `write_value_into`, leaving a plain `set`; `scalar_node`'s wrapper
       argument through `leaf(…)`; the "inner value for a newtype" comment
       on `peek_value`; the test helper's `wrapper: None` at the bottom of
       `field_node.rs`. Nothing replaces them: no `begin_inner`, no outer
       shape. 6.2 keys on `LeafArgs.shape`, which is now always the path's
       own type.

    4. **(Todd) Refuse a newtype that reaches the struct arm.** With the arm
       gone, a `Headline(String)` that has no `field:` and no registered
       field would land on the struct arm and quietly become a fieldset with
       a leaf at `headline.0`: exactly the guess N retires. So, before the
       struct arm, panic for a one-field struct whose field is a scalar.
       Keep that scalar condition: `Pair(Config)` still walks as a struct
       (`a_newtype_around_a_non_scalar_is_still_a_struct`). The message names
       the type, `TypeRegistry::field_for`, `field:` and `fields::Newtype`,
       and when `shape.inner` is `Some`, adds that formoxus ignores
       `#[facet(transparent)]`.

       **Done 2026-11-26 (Claude, at Todd's request).**
       `refuse_unregistered_newtype` in `build.rs`, called from the struct
       arm. A one-field TUPLE struct only, not "a one-field struct" as
       above: `a_one_field_named_struct_is_still_a_struct` keeps
       `Config { name: String }` walking as a struct. The message names the
       path and both types, and gives the exact line to write,
       `TypeRegistry::new().newtype::<Byline, String>()`. `newtypes` 28/28.

    5. **SUPERSEDED 2026-11-26 by 5.9 (AB built now, AB.12 option 1).**
       5.9.3.4 and 5.9.3.10 do it through the form's list. Kept for the
       reasoning in 1 and 9. **(Todd) The set-item key, no longer parkable.**
       Expanded 2026-11-26 by Claude.

       1. **What breaks, and why.** A validator names one set item by its
          value: `path!(Shelf.labels[= &old])`. `path!` turns the value into
          text with `item_value_key`, through the private `entry_text` in
          `path.rs`, and `ModelPath::resolve` later finds the item whose own
          entry is that text. An item's entry is what its node writes in
          `collect_values`, which for a leaf is its field's `format`. So the
          two sides must write the same text for the same value. The node
          side already does it right: a registered `Tag` is a
          `FieldNode<Newtype<Tag, StringField>>`, whose `format` writes
          `rust`. The key side has only the value, with no node and no form
          (a validator can run on the server, long after the form is gone),
          so `entry_text` falls through to facet's `Display`, which for a
          plain `Tag` writes `⟨Tag⟩`. The key never matches, and the error
          lands where every unresolved path does, not on the item.
       2. **There is no newtype branch to remove.** `entry_text` had one
          (`newtype_inner`, then `field(0)` and the inner value's `Display`),
          and 5.5.3 removed it with `newtype_inner`. This step adds a
          branch, not takes one away.
       3. **The registry learns to format a value.** The only thing that
          knows how a registered type is written is its field, so the
          registry keeps, next to each field's constructor, a closure that
          formats a value of the field's type:
          `Arc<dyn for<'m, 'f> Fn(Peek<'m, 'f>) -> String + Send + Sync>`.
          `field_for` builds it from a clone of the field: the closure
          borrows `&F::Value` out of the peek with
          `peek.get::<F::Value>()` and calls the field's `format`. The
          `get` cannot fail, because the entry is keyed by
          `F::Value::SHAPE`, so `expect` with a message saying so. (Proven
          in Claude's scratch probe: the higher-ranked closure type compiles
          and formats a `Note` through a `Peek`.) `Arc` rather than `Box` to
          match `NodeCtor`; nothing clones it, so either works.
       4. **`REGISTRY` keeps its type.** It stays
          `OnceLock<TypeRegistry>`. What grows is `TypeAndCtor`, which gains
          a third member for the closure. Two consequences:
          1. `TypeAndCtor` derives `Debug`, and an `Arc<dyn Fn>` has no
             `Debug`. Either wrap the closure in a small type with a
             hand-written `Debug`, the way `NodeCtor` does
             (`field_node.rs`), or write `TypeAndCtor`'s `Debug` by hand.
          2. With three members the name `TypeAndCtor` no longer fits;
             `RegisteredField { kind, ctor, format }` would. Optional.

          `Send + Sync` costs nothing new: the closure captures a clone of
          `F`, and `field_for` already requires `F: Send + Sync`.
       5. **A lookup beside `field_ctor`.** `pub(crate) fn
          format_value(value: Peek<'_, '_>) -> Option<String>` on
          `TypeRegistry`: `registry()?`, the entry for `value.shape()`, its
          field, then the closure. `None` with nothing installed or nothing
          registered for the type.
       6. **`entry_text` asks the registry first.** Before the enum branch
          and before `Display`. First, not last, for the reason the walk's
          constructor arm comes before the scalar arm: an app that registers
          a field for a built-in type (a `String`) replaces the built-in
          everywhere, so its items must key through that field too, or the
          two sides disagree again. Update `item_value_key`'s doc, which
          still says "a newtype's inner scalar": the text is now "the
          registered field's `format` for a registered type, a unit enum's
          variant name, any other leaf's display".
       7. **Both sides need the registry, and both have it.** `path!` runs
          wherever the validator does, on the server included, which is
          fine because `install` runs in `main` on both sides (5.1).
       8. **Tests.** `set_item_errors::a_newtype_item_is_found_by_its_inner_value`
          goes green. The inline `path::tests::a_values_text_is_its_item_entry`
          cannot: its `Tag` is unregistered, and the library's tests share
          one process, so one test calling `install` would collide with any
          other that did. Claude takes the `Tag` case out of it, since
          `set_item_errors` covers it through the suite's registry.
       9. **A gap this leaves, raised rather than fixed.** A per-path
          `field:` on a set's items (`labels[] => { field: TagField }`)
          formats the node with `TagField`, but `path!` has no form and
          cannot see the spec, so the key still comes from the registry or
          `Display`. If the two write different text, a `[= v]` error never
          reaches its item. It predates this step (it came in with box 4's
          `field:`); a registered field avoids it. Recorded as decision AA
          for Todd.

    6. **(Todd) The runtime path check** (`build.rs:185`, "`{walked}` is a
       `{shape}`, one input, which has no field"). Its test becomes "a type
       with a registered field, or a file upload". A plain newtype is no
       longer refused there, so `path!(Story.headline.0)` resolves and the
       build then panics with 4's message, which is the right one. A
       registered `TimeLimit` or `Ref<Question>` IS one input, so a path
       into its fields is refused. This relies on `install` running before
       specs are built, which its doc requires. Claude adds a test for the
       registered case in 1.

    7. **(Todd) Reword the last-resort panic** (`other => panic!("field
       {name} has unsupported type {other:?}")`) the same way as 4: name
       the type with `{shape}` and point at `TypeRegistry::field_for` and
       `field:`.

    Afterwards the only failures should be 6.2's 7, 5.6's 1, and any of 1's
    tests that wait on X. Run `cargo test -p formoxus --lib` too: `path.rs`'s
    item-key tests touch `entry_text`.

    **Letter for Todd (2026-11-26, Claude recommends; DECIDED the same
    day):**

    X. **`field:` with a `Newtype`.** Box 4 lets `field:` take only a struct
       expression or a unit struct, because `form!` reads the field's type
       off the expression for its `const _` checks, and refuses a call
       (`form_field_not_a_struct_expression`). `Newtype::<Blurb, _>::new(…)`
       is a call, so W's third point needs one of:
       1. **Recommended:** `form!` also takes a call whose callee is a type
          path, `Newtype::<Blurb, StringField>::new(…)`. It reads the type
          off the callee, so the generics have to be written in full, and
          pins it by checking that the call's value has that type. This is
          a generalisation of box 4's rule, and it would also take
          `SnippetField::new(10)`, which box 4 refused. Claude, in 5.8.
       2. `Newtype` has public fields and is written as a struct expression,
          `Newtype::<Blurb, StringField> { field: StringField { … },
          ..Default::default() }`. No macro change, but it exposes the
          `PhantomData` and reads worse.
       **DECIDED 2026-11-26 (Todd): 1, as recommended. SUPERSEDED the same
       day by Y:** `form!` builds the `Newtype` itself, so it never needs to
       take a call, and box 4's rule (a struct expression or a unit struct)
       stays as it is.

    Y. **SUPERSEDED 2026-11-26 by AB (10.2), once AB is built.** **A bare `newtype` key in `form!`.** `bio => { newtype,
       max_length: 280 }` says the path's type is a tuple newtype, and the
       other keys read as if the path were its inner type. No type follows
       the key: `form!` already gets a path's type from rustc by projecting
       into the model (`field_kind::shape_of(|__m: &Model| &__m.bio)`), and
       `newtype` lets it project one step further, `&__m.bio.0`, so rustc
       infers the inner type too. (facet's `Facet` trait has only `SHAPE`,
       so there is no type-level "inner type" to ask for; the projection is
       the only way, and it is enough.) From there:

       1. **The checks** use `shape_of(|__m| &__m.bio.0)`, which is
          `String`'s shape, so `widget: checkbox` or `min: 3` on `bio` is a
          compile error again instead of `Unknown`.
       2. **The settings** go through a sibling of `field_kind::field_for`
          that takes both projections: `P` from `&m.bio`, `I: DefaultField`
          from `&m.bio.0`, the closure configures `I::Field`, and the result
          is `Newtype::<P, I::Field>::new(configured)`. The dev never names
          `Blurb` or `String`.
       3. **Tuple newtypes only**, because of `.0`. On a one-field named
          struct the error is rustc's "no field `0` on type `Bio`"; span it on
          the `newtype` key, and pin it with a golden. Such a type can only
          be registered: box 4 refuses a call in `field:`, and `Newtype`'s
          fields are private, so there is no struct expression to write.
       4. **A flag, like the other bare keys.** Field-body flags are already
          written without a value (`parse_value` refuses `name:` after one),
          and a nested path needs `=>`, so `newtype` followed by `,` or `}`
          is unambiguous.

       A key of its own rather than `field: Newtype<Blurb, String>`: no type
       name to recognise (an alias would break it), and `field:` keeps its
       one meaning, beside which box 4 refuses every setting. `newtype` and
       `field:` together are refused. The inner type must have a
       `DefaultField` (`String`, `bool`, a number); a newtype over a field of
       the dev's own (`Newtype<Wrapper, TimeLimitField>`) is registered with
       `field_for`. Claude, in 5.8. **DECIDED 2026-11-26 (Todd); bare, with
       no type after it, decided the same day (it was `newtype: String`).**

    Z. **`TypeRegistry::newtype::<T, V>()`**, the app-wide twin of Y:
       `field_for(Newtype::<T, <V as DefaultField>::Field>::new(Default::default()))`,
       Todd, in 5.5.2. **DECIDED 2026-11-26 (Todd).** It names `V` where Y
       infers it, because nothing gives the registry a projection to infer
       from; the alternative `.newtype(|m: &Markdown| &m.0)` would infer
       both, and was turned down the same day: a list of types reads better
       in `main` than a closure.

    AA. **A `field:` on a set's items and the `[= v]` key (5.5.5.9).**
       `path!(M.set[= v])` writes `v`'s text from the registry or `Display`,
       because it has no form and cannot see a spec; the item's node writes
       its text with whatever field it was built with. A `field:` on
       `labels[]` whose `format` differs from the registered one (or from
       `Display`, for a type with nothing registered) makes every `[= v]`
       error on that set miss its item. Options:

       1. **Document it, nothing more (recommended).** In `field:`'s docs
          and `path!`'s: a set's items are named by value through the
          type's registered field, so give a set's item type its field in
          the registry, not with `field:`. Cheap, and the case is narrow: a
          set item with a per-path field that also formats differently from
          the app-wide one.
       2. **Refuse `field:` on a set's items** in `form!`, pointing at the
          registry. Closes the gap, but also refuses the harmless case
          where the two fields format the same way.
       3. **Resolve `[= v]` by value instead of text.** Not possible:
          `WireForm::errors_at` resolves on the server from the wire's text
          alone, with no nodes to compare values against.

       Open, for Todd.

    Why `Newtype<T, F>` itself keeps a FIELD as its second parameter, while
    Y and Z work from a value type: a dev must be able to wrap a field of their
    own (`Newtype<Wrapper, TimeLimitField>`), and one trait covering both
    value types and field types would hit Rust's rule against overlapping
    impls. The value-type spelling lives only where formoxus can translate
    it through `DefaultField`.

  - [ ] **5.9** (Todd and Claude) **Build AB.** Written 2026-11-26 by
    Claude. Done NEXT, in place of 5.5.5 as it stood (Todd chose AB.12's
    option 1), and before 5.5.6, 5.5.7 and 5.6. On this branch, as one
    breaking change: the compiler errors are the to-do list. Each step
    says who does it, what, why, and the traps.

    1. **(Todd) Decide AC, AD and AE** (below the steps). Each changes what
       the tests in 2 look like. **Done 2026-11-26: all three as
       recommended.**

    2. **(Claude) Tests first.** Nothing below exists yet, so the suite
       stops compiling until 3 to 5 are done, as with 5.2.
       1. The suite's types move from `install_test_registry` and the
          `register` functions in `suite.rs` to one `types!` list per
          module, pointed at by `FormoxusTypes` (or by `with_types` on a
          hand-built spec, AD). Most tests then need nothing installed.
       2. New tests for each probed behaviour: two lists in one process
          giving one type different fields; a server-style rebuild
          (`empty_form`, then `distribute_values`) building a list item
          and a variant's fields through the kept list; a form with no list
          refusing a custom type.
       3. Override tests. A test binary is one process and `install` runs
          once in it, so the suite installs ONE overrides list for all of
          them, replacing a type that only the override tests use. They
          cover: the override wins over a form's list; it does not reach a
          path the form configured (AB.7); a built-in the app did not
          replace stays the form's own (the trap in 3.8).
       4. Library tests (AE): a fixture library with its own `types!` and
          forms, included by the suite.
       5. trybuild goldens for `types!`' compile-time errors: a type listed
          twice; a `replace` whose `KIND` differs from the included list's
          (AB.8); a replacement widget that cannot render the field, once
          box 6 adds widget entries; no `FormoxusTypes` in scope, per AC.

       **Done 2026-11-26 (Claude), uncommitted.** Type-checked against
       type-only stubs in a scratch copy: the one error left is the
       expected one (`newtypes::a_setting_on_a_listed_newtypes_path_reaches_its_inner_field`
       needs `form!` to ask `FieldFor`, 5.9.5). What was written:
       1. `formoxus-fixture-lib`, a workspace member and a dev-dependency of
          formoxus: `Badge` (never overridden), `Stamp` (the suite replaces
          it), `Seal` (in a second list the suite does not include), a
          `Card` form with `pinned => { field: PinnedStamp }`.
       2. `suite.rs`: `install_test_registry` and the `register` functions
          gone; one `SuiteOverrides` list (includes the fixture, `replace`s
          `Stamp`, lists `Seal` as an own entry) installed by
          `install_overrides()`; `across_the_wire_with(types, model)`.
       3. The seven modules each have a private `types!` list and
          `use … as FormoxusTypes;`; hand-built specs in the tests that met
          a listed type take `.with_types(…::registry())`. 81
          `install_test_registry();` calls gone.
       4. `type_lists.rs`, 17 tests: one list, two lists in one process, a
          rebuild building list items and a chosen variant's fields through
          the kept list, the set-item key through `FormoxusTypes`, a
          library's form, including a library, and five override tests.
       5. `newtypes.rs`: `Motto` listed as `field Newtype<Motto, MottoString>`
          (AG); a new test that a setting on a listed newtype's path
          reaches its inner field (what Y was for); the duplicate test moved
          to a golden; the refusal tests expect the message to name
          `types!` and the line to add.
       6. `custom_fields.rs`: the 7 registered-widget tests plus the one
          that only passed because no widget was registered are ignored
          (AH); the 5.6 widget test and the double-install test moved
          (to a golden and to `type_lists`).
       7. Seven goldens in `tests/ui-pending/` (no `.stderr` until the
          checks exist): a type listed twice, a built-in entry, a `replace`
          with another `KIND`, a `replace` with nothing included, a type
          both listed and included, a path whose type is in no list, and a
          `widget:` that cannot render a listed type.

       The API the tests chose, for 5.9.3 to build or change:
       1. `formoxus::types! { [vis] struct Name; include krate::List;
          T => newtype V, T => field F, replace krate::T => field F }`,
          exported through the prelude like `form!`.
       2. `Name::registry() -> &'static TypeRegistry`,
          `Name::overrides() -> TypeRegistry`, `Name::install()`.
       3. `FormSpec::with_types(&'static TypeRegistry)`.
       4. `formoxus::prelude::FormoxusTypes`, an alias of
          `formoxus::type_registry::BuiltIns`.
       5. `refuse_unregistered_newtype`'s message now has to name `types!`
          and the line to add (`Headline => newtype String`), plus `field:`;
          "transparent" for a transparent type. (Claude wrote that message;
          Claude can update it in 5.9.3 if wanted.)
       6. When `item_value_key` takes the list (5.9.3.10), `path.rs`'s
          inline `a_values_text_is_its_item_entry` loses its unregistered
          `Tag` case (Claude).

    3. **(Todd) The run-time side, in formoxus.**
       1. **`FieldFor<V>` is a trait, generic over the model type `V`.**
          (Expanded 2026-11-26 at Todd's request.) A list is a unit struct,
          `struct AppTypes;`, and `types!` implements the trait for it once
          per type the list holds: `impl FieldFor<Markdown> for AppTypes`,
          `impl FieldFor<String> for AppTypes`, and so on. So "which field
          does `AppTypes` give a `Markdown`" is a trait lookup rustc does at
          compile time, `<AppTypes as FieldFor<Markdown>>::Field`.
          It goes in `type_registry.rs`, beside `TypeRegistry`, because the
          two are one list seen at compile time and at run time:

          ```rust
          #[diagnostic::on_unimplemented(
              message = "`{V}` has no field in `{Self}`",
              note = "list it in `{Self}`'s `formoxus::types!`: `{V} => newtype String`, or `{V} => field YourField`"
          )]
          pub trait FieldFor<V> {
              /// What a path's settings in `form!` configure: the field
              /// itself, or for a newtype entry its inner field.
              type Settings: Default;
              /// The field a path of type `V` gets.
              type Field: Field + Send + Sync;
              /// The field, from settings.
              fn build(settings: Self::Settings) -> Self::Field;
          }
          ```

          Two associated types, not one, because a newtype entry's
          settings belong to its inner field: for `Note => newtype String`,
          `Settings` is `StringField` and `Field` is
          `Newtype<Note, StringField>`, and `build` wraps one in the other.
          For a `field F` entry both are `F` and `build` returns its
          argument. `Settings: Default` because a path with no settings
          starts from the default; `Field: Send + Sync` because
          `field_for` needs it. Nobody writes an impl by hand: `types!`
          writes them all, which is what keeps the compile-time list and
          the run-time registry from disagreeing.
       2. **Built-ins: one list in the macro, `BuiltIns` in formoxus.**
          1. **The list of built-in types lives in formoxus-macros**, in
             `types!`' own module (4): `String`, `bool`, the ten numbers,
             each with its field. Every list gets a `FieldFor` impl for
             each, so `form!` can ask any list about a `String`.
          2. **`BuiltIns` is declared in formoxus by invoking the macro**,
             in `type_registry.rs`: `formoxus_macros::types! { pub struct
             BuiltIns; }`, a list with no entries of its own. The macro
             writes `::formoxus::…` paths, so `lib.rs` needs
             `extern crate self as formoxus;` for them to resolve inside
             formoxus itself. The prelude then exports
             `pub use crate::type_registry::BuiltIns as FormoxusTypes;`
             (AC), and `BuiltIns::registry()` is what a form with no list
             falls back to (3.7).
          3. **Order trap: `BuiltIns` needs `types!`, which is Claude's
             step 4.** Do the rest of 3 first; `BuiltIns` is one line once
             4 exists. Until then nothing needs it to compile but the
             prelude line.
          4. **How a list's registry gets its built-ins.** `registry()`
             builds a `TypeRegistry` from every `FieldFor` impl, built-ins
             included, but AF.1 has the public `field_for` refuse a
             built-in type. So the generated code calls a hidden
             `#[doc(hidden)] pub fn __built_in<F: Field + Send + Sync>(self,
             field: F) -> Self`, which is `field_for` without that check.
             The check itself needs the run-time side to know which shapes
             are built-in: a `fn is_built_in(shape) -> bool` on the
             scalar types the macro lists. That is a second copy of the
             list, so a test pins that every built-in the macro writes is
             one `is_built_in` accepts, and the other way round.
          5. **`DefaultField` and its impls are deleted** (AB.10.4);
             `fields.rs` keeps `Number`'s impls, which `NumberField<T>`
             needs.
       3. **No run-time `replace` is needed.** Earlier this step had a
          `replace_field_for`. Under AF a `replace` supersedes only an
          entry from an included list, and `types!` sees both through the
          replay, so it drops the superseded entry before writing anything:
          the run-time builder never sees two fields for one type, and
          `field_for`'s duplicate panic stays as it is. `overrides()` is
          the `replace` entries alone, through the ordinary `field_for`.
       4. **Your `FieldInfo.format_value`** keeps the shape it has: a
          closure from a `Peek` to the field's text, built in `field_for`
          from a clone of the field (clone BEFORE `NodeCtor::for_field`
          takes it). What changes is only which registry it is looked up
          in (7).
       5. **`FormSpec.types: Option<&'static TypeRegistry>`** and a public
          `with_types(&'static TypeRegistry)`, which `form!` calls with
          `FormoxusTypes::registry()`. `&'static` because each list is a
          static, which also keeps `FormSpec` cheap to clone.
       6. **`Walk.types`, and the copies that walk later.** `Walk` gains
          the form's list, set from the spec in `form_for_impl`.
          **Trap:** `Items` and `VariantNode` must keep `walk.types` when
          they are built (a `with_types` on `Items`, a field on
          `VariantNode`), and use it when they build a `Walk` of their own
          (`items.rs`, `variant_node.rs`). They walk again later, when the
          server fills a form from the wire, when an item is added, and
          when a variant is chosen. Forget it and those later walks quietly
          use only the installed list and the built-ins: the form works
          until a user adds an item. The probe changed exactly these
          places.
       7. **One lookup, in AB.6's order,** used by the walk, by the
          set-item key and later by box 6's widgets: the path's spec; then
          the installed list; then the form's list, or `BuiltIns::registry()`
          when the form has none. A single function, because three copies of the
          order would drift.
       8. **OVERTAKEN by AF (no list can replace a built-in, and only
          `replace` lines are installed).** Kept for the reasoning. **Trap:
          the installed list must not hold the built-ins the app left alone.** If `AppTypes::install()` installed the whole list,
          its untouched `String` would beat a library form's own
          `replace String`, since the installed list is asked first. So
          `types!` installs only what the app chose: its own entries, its
          `replace`s, and what it included. The run-time side needs nothing
          for this beyond `install` taking whatever registry it is given;
          the point is in `install`'s docs and in 4.
       9. **The walk's scalar arm and `scalar_node` go.** Every built-in
          now comes through 7. Keep a clear panic for a scalar with no
          field (a `char`, an `i128`), which today comes from `scalar_node`
          returning `None`.
       10. **The set-item key takes the form's list.**
           `item_value_key` and `entry_text` get the list as a parameter
           and format through 7's order. It must be the same order as the
           walk's, or a key computed under an override would not match the
           item the walk built.

    4. **(Claude) `types!`**, a proc macro in formoxus-macros
       (`formoxus-macros/src/types.rs`, with its `#[proc_macro]` entry in
       `lib.rs` beside `form`), exported through the prelude like `form!`
       (`pub use formoxus_macros::{form, path, types, using_fns};`), so
       `formoxus::types!` works. It holds the one list of built-in types
       (3.2.1). The grammar from AB.2. A `newtype V` entry's
       inner field is the list's own entry for `V`
       (`<Self as FieldFor<V>>`), never a fixed table (AB.10.4). It writes: the struct;
       one `FieldFor` impl per entry, per built-in not replaced, and the
       `Option<T>` impl; `registry()` (everything) and the installed
       overrides (3.8), each a `LazyLock` static; `install()`; and the
       replay `macro_rules!` of AB.4, written by the proc macro so its
       `$crate` names the right crate. `include` chains through an
       accumulator, as in the probe. AB.8's check: through the replay it
       sees the included entry a `replace` supersedes, and emits a free
       `const _` comparing `KIND`s, with a message naming both lists.
       Diamond includes (AB.11.1) wait until a real case: today they are a
       plain E0119.

    5. **(Claude) `form!` and `path!` use `FormoxusTypes`.** `field_kind`'s
       settings and check helpers are bounded on `DefaultField` today
       (`field_for`, `choice_field_for`, `ValueOf`, two more); they take
       the list as a type parameter and ask `FieldFor` instead, and
       `DefaultField`'s `on_unimplemented` message moves to `FieldFor`'s.
       `form!` emits `FormoxusTypes` in each of them and
       `.with_types(FormoxusTypes::registry())` on the spec; `path!` passes
       `FormoxusTypes::registry()` to `item_value_key`. Re-bless the goldens
       and read every diff: the spans of an unlisted type's error move.

    6. **(Claude) The rest of the tree.**
       1. The examples crate: its forms need no change, since the prelude's
          `FormoxusTypes` covers forms over built-in types (AC). Any custom
          type it uses gets a list.
       2. `type_registry.rs`' module docs, rewritten around `types!`: what
          a list is, `FormoxusTypes`, `include` and `replace`, the lookup
          order, why built-ins never change (AF), and what `install` is
          now for. The same for `TypeRegistry`, `field_for` and `install`,
          whose docs still describe the process-wide registry as the only
          copy.
       3. The `build.rs` comment on the constructor arm, which still says
          registering a built-in replaces it everywhere (AF.4).
       4. `refuse_unregistered_newtype`'s message (5.9.2's API item 5).
       5. Memory: the probe file, `next_session.md`, and a new entry for
          the lookup order if the docs alone do not carry it.

    7. **(Todd) Review, then on to 5.5.6.** 5.5.6's path check and 5.5.7's
       message are unchanged by AB. 5.8 loses Y. Box 6 becomes widget
       entries in `types!` (AB.11.3), looked up in 3.7's order.

    AC. **DECIDED 2026-11-26 (Todd): 1, the prelude's `FormoxusTypes` (Todd
        named it; `FormTypes` before), an
        agreed exception to "no new re-exports until publish".**
        **What `form!` does with no `FormoxusTypes` in scope (AB.11.4).** The
        suite has 321 `form!` calls, the examples more, apcsp more again,
        and most forms use only built-in types. Options:
        1. **(Recommended) The prelude supplies a fallback.**
           `formoxus::prelude` exports `FormoxusTypes`, an alias for
           `BuiltIns` (3.2). A glob import is shadowed by any explicit
           `use` or local item of the same name, so a module that writes
           `use AppTypes as FormoxusTypes;` gets its own list, and every other
           form keeps working unchanged. This is a new prelude export,
           which the 2026-11-09 rule ("no new re-exports until publish")
           needs Todd to allow; it is the one export that is the point of
           the design rather than a convenience.
        2. Require it: every module with a form names a list. Explicit,
           and hundreds of edits for no gain where only built-ins appear.
        3. A `types:` key on `form!` naming the list, defaulting to 1 or 2.
           Useful alongside 1 for a form that wants a list other than its
           module's; not needed to start.

    AD. **DECIDED 2026-11-26 (Todd): as recommended.**
        **What a hand-built `FormSpec` with no list sees (AB.11.5).**
        Recommended: the installed list, then the built-ins, through 3.7's
        order with no form list. The suite's 369 hand-built specs then keep
        working, and one that needs a custom type adds
        `.with_types(SomeTypes::registry())`. The alternative, built-ins
        only, would hide the app's overrides from any spec built by hand,
        which is the inconsistency AB exists to remove.

    AE. **DECIDED 2026-11-26 (Todd): as recommended.**
        **Where the library tests live (2.4).** Replay needs a second crate.
        Recommended: a workspace member `formoxus-fixture-lib`
        (`publish = false`) holding a `types!` list, a model type re-exported
        from its root, and a form, with formoxus taking it as a
        dev-dependency. Cargo allows that cycle for dev-dependencies, and
        the suite is an integration test, so it links the same formoxus the
        fixture does. The alternative is to test libraries only in the
        examples crate, which `just ci` builds but whose tests are thin.

    AF. **DECIDED 2026-11-26 (Todd, with Claude's corollary): built-ins
        cannot be replaced, and an override is exactly a `replace` line.**
        `String`, the numbers and `bool` read the same in every form of
        every crate. A dev who wants a shouting `String` declares a type of
        their own and uses it, or writes `use … as …`. So:
        1. `types!` refuses an entry for a built-in type, with a message
           saying it is the same everywhere, and a hand-built
           `TypeRegistry::field_for` panics for one.
        2. `include` brings a library's own entries, nothing else.
        3. `install` installs only the app's `replace` lines, not its own
           entries: an own entry for a type the app did not include (a
           library's `Badge`) must not quietly override that library's
           forms. A `replace` needs its type to come from an included list.
        4. This removes 3.8's trap (no list can replace `String`, so a
           built-in can never beat a library's choice), and reverses two
           passages that promise replacing a built-in: `type_registry.rs`'s
           module docs and the comment on the constructor arm in `build.rs`.
           No test relied on it.

    AG. **DECIDED 2026-11-26 (Todd): no settings in `types!`.** An entry
        names a field TYPE, built with `Default`. A type-wide default is a
        field of the dev's own whose `Default` carries it:
        `Motto => field Newtype<Motto, MottoString>`, where `MottoString`'s
        `Default` has `max_length: Some(5)`. `FieldFor` needs no
        `settings()`. A form's settings on such a path reach only what the
        entry's field makes public, and `Newtype` makes nothing public: a
        type-wide default belongs on a type of its own.

    AH. **DECIDED 2026-11-26 (Todd): the registered-widget tests wait for
        box 6.** The 7 in `custom_fields` test a type's registered widget,
        which box 6 redesigns as `types!` entries; a `widget:` on the path
        would test something else, already covered. They are
        `#[ignore = "box 6: widget entries in types!"]` until then, with
        their bodies unchanged, so they are rewritten once.

    AI. **Found 2026-11-26 writing 2: the replay macro gets a name of its
        own, and only a `pub` list has one.** AB.4 gave it the struct's name
        so one `use` would bring both. But an app declares its list in one
        module and points forms at it from others, `use crate::types::AppTypes
        as FormoxusTypes;`, and that absolute path then also reaches the
        macro, which rustc denies for a crate's own macro-generated
        `#[macro_export]` (the lint the probe hit in the library). So:
        1. The replay macro is `__formoxus_replay_<Name>`, hidden, at the
           crate root as every `#[macro_export]` is.
        2. `include` takes the list's crate and name, `include
           fixture::FixtureTypes;`, and calls
           `fixture::__formoxus_replay_FixtureTypes!`, wherever in the crate
           the struct itself lives.
        3. Only a `pub` list gets one: no other crate can include a list
           that is not `pub`, and two private lists of one name in two
           modules would otherwise collide at the crate root.

  - [ ] **5.6** (Todd) `kind` and the run-time checks (O.2). Expanded
    2026-11-26 by Claude. Two halves: loosen the compile-time checks for a
    type the compiler cannot judge, then make the build check what they no
    longer do. The test that waits on it is
    `a_widget_that_cannot_render_the_registered_fields_kind_panics_at_build`.

    1. **`kind` gives `Unknown` for every user struct.** A plain newtype
       already gets `Unknown` today: `is_newtype` with no `inner` (the
       `None => Kind::Unknown` arm, `field_kind.rs:811`), because a tuple
       struct's field shape sits behind a `ShapeRef` fn pointer const code
       cannot call. What changes is the last `else`: a
       `Type::User(UserType::Struct(_))` gives `Unknown` instead of
       `Other`. And, by W, a transparent struct gives `Unknown` too: drop
       the `shape.inner` following in `kind` (the `is_newtype` arm) and in
       `is_unit_enum`, so `Markdown` is judged like any other user struct. Not every `Other`: an unsupported primitive (`usize`,
       `char`) and anything else that can never be registered stays
       `Other`. Rewrite `Unknown`'s doc: it now means "a type the compiler
       cannot judge, because the app may register a field for it at run
       time, and `form!` cannot see the registry", with the newtype case
       as one instance of that.

    2. **What 1 loosens.** Each const check that reads `kind` treats
       `Unknown` as "let it through". So for a struct path:
       1. `is_single_value` (`field_kind.rs:392`): `widget:` on a struct
          compiles. Golden `form_widget_on_a_struct` stops failing.
       2. `applies`/the attribute check (`:334`): `max_length:` and even
          `class` on a struct compile. Golden
          `form_constraint_on_a_struct` stops failing, and the inline test
          `a_struct_and_a_list_still_take_nothing_not_even_class` fails for
          `Address`.
       3. `is_leaf`/`is_list_of_leaves` (`:476`): `set` on a
          `Vec<Answer>` compiles. Golden `form_set_on_a_list_of_structs`
          stops failing. **This one is not in O.2's text**, which names only
          `widget:` and table attributes. It follows from the same rule (a
          registered struct IS a leaf, so a set of `Ref<Question>` must be
          allowed), but say if you would rather keep it a compile error at
          the cost of a set of registered structs; see letter U.
       4. `renders` (`:526`) and `field_type` (`:860`) already treat
          `Other` and `Unknown` alike, so nothing changes there.

       The goldens and the inline test are Claude's, in 5.8, so expect
       `just ci` to be red on trybuild and that one lib test between this
       step and 5.8.

    3. **Then the build refuses what the compiler let through.** Each check
       lives where the walk already decides what a struct path is:
       1. **An unregistered struct with a widget or attributes.**
          `struct_node` already asserts `spec.widget.is_none()`
          (`build.rs:809`). Add `spec.attrs` being empty to the same
          assert, and reword its message to name the type and say
          `TypeRegistry::field_for` or `field:`. Today's message ("did you
          mean `label`?") is still the right hint for a struct the dev
          meant as a fieldset, so keep it as the second half.
       2. **A set over items that are not leaves.** `set_node` builds each
          item through `node_for_shape`, so an unregistered struct item
          would become a fieldset inside a set, which the set's rendering
          and its `[= v]` keys do not expect. Assert, once, before building
          items, that the item shape is something a leaf is built from:
          a scalar, a unit enum, or a registered type.
          The message names the type and `TypeRegistry::field_for`. Needed
          because of U.
       3. **A registered field's widget and attributes, in
          `LeafArgs::node`.** After the widget is chosen, assert that it
          renders `F::KIND`: `widget.accepts().contains(F::KIND)`, mirroring
          `renders`, so `|| (TAKES_CHOICES && has_choices)` once Q is back.
          `AnyWidget::accepts()` already exists (`pub(crate)`, from the
          upload work), so the parenthesis in the old text is moot. The
          message names the path and `widget.name()`; the test checks for
          `limit` and `int_only`. Likewise for attributes: each
          `spec.attrs` key must apply to `F::KIND`, the run-time twin of
          `applies`. Scope these to the leaves the compiler could not
          judge, `kind(self.shape)` being `Unknown` (expose a
          `pub(crate) const fn is_unknown`). See letter V for why not every
          leaf.

    4. **The old 5.1's proxy scan is not needed**, as before: there is no
       proxy.

    Then the suite should be 1015 passed, with only 6.2's 7 failing, plus
    the 5.8 goldens and the one inline test.

    **Letters for Todd (2026-11-26, Claude recommends; both DECIDED the same
    day):**

    U. **A set of structs compiles (2.3).** Recommended: yes, as `Unknown`
       implies, with 3.2's run-time assert. A registered struct is a leaf,
       and the motivating case, a set of `Ref<Question>`, needs this.
       Alternative: a separate kind for user structs that every check but
       `is_leaf` treats as `Unknown`, which keeps `set` on a `Vec<Answer>`
       a compile error and makes a set of registered structs impossible
       until #18 can check against the registry.
       **DECIDED 2026-11-26 (Todd): yes, as recommended.**

    V. **Mostly overtaken 2026-11-26 by AB (10.3), once AB is built:** what
       stays run-time is AB.9's second-to-last row. **The run-time widget and attribute checks only where the compiler
       could not judge (3.3).** Recommended, because it is what O.2 says
       and because it changes nothing for a hand-built `FormSpec` over
       ordinary types: `field_kind.rs:381-385` records that a pair `form!`
       refuses still renders from a hand-built spec, on purpose.
       Alternative: check every leaf. That is one rule rather than two and
       is harder to misuse, since a hand-built spec would get the same
       refusal `form!` gives, but it is a behaviour change for hand-built
       specs, and any suite test that builds a refused pair by hand would
       start to panic. The suite would tell us how many in one run.
       **DECIDED 2026-11-26 (Todd): yes, only the `Unknown` leaves.**

  - [ ] **5.7** (Todd, DEFERRED with Q) Q's run-time side: `FieldText<P>`, the registry storing
    it, and the choice field over `P` that reads it.

  - [ ] **5.8** (Claude) `form!`'s side: J's dispatch with Q's fallback, O.1's
    note on the settings route, and the goldens that change (O.2). Then the
    suite is green. Added 2026-11-26: Y's bare `newtype` key, with its tests
    and goldens (it replaces X's call form, which is not built); the goldens and inline `field_kind` test 5.6
    turns red (5.6.2); `form_setting_on_a_newtype`'s note pointing at
    `fields::Newtype`; and a third route in `Newtype`'s and
    `TypeRegistry`'s docs, the `newtype` key, once it exists (5.5.2.6's docs
    leave it out until then), and in `refuse_unregistered_newtype`'s
    message, whose per-path advice is `field:` until then.

- [ ] **6.** (Todd) The per-type widget, chosen at build (P). Rewritten
      2026-11-25 by Claude; the render-time version (6.1 to 6.5 of
      2026-11-24, in `a8463e0`) is superseded with L. **AB (10.7) makes the
      per-type widget an entry in `types!` once built**; the lookup order is
      AB.6's.

  - [ ] **6.1** (Todd) `widget_for` itself moved to 5.3.4 on 2026-11-26,
    because the suite's registry calls it. What is left here is the
    compile-time half: check the widget against `T` with an inline `const {
    assert!(field_kind::renders::<W>(T::SHAPE, false)) }`, which fires when the
    app is built (not under `cargo check`; see
    `.claude/memory/const_shape_walk_blocked.md`). After W this only catches
    something for a BUILT-IN type, `widget_for::<bool>(Textarea)` say,
    since every user type is `Unknown` to `kind` and `renders` lets it
    through. That case is still worth it: a built-in type has no registered
    field, so 5.3.3 and 5.3.4's run-time check never sees it, and V's build
    check is only for `Unknown` leaves.

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

- [ ] **11.** (Todd decides, then both) **More built-in types.** Added
      2026-11-26 at Todd's request: `Email(String)`, `Password(String)` and
      others Django's forms offer, so a dev does not write the same newtype,
      field and widget in every app. Under AB they would be built-ins: in
      every `types!` list, and like `String` never replaceable (AF). After
      5.9, since it extends `types!`' own list of built-ins (5.9.3.2).

      What Django has that formoxus does not, each with the field formoxus
      would give it:

      1. `EmailField`: `Email(String)`, an `email` input, a check that it
         looks like an address.
      2. `Password(String)`: not a Django field (Django uses a
         `PasswordInput` widget on a `CharField`), but here the type would
         carry it: a `password` input, never trimmed (#16), never refilled
         from a stored value.
      3. `URLField`: `Url(String)`, a `url` input and its check.
      4. `SlugField`: `Slug(String)`, letters, digits, `-` and `_`.
      5. `UUIDField`: a UUID.
      6. `GenericIPAddressField`: `std::net::IpAddr`, which is already a
         type nobody needs to define.
      7. `DecimalField`: a decimal, which needs a crate (`rust_decimal`).
      8. `DateField`, `TimeField`, `DateTimeField`, `DurationField`: `date`,
         `time` and `datetime-local` inputs, which need a date crate
         (`jiff`, `time` or `chrono`) or `std::time::Duration` for the last.
      9. `JSONField`, `RegexField`: likely not; a `pattern:` setting covers
         most of the second.

      AJ. **Where an `Email` type lives, so models stay plain.** A model
          that writes `email: formoxus::Email` would depend on formoxus,
          which "models stay plain" rules out (apcsp's `models` crate does
          not depend on it). Options:
          1. **(Recommended) A tiny crate of value types,** say
             `formoxus-types`, depending only on `facet`: `Email`,
             `Password`, `Url`, `Slug`. A models crate can depend on it
             with no form library behind it, the way decision I has
             `Ref<T>` living outside the storage layer. formoxus's built-ins
             list knows them. The test for each type is whether it means
             something without a form: an `Email` does; a `Password` is
             arguably a storage concern too.
          2. Types from crates the models already use, recognised behind
             cargo features: `std::net::IpAddr` needs none; `uuid::Uuid`,
             `jiff::civil::Date`, `rust_decimal::Decimal`, each behind a
             feature, so formoxus does not pull them in for everyone.
          3. Both: 1 for the types that are only text with a meaning, 2 for
             the ones a real crate already defines.

          1 and 2 are not rivals: a model's date is almost certainly already
          a `jiff` or `chrono` type, while nothing upstream defines `Email`.
          So the likely answer is 3.

      AK. **Which, and in what order.** Recommended first: `Email`,
          `Password`, `Url`, then the date and time types, since forms ask
          for those most and the browser already has an input for each.
          `Decimal`, `Uuid`, `IpAddr` and `Slug` when a real form needs one.
