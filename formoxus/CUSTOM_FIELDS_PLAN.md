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
      | The app's own forms, against `AppTypes` | compile time, all of it but one case (below) |
      | A library's forms, against its own list | when the library compiles |
      | The app's `replace`s, against the library's entries | when the app compiles |
      | An installed widget meeting a library form's own `field:` (a field type the library's list does not hold) | when that form is built, as V's checks are now |
      | A hand-built `FormSpec` with no list | when the form is built, against the built-ins and the installed list |

      The one case, found building it (5.9's G): a path that mentions an
      unlisted newtype with no settings and no widget (`shade => { label:
      "Shade" }`) is refused when the form is built. A const cannot tell
      `Shade(String)`, which is refused, from `Wrapper(Address)`, which is
      walked as a struct, since a tuple struct's field shape is a fn
      pointer. `form_unlisted_type` stays in `ui-pending/`.

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

**Box 5's decisions**, moved here and shortened 2026-11-27 when box 5 was
pruned. Code and tests cite these letters. The full reasoning, with the
options turned down, is in git at `9cd17bf`.

M. **DECIDED 2026-11-25: the registry is a type of its own,
   `formoxus::type_registry::TypeRegistry`, not part of `Formoxus`.**
   `Formoxus` also reaches a subtree through `provide_defaults`, so a field
   registered there would reach the client's build but not the server's.
   A separate type that can only be installed makes that impossible. Under
   AB it is what a `types!` list builds, and `install` holds only the
   overrides (AF.3).

N. **DECIDED 2026-11-25, amended by W: formoxus no longer unwraps a newtype
   on a guess.** `FieldNode::wrapper` is gone (5.5.3).

W. **DECIDED 2026-11-26 (Todd): no newtype is special, and formoxus ignores
   `#[facet(transparent)]`.** Explicit is less surprising. A newtype gets its
   field from a list, `Markdown => newtype String`, built as
   `fields::Newtype<T, F>`, which passes everything to an inner field and
   wraps and unwraps through facet reflection, so the model needs no `From`
   impls. It buys one rule (a type the compiler cannot classify needs a
   field), no `begin_inner` write-back, settings on a newtype path, and no
   `try_from` trap (a transparent type refusing a value inside
   `write_value_into`, after `validate()` passed, with no field error for
   the user). **Revisit if it proves obnoxious (Todd):** handle a
   transparent wrapper over a `String`, `bool` or number automatically,
   after the list lookup, so a listed field still wins.

O. **SUPERSEDED 2026-11-26 by AB and 5.6's revision.** O assumed `form!`
   cannot see what the app registers, so a struct or newtype path would be
   `Unknown` and checked only at build. `form!` reads the list in scope
   (`TypeKinds::KINDS`), so a listed type is judged at compile time by its
   field's `KIND`, an unlisted struct stays `Other` and is refused, and
   only an unlisted newtype is `Unknown`. O's `FormSpec::check()` is
   DEFERRED to #18.

P. **DECIDED 2026-11-25: the per-type widget is chosen once, at build,** in
   `LeafArgs::node`, in H's order: the path's `widget:`, then the type's
   widget, then the field's `default_widget`. Server and client then pick
   the same widget, which supersedes L. Under AB the type's widget is an
   entry in `types!` (box 6).

Q. **DEFERRED 2026-11-25 (Todd), BACK 2026-11-27 as 5.7, whose revision replaces the mechanism below: `choices:` on a listed type**
   (`Ref<Question>` in `groups[].questions[]`). Recommended when it comes
   back: a choice field over `P` whose `format` is `P`'s listed field's,
   and whose `parse` finds the choice whose text equals the input, so it
   never parses `P` itself. The registry hands that text out typed, as an
   `Arc<dyn FieldText<P>>` behind an `Arc<dyn Any + Send + Sync>`. A
   `choices:` on an unlisted `P` panics at build. Waits in 5.7.

U. **DECIDED 2026-11-26 (Todd), met by 5.6's revision: a set of LISTED
   structs compiles** (`is_leaf` reads `KINDS`), since a listed struct is a
   leaf and a set of `Ref<Question>` needs it. A set of unlisted ones stays
   a compile error; 5.6.3.2 backs that up for a hand-built spec.
   **AMENDED 2026-11-27 by AN:** a set of unlisted structs is allowed too.

V. **DECIDED 2026-11-26 (Todd), scoped by AL: run-time widget and attribute
   checks only where the compiler could not judge.** A hand-built spec over
   built-in types keeps rendering a pair `form!` refuses, on purpose
   (recorded in `field_kind.rs`). Checking every leaf would be one rule, but
   a behaviour change for hand-built specs.

X. **SUPERSEDED 2026-11-26 by Y, then AB.** `field:` still takes only a
   struct expression or a unit struct; nothing needs it to take a call.

Y. **SUPERSEDED 2026-11-26 by AB.** No bare `newtype` key in `form!`. A
   list's `T => newtype V` entry does its job, and a path's settings reach
   the inner field through `FieldFor::Settings`.

Z. **SUPERSEDED 2026-11-26 by AB.** No `TypeRegistry::newtype::<T, V>()`;
   the list entry replaced it.

AA. **OPEN, for Todd: a `field:` on a set's items and the `[= v]` key.**
    `path!(M.set[= v])` writes `v`'s text through the list, because it has
    no form and cannot see a spec, while the item's node writes its text
    with whatever field it was built with. A `field:` on `labels[]` whose
    `format` differs from the listed field's makes every `[= v]` error on
    that set miss its item. It predates AB (box 4's `field:`). Options:
    1. **Document it, nothing more (recommended).** In `field:`'s and
       `path!`'s docs: a set's items are named by value through the type's
       listed field, so give a set's item type its field in `types!`, not
       with `field:`. The case is narrow.
    2. **Refuse `field:` on a set's items** in `form!`, pointing at
       `types!`. Closes the gap, but refuses the harmless case where both
       fields format the same.
    3. **Resolve `[= v]` by value.** Not possible: `WireForm::errors_at`
       resolves on the server from the wire's text alone.

AC. **DECIDED 2026-11-26 (Todd): the prelude exports `FormoxusTypes`,** an
    alias of `BuiltIns`, an agreed exception to "no new re-exports until
    publish". A glob import is shadowed by an explicit `use`, so a module
    that writes `use AppTypes as FormoxusTypes;` gets its own list and every
    other form keeps working. A `types:` key on `form!` (AB.11.4) is not
    needed to start.

AD. **DECIDED 2026-11-26 (Todd): a hand-built `FormSpec` with no list sees
    the installed overrides, then the built-ins.** One that needs a custom
    type adds `.with_types(SomeTypes::registry())`. Built-ins only would hide
    the app's overrides from specs built by hand.

AE. **DECIDED 2026-11-26 (Todd): the library tests live in
    `formoxus-fixture-lib`,** a `publish = false` workspace member that
    formoxus takes as a dev-dependency, since replay needs a second crate.

AF. **DECIDED 2026-11-26 (Todd, with Claude's corollary): built-ins cannot
    be replaced, and an override is exactly a `replace` line.** `String`, the
    numbers and `bool` read the same in every form of every crate; a dev who
    wants a shouting `String` declares a type of their own. So:
    1. `types!` refuses an entry for a built-in type, and
       `TypeRegistry::field_for` panics for one (5.9.3.2.4).
    2. `include` brings a library's own entries, nothing else.
    3. `install` installs only the app's `replace` lines: an own entry for a
       type the app did not include must not quietly override a library's
       forms. A `replace` needs its type to come from an included list.
    4. Nothing can beat a library's choice for a built-in, so the installed
       list never needs to leave the built-ins out.

AG. **DECIDED 2026-11-26 (Todd): no settings in `types!`.** An entry names a
    field TYPE, built with `Default`. A type-wide default is a field of the
    dev's own whose `Default` carries it,
    `Motto => field Newtype<Motto, MottoString>`.

AH. **DECIDED 2026-11-26 (Todd): the registered-widget tests wait for box
    6.** The 8 in `custom_fields` are
    `#[ignore = "box 6: widget entries in types!"]` with their bodies
    unchanged, so they are rewritten once.

AI. **Found 2026-11-26: the replay macro has a name of its own, and only a
    `pub` list has one.**
    1. It is `__formoxus_replay_<Name>`, hidden, at the crate root, as every
       `#[macro_export]` is.
    2. `include` takes the list's crate and name, `include
       fixture::FixtureTypes;`, wherever in the crate the struct lives.
    3. Only a `pub` list writes one: two private lists of one name in two
       modules would otherwise collide at the crate root.

AL. **DECIDED 2026-11-26 (Todd): 5.6.3.3 checks the leaves whose field came
    from a list or the overrides,** not those `kind` calls `Unknown`: a
    listed struct is `Other` to `kind`, so that test would skip exactly the
    leaves the check is for.

AN. **DECIDED 2026-11-27 (Todd): a set holds what a `BTreeSet` field holds,
    structs included.** An unlisted struct needs no entry to be a set item:
    each item is a group of fields, which `SetNode` already shows (its
    fields joined), edits (through a draft that is a group of fields) and
    orders (a custom `sort_by` rebuilds it). So `set` on a `Vec` and
    `treat_as_set` take the same items a `BTreeSet` field does
    (`field_kind::is_list_of_set_items`, which replaced `is_list_of_leaves`):
    one value, or a group of them (a struct, or an enum with data). Not an
    `Option`, a list or a set: a list or set item writes its own item keys
    at its path, so a set would show and compare its items by keys (probed:
    a set of `Vec<String>` builds and validates, and shows "#0,#1, a, c").
    Todd's two conditions:
    1. **An order**, at compile time: a struct has no default order, so
       `form!` requires `sort_by` (`form_set_on_a_list_of_structs`).
    2. **A known struct**, at build time: the set always builds its draft,
       whose walk refuses a field formoxus cannot build, naming that field.
       Const code cannot read a struct's field types, so not at compile time.

AO. **SUPERSEDED 2026-11-27 by AR.** **`path!(…[= v])` for a struct item.** A set item is
    found by its own entry, and a struct item has none (its fields write at
    `pins.#3.label`, `pins.#3.rank`), so a validator's `[= v]` on a set of
    structs names no item, and its error shows with the form's. Placing is
    not the problem: an error at `pins.#3` already renders on that item, as
    a list item's does. A list never meets this, since it finds an item by
    position (`[i]`), and a set cannot: the validator counts in the model's
    iteration order, which is not `sort_by`'s, and a `HashSet`'s is not even
    the same between two copies. Recommended fix: for a struct item, match on
    its fields. `path!` writes `v` as (field path, text) pairs, each through
    the list as today, and `resolve` picks the item where each pair matches
    the entry at `pins.#k.<field>`, which works on the server from the wire
    alone. That is exactly a set item's identity, since a set is unique on
    the whole value. Costs an escaping rule for the separators, and a scope:
    leaf and nested-struct fields only, since a `Vec` field's item keys move.

AP. **DECIDED 2026-11-27 (Todd): a group needs names, so every tuple
    struct needs a field from a list.** A struct is a group of its NAMED
    fields: each name is a label and a path segment. A tuple's parts have
    neither (`default_label` gives digits no label), so walked as a group,
    `Imag(f32)` is a fieldset around one unlabelled input at `imag.0`, and
    `Pair(u8, u8)` two of them. So a tuple struct of any arity, a newtype
    included, builds only through a list entry, which makes it one value, a
    leaf, whatever its parts are. That forces the entry for a newtype, whose
    only reason to exist is to be treated differently from what it wraps
    (`Markdown => newtype String`; a widget alone is not enough, since
    formoxus would then have to take the field from the inner type, the
    guess N and W retired). And it lets any tuple struct into a model, even
    `Address(u32, String, String, State, Zip)` (Todd's deliberately horrible
    example), as long as the dev lists it with a field of their own that
    parses and formats it as text, whose `default_widget` renders it (box 6
    adds a widget entry beside it). This replaces 5.5.4's narrower rule (one
    field, over a scalar), which let `Pair(u8, u8)` and `Wrapper(Address)`
    through as groups.

    1. **A plain tuple, `(u8, u8)`, is not handled at all:** not walked and
       not listable, since an entry for an anonymous type would be awkward
       to name and to write a field for. `types!` refuses one as an entry,
       and the build refuses one in a model, saying to declare a tuple
       struct and list that.
    2. **A tuple variant with one field** (`Mood::Bold(Shade)`) is the
       exception: its variant name labels its payload.
    3. **A tuple variant with several fields** (`Pen::Rgb(u8, u8, u8)`) is
       refused when the form is built, as a tuple struct would be without
       its entry (Todd, 2026-11-27). The dev owns the enum, so a struct
       variant, `Rgb { r: u8, g: u8, b: u8 }`, costs nothing; or the payload
       becomes one listed tuple struct, `Rgb(Color)`. An enum the dev does
       not own can be listed whole with a field of its own.

    **Aliases** (`type Date = String;`) are not types: Rust replaces the
    name before anything sees it, so a `Date` reads exactly as a `String`
    everywhere, and a `types!` entry for one is rustc's E0119 against the
    built-in `String`. Documented, not handled: it is a fact about Rust,
    and the reason the newtype pattern exists (Todd, 2026-11-27).

AQ. **DECIDED 2026-11-27 (Todd): a set may hold lists and sets.** Nothing
    principled refused them: a set whose items have no default order needs
    `sort_by`, and that custom order dedups and finds items by rebuilding
    each item's value and comparing with `==`, which is right for a
    `Vec<String>`. What is missing is the text a list or set item shows (its
    own item keys, "#0,#1, a, c") and a browser test of a set inside a
    set's draft. An `Option` item stays refused, on principle: a set refuses
    a blank item, and `None` is blank, so it could never be added. Box 12.4.

AR. **DECIDED 2026-11-27 (Todd): `path!(…[i])` on a set counts in the
    validator's own order.** Replaces AO. Set items already have keys (`#k`,
    from `Items`, as list items do) that survive round trips; what a
    validator lacks is a way to name one, since it sees the plain model. A
    list bridges that by position, because the `Vec`'s order is the order
    shown. A set can too, if `[i]` counts in the order the validator
    iterates: formoxus builds the model it hands the validator, so right
    after building it, it records each set's item keys in that instance's
    iteration order, matching each element to its item node by value. Then
    `resolve`, unchanged, is given those keys for a set. Works for any item
    type (struct, list, set), needs no text encoding, and avoids AA for
    `[i]`. `[= v]` stays, as the shorthand for a set of leaves. **Trap:** the
    index means something only in the instance formoxus handed over; a
    validator that iterates its own copy of a `HashSet` may count in another
    order (a `BTreeSet` iterates by `Ord`, so any copy agrees). Box 12.3.

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

- [ ] **5.** (Todd, with Claude on tests, `form!`'s side and every message a
      dev reads) The type registry, built as AB: a `types!` list per app or
      library, which `form!` and `path!` read through the `FormoxusTypes` in
      scope, plus the `replace` lines an app installs as overrides. Pruned
      2026-11-27 by Claude: the finished and superseded steps are summarized
      below, their decisions moved to the Decisions section (M to AL), and
      the full text is in git at `9cd17bf`.

  **Done.**

  1. **5.1** (2026-11-25): a value set in `main` before `launch` is seen by
     a server function, so an installed registry reaches both sides.
  2. **5.2 to 5.4** (2026-11-26): the process-wide `TypeRegistry`
     (`field_for`, `install`, a panic on a second field or widget for one
     type), and the walk asking it right after the spec's constructor.
  3. **5.5** (2026-11-26 and 27): no newtype is special (W).
     `fields::Newtype<T, F>`; the newtype arm, `newtype_inner` and
     `FieldNode::wrapper` deleted; an unlisted one-field tuple struct
     refused before the struct arm (`refuse_unregistered_newtype`); a path
     into a listed type's fields refused (`field_named`, through
     `FormSpec::shape_at`); the last-resort panic names the path, the type
     and both ways out. 5.5.5 was done as part of 5.9.
  4. **5.9.1 to 5.9.5** (2026-11-26): `FieldFor<V>`, `FormSpec::with_types`,
     the list kept by `Items` and `VariantNode` for later walks, one lookup
     (`TypeRegistry::find`), the set-item key through the list,
     `formoxus-fixture-lib`, `types!` with `include` and `replace`,
     `BuiltIns`, the prelude's `FormoxusTypes`, and `form!` and `path!`
     reading it. `DefaultField` is gone. Questions QA to QJ answered
     2026-11-26, all as recommended.
  5. **5.6, revised** (2026-11-26): `form!` judges a listed type by its
     field's `KIND` through `TypeKinds::KINDS`, so the compile-time checks
     were never loosened. An unlisted struct stays `Other` and is refused;
     only an unlisted newtype is `Unknown`. `struct_node`'s assert now
     covers attributes as well as a widget (5.6.3.1).
  6. **5.9.6.1, 5.9.6.4** examples unchanged (their forms use built-ins
     only); `refuse_unregistered_newtype`'s message names `types!`.

  **Open, in order.**

  - [x] **5.9.3.2.4** (Todd 1 to 3, Claude 4 and 5) **Built-ins register
    through a hidden door, and `field_for` refuses them.** Done 2026-11-27.

    **Why:** under AF no list can give a built-in type (`String`, `bool`,
    the ten numbers) a field of its own. `types!` already refused that at
    compile time, but a hand-built `TypeRegistry` could still do it at run
    time, because `field_for` accepted any field. Closing that gap means
    `field_for` refuses built-ins, and every list's `registry()` then needs
    a way in that skips the refusal.

    1. **`is_built_in(shape) -> bool`**, `pub(crate)`, in
       `type_registry.rs`: true for exactly the twelve, false for `usize`,
       `isize`, `char` and `i128`. It is a second copy of the list in
       `types!`' `built_ins()`.
    2. **`field_for` refuses a built-in**, first, before the duplicate
       check, with a message saying it reads the same in every form and to
       declare a type of one's own (`struct Shouty(String);`).
    3. **`#[doc(hidden)] pub fn __built_in`**, `field_for` without the
       refusal. Both call the private `add_to_registry`, so the `FieldInfo`
       is built in one place. `pub` because the code `types!` writes runs
       in the dev's crate.
    4. **`types!` switched:** each list's `registry()` registers its
       built-ins with `__built_in` and everything else with `field_for`.
       Without this, every list would panic on first use.
    5. **Tests:** `every_built_in_is_one_is_built_in_accepts` and
       `is_built_in_refuses_what_is_not_built_in` (inline), and
       `type_lists::field_for_refuses_a_built_in_type` (suite).

  - [x] **5.9.3.9** (Todd) **The built-ins come through the lookup, and
    the scalar arm goes.** Done 2026-11-27. One test changed:
    `forms::an_unsupported_scalar_fails_loudly_at_construction` now expects
    the last-resort message for a `usize`.

    **Why:** every list now holds the built-ins, so the walk's separate
    scalar arm, and `scalar_node` behind it, are a second route to the same
    twelve types. Two routes can disagree, and the scalar arm is also why
    the comment on the constructor arm still talks about "replacing a
    built-in".

    1. **`find` falls back to the built-ins.** In `TypeRegistry::find`
       (`type_registry.rs:251`), the last step is `types.and_then(in_list)`
       with a `// TODO: BuiltIns`. Use the form's list, or
       `BuiltIns::registry()` when the form has none:
       `types.or_else(|| Some(BuiltIns::registry())).and_then(in_list)`.
       That is what a hand-built spec with no `with_types` gets (AD): the
       installed overrides, then the built-ins. Delete the TODO.
    2. **Delete the scalar arm** in `node_for_shape` (`build.rs:429`, the
       `shape.scalar_type()` branch with its "Until `BuiltIns` exists"
       comment), **and `scalar_node` itself** (`build.rs:541`). Remove any
       imports only they used; they show up as unused-import warnings.
    3. **No new panic is needed.** A `char`, `usize` or `i128` then finds
       nothing in `find` and falls through to the last-resort arm, whose
       5.5.7 message names the path and the type and offers a `types!` line
       or `field:`.
    4. **Check:** the whole suite stays green. It now builds every `String`
       and number through the list instead of the scalar arm, so any
       difference between `scalar_node` and the built-in fields in `types!`
       shows up as a failure.

  - [x] **5.9.6.3** (Claude, after 5.9.3.9) **The comments and a test that
    3.9 makes stale.** Done 2026-11-27, plus three more stale mentions: the
    `types!` built-ins doc, `roundtrip.rs`' module doc, and `is_newtype`'s doc,
    which cited the deleted `newtype_inner`. Item 3 is met by the `forms` test
    above, whose `usize` is the same case as a `char`.
    1. The comment on the constructor arm in `node_for_shape`, which says a
       field registered for a built-in type replaces it everywhere (untrue
       under AF).
    2. The two `field_kind.rs` comments that mention `scalar_node` (about
       lines 869 and 1271).
    3. A `char` case in
       `newtypes::a_type_formoxus_cannot_build_panics_naming_the_path_and_the_type`,
       which covers only an array today.

  - [x] **5.9.6.2** (Claude) Done 2026-11-27, with the registry's panic
    messages reworded in house style, and `widget_for`'s doc saying no form
    reads it yet (box 6). The docs carry the lookup order, so memory needs
    no entry of its own for it. **Rewrite `type_registry.rs`' module docs**
    around `types!`: what a list is, `FormoxusTypes`, `include` and
    `replace`, the lookup order, why built-ins never change (AF), and what
    `install` is now for. The same for `TypeRegistry`, `field_for` and
    `install`, whose docs still describe the process-wide registry as the
    only copy. Then memory (5.9.6.5): the probe file and an entry for the
    lookup order if the docs alone do not carry it.

  - [x] **5.6.3.2** **NOT NEEDED, found 2026-11-27.** A set of an unlisted
    struct is supported, not a mistake: each item is a group of fields,
    `SetNode` shows it as its fields joined, edits it through a draft that is
    a group of fields, and a custom `sort_by` rebuilds it
    (`sets::a_set_item_that_cannot_be_built_sorts_last_without_panicking`).
    What matters is that every field can be built, and the walk already
    refuses one that cannot when the form is built, because `set_node` always
    builds the set's draft, even for an empty set; its message names the
    field's own path (`type_lists::a_set_item_with_a_field_formoxus_cannot_build_is_refused_naming_the_field`).
    A check up front would repeat the walk's rules and name only the set.
    Todd built one, and it was taken out the same day. Also, this check was
    never for hand-built specs only: `form!` checks only the paths it names,
    so an unmentioned `HashSet<Spot>` reaches the build too. The old text
    follows. **A set over items that are not leaves, in a hand-built spec.** Through `form!` a set of
    unlisted structs is a compile error (U), but a hand-built spec can
    still ask for one. `set_node` builds each item through
    `node_for_shape`, so an unlisted struct item would become a fieldset
    inside a set, which the set's rendering and its `[= v]` keys do not
    expect. Assert once, before building the items, that the item shape
    is something a leaf is built from: a built-in, a unit enum, or a type
    the form's list or the overrides give a field.

  - [ ] **5.6.3.3** (Todd; Claude writes the messages) **A listed field's
    widget and attributes, in `LeafArgs::node`, for a hand-built spec.**
    After the widget is chosen, assert that it renders `F::KIND`
    (`widget.accepts().contains(F::KIND)`, mirroring `renders`, so
    `|| (TAKES_CHOICES && has_choices)` once Q is back), naming the path
    and `widget.name()`. Likewise each `spec.attrs` key must apply to
    `F::KIND`, the run-time twin of `applies`. Check only the leaves whose
    field came from a list or the overrides (AL): the walk knows, because
    `TypeRegistry::field_in` returned the field. Through `form!` this never
    fires, since the compiler already judged the same list; built-in leaves
    in a hand-built spec stay unchecked on purpose (V).

  - [ ] **5.7** (Todd, then Claude) **`choices:` on a listed type (Q),
    REVISED 2026-11-27 for AB; no longer deferred** (Todd put it after
    5.6.3.3 and before box 6). `P` is the type of the path that carries
    `choices:`, the value being chosen: in `questions[] => { choices:
    COURSE_QUESTIONS }` over `questions: Vec<Ref<Question>>`, `P` is
    `Ref<Question>`.

    **Why it does not work today.** A `<select>`'s options submit text, so
    `ChoiceField<T>` writes each choice with `format` and reads it back with
    `parse`, both its defaults, which know only a string's, a number's and
    a bool's text; and its `KIND` is `field_type_of(T::SHAPE)`, which
    panics at compile time on a struct ("a choice field's value must be
    text, a number, or a bool"). Only `P`'s listed field knows that a `Ref`
    is written `question:q1`.

    **Why the old plan is obsolete.** Q (2026-11-25) had the registry hand
    out a `FieldText<P>` behind an `Arc<dyn Any>`, downcast at build,
    because `form!` could not see a registry filled at run time. Under AB,
    `choice_node` already receives the list as `Reg`, so `P`'s field is
    known at compile time: `<Reg as FieldFor<P>>::Field`. No `Any`, no
    downcast, no lookup.

    1. **(Todd) `ChoiceField` holds the item's field.** Give it the field
       for `P` (a second type parameter, `ChoiceField<T, F>`, or `F` alone,
       with `T = F::Value`), and take `format`, `parse`, `trims` and `KIND`
       from it, so a `ChoiceField<u8>` and a `ChoiceField<Ref<Question>>`
       work the same way. `check` stays membership. `choice_node` builds it
       from `Reg::build(Default::default())`. Keep it one type, not a second
       choice field beside the first (one function, not two).
    2. **(Todd) Choices written as text.** apcsp's tables are `const`
       `(&str, &str)` pairs, and a `Ref<Question>` has no `const` value, so a
       choice's value may be written as `P`'s text and parsed through `P`'s
       field once, when the field is built, panicking with the choice's
       label if it does not parse. `ChoiceValue` today turns `&str` into a
       `String` only; it needs a way to parse through the field instead,
       which means it is no longer a plain conversion. Real `P` values still
       work.
    3. **(Claude)** `form!`'s side, if any: the `choices:` checks
       (`is_single_value` with `has_choices`, `renders`) already judge a
       listed type by its `KIND`. Bring the three tests in
       `custom_fields.rs`' `#[cfg(any())] mod deferred_q` back (they write
       choices as text, so 2 is what they test), plus one with real `P`
       values and one for a choice text that does not parse; the
       `choices:` docs.

- [ ] **6.** (Todd) The per-type widget, chosen at build (P). Rewritten
      2026-11-25 by Claude; the render-time version (6.1 to 6.5 of
      2026-11-24, in `a8463e0`) is superseded with L. **AB (10.7) makes the
      per-type widget an entry in `types!` once built**; the lookup order is
      AB.6's.

  - [ ] **6.1** **FOLDED INTO box 6's `types!` widget entries (AM).** When
    `types!` takes a widget for an entry, it writes a free `const _`
    beside it asserting that the widget's `ACCEPTS` contains the entry's
    field's `KIND`, and box 6 decides whether a built-in type can have a
    widget entry (AF). The text below is the superseded version.
    `widget_for` itself moved to 5.3.4 on 2026-11-26,
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

    AM. **DECIDED 2026-11-26 (Todd): as recommended, 6.1 folds into box 6.**
        Is 6.1 still a step? It is stale
        twice over: a user struct is `Other` to `kind`, not `Unknown`, and
        AB.10.7 makes the per-type widget an entry in `types!` rather than a
        `widget_for` call. In `types!` the check is a free `const _` the
        macro writes beside the entry, `W::ACCEPTS.contains(F::KIND)`, which
        fires under `cargo check` too, with no shape walk, and covers a
        user type exactly, because it asks the entry's own field. A widget
        for a BUILT-IN type is AF's question (can a list change how a
        `bool` renders?), and box 6 should settle it. Recommended: fold 6.1
        into box 6's `types!` widget entries, with that check, and decide
        the built-in case there.

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

      **Updated 2026-11-26 for AB:** "a `TypeRegistry` installed in `main`"
      is now a `types!` list in apcsp (`TimeLimit => field TimeLimitField`,
      `Markdown => newtype String`, `ChoiceId => …`), with `use AppTypes as
      FormoxusTypes;` where its forms are, and `install()` only if it has
      `replace` lines. `MarkdownWidget` waits on box 6's widget entries.

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

- [ ] **12.** (Todd and Claude) **What "renderable" means, nailed down
      (AP, AQ, AR).** Added 2026-11-27. **Order (Todd, 2026-11-27): 5.6.3.3,
      then 5.7, then box 6, then this box, then 7 to 9.** A type at a path is renderable when
      every part of it reduces to form controls formoxus can render, or
      combines into lists and sets of them. This box settles the three places
      the rule was uneven; once it lands, Claude writes the definition into
      CLAUDE.md's vocabulary and memory (G, 2026-11-27: not before). In AP,
      AR, AQ order: the smallest first, and AQ's browser test last.

  - [ ] **12.1** (Claude) Tests first for 12.2 and 12.3, failing until they
    are built.

  - [ ] **12.2** (Todd; Claude writes the messages and docs) **No tuple
    struct is a group (AP).**
    1. **Widen `refuse_unregistered_newtype`** (`build.rs:465`) to every
       tuple struct no list holds (`StructKind::TupleStruct`, any arity),
       and refuse every plain tuple (`StructKind::Tuple`), listed or not
       (AP.1). Drop the one-field and over-a-scalar conditions, and probably
       the name with them (`refuse_tuple`). It runs before the struct arm,
       after the list lookup, so a listed tuple struct still builds through
       its entry. A unit struct (`struct Marker;`) is not a tuple and is
       left alone.
    2. **Refuse a path into one,** in `field_named` (`build.rs`, the
       5.5.6 guard): `with_widget(&path!(M.pair.0), …)` names a part that
       will never be built, so it is refused like a path into a listed
       type's fields, rather than silently lost.
    3. **Tuple variants (AP.2, AP.3):** a variant's fields are built one by
       one (`fields_from_enum`, `node_for_shape` per field), never through
       `struct_node`, so the widened refusal does not reach them. Add a
       refusal where a variant's fields are built: a tuple variant with more
       than one field. A one-field tuple variant builds as today.
    4. **Claude:** the messages (a tuple struct: a `newtype` line for one
       field around a type with a field, `field YourField` for any; a plain
       tuple: declare a tuple struct and list that; a tuple variant with
       several fields: a struct variant, or one listed tuple struct as its
       payload); `types!` refusing a plain tuple entry, with a golden;
       `newtypes::a_newtype_around_a_non_scalar_is_still_a_struct` flips to a
       refusal, and so does `submissions.rs`' `Pen::Rgb(u8, u8, u8)` round
       trip ("Tuple variants, which build exactly as struct variants do");
       new tests for `Pair(u8, u8)`, a plain `(u8, u8)` field, a path into a
       tuple struct's part, a listed tuple struct of several fields, and a
       one-field tuple variant; the `type_registry.rs` module docs' "a type
       no list holds" paragraph, with aliases, and
       `refuse_unregistered_newtype`'s doc.

  - [ ] **12.3** (Todd for the recording; Claude for `path!`, tests and
    docs) **`[i]` on a set (AR).**
    1. **Where:** `FormState::validate` and `Submission::accept`, after the
       model is built and before the validator runs, since that instance is
       the one the validator iterates. Every set counts, nested ones
       included (a set inside a list item, inside a variant).
    2. **What:** for each set, walk the built instance's elements in their
       iteration order and find, for each, the item node holding the same
       value, then record the item keys in that order, keyed by the set's
       path. Matching by value is what `CustomOrder::position` already does,
       but typed; here the item type is known only as a `Shape`, so the
       item's value has to be rebuilt untyped (check facet-reflect 0.46.5
       for an `alloc_shape`) and compared as a `Peek`. A set of leaves can
       compare `peek_value()` directly, as `DefaultOrder` does.
    3. **Then `resolve` is not changed:** the `entry` closure at
       `state.rs:605` returns the recorded keys for a set's path in place of
       the shown order, and every other entry as before.
    4. **Claude:** `path!` stops refusing `[i]` on a set (its doc's "Only on
       a `Vec`"); tests over a `BTreeSet` and a `HashSet` of structs, with
       the error landing on the right item in the browser-side `validate`
       and in `Submission::accept`; the trap in AR, in `path!`'s doc.

  - [ ] **12.4** (Todd for `shown`; Claude for the rest) **Sets of lists and
    sets (AQ).**
    1. **Todd:** a `ListNode` and a `SetNode` leave their own item keys out
       of what they show. The keys are a `RawValue::Structure` entry at the
       node's own path; a variant's name is a `Structure` entry too and must
       still show, so drop only the node's own keys, not every `Structure`.
    2. **Claude:** `field_kind::is_list_of_set_items` takes lists and sets
       as items (still not an `Option`); `form!`'s and `treat_as_set`'s
       messages; `form_set_on_a_list_of_lists` becomes the missing-`sort_by`
       error; suite tests (a set of `Vec<String>` shows "a, c", dedups by
       value, round-trips; a set of sets); an e2e test form for a set of
       lists, with its test, added in a pair.
