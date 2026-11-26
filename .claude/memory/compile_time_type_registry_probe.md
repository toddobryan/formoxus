---
name: compile-time-type-registry-probe
description: "DECIDED 2026-11-26 as CUSTOM_FIELDS_PLAN decision AB (Todd), not built. PROVEN in scratch probes: a compile-time type registry works through `impl FieldFor<ModelType> for AppTypes` (local marker type satisfies the orphan rule); one `types!` list yields compile-time impls AND the run-time TypeRegistry; all checks fire under cargo check. Not decided."
metadata:
  type: project
---

**DECIDED 2026-11-26: CUSTOM_FIELDS_PLAN decision AB** — per-crate `types!`
lists, libraries by replay, each form carries its list, the app's list
installed as OVERRIDES (order: path's own > installed > form's list >
built-ins), an override never reaches a configured path, a `replace` with a
different KIND from the library's is refused at compile time. Open: WHEN
(AB.12, mid-5.5 or after). The plan holds the design; this file holds the
probe evidence.

Todd asked (2026-11-26, during CUSTOM_FIELDS_PLAN 5.5.5) whether the
`TypeRegistry` could exist at compile time. Probe: three crates in the
session scratchpad (`tt-probe`: plain `models`, `fxt` standing in for
formoxus, `app`), against a working-tree copy of formoxus.

**What works (all verified under `cargo check`, not just build):**

1. `pub trait FieldFor<V> { type Settings: Default; type Field: Field; fn build(Settings) -> Field }`
   in formoxus, implemented by the APP for its own unit struct:
   `impl FieldFor<models::Markdown> for AppTypes`. Legal although the app owns
   neither the trait nor the model: the orphan rule needs one local type, and
   `AppTypes` is T0. Models stay plain (no formoxus dependency).
2. A `types! { pub struct AppTypes; Markdown => newtype String, Minutes => field MinutesField }`
   macro writes the struct, one impl per entry, the built-ins, an
   `impl<T> FieldFor<Option<T>> for AppTypes where AppTypes: FieldFor<T>`,
   and `AppTypes::install()` building the run-time `TypeRegistry` from the
   same list. Run-time walk then finds an unnamed `Vec<Markdown>`.
3. `Settings` is what makes newtype settings work: for a newtype entry it is
   the INNER field (`StringField`), `build` wraps it in `Newtype`. So
   `bio => { max_length: 280 }` on a registered newtype is typed at compile
   time with no `newtype` key (Y). `Option<Markdown>` gets the same.
4. Errors: unregistered type -> E0277 with `on_unimplemented` ("`Headline`
   has no field in `AppTypes`", lists what IS registered); a setting the
   field lacks -> E0609; a KIND check via `const fn kind_of<M, P, R: FieldFor<P>>(fn(&M)->&P)`
   in a free `const _` -> E0080 with our message.

**What does NOT work:** a blanket `impl<V: DefaultField> FieldFor<V> for AppTypes`
beside a specific entry: E0119, "upstream crates may add a new impl of
`DefaultField` for `Markdown`". So the macro must list built-ins itself.

**Libraries, probed the same day (Todd: "if we have a story for `types!`
and `form!` in a library, we have something"):**

1. `form!` finds its list by NAME RESOLUTION, not by the macro knowing
   anything: it writes a bare `FormTypes`, resolved at the call site. The app
   writes `use AppTypes as FormTypes;`; the library writes
   `type FormTypes = crate::LibTypes;` (a type alias, because a `use`
   imports the macro of the same name too, and a crate may not reach its own
   macro-expanded `#[macro_export]` by an absolute path:
   `macro_expanded_macro_exports_accessed_by_absolute_paths`, deny-by-default).
2. Including works by REPLAY: a library's `types!` also emits
   `#[macro_export] macro_rules! LibTypes` (same name as the struct, other
   namespace, so one `use libt::LibTypes;` brings both) that calls
   `::formoxus::types!` back with its entries `$crate::`-qualified ahead of
   the includer's. The app writes `include LibTypes;` and gets real per-type
   impls plus one run-time registry covering both lists.
3. TRAP: that `$crate` must come from a PROC macro. Emitted from
   `macro_rules`, it names the crate the tokens were written in (fxt), even
   with the `$` passed in from the library: E0425 "cannot find `Markdown` in
   `$crate`". A proc macro's parsed output carries the call-site span, and
   `$crate` names the library. formoxus-macros would emit it, so fine.
4. Including by DELEGATION (`impl<T> FieldFor<T> for App where Lib: FieldFor<T>`)
   fails as soon as the app lists a model type from a third crate: E0119,
   "upstream crates may add a new impl of `FieldFor<Minutes>` for `LibTypes`".
   The common case, so replay it is.
5. Listing a type the included list has: E0119 on that type. Where `replace`
   would apply.
6. Requirement found: a library's entries must be reachable from its root
   (`pub use models::Markdown;`), since the replay writes `$crate::Markdown`.

7. SEVERAL libraries work: `include LibTypes; include NewsTypes;` chains
   through an accumulator (`types! { @acc [entries so far] … }`); each
   replay macro appends its entries and calls back. Each library's forms
   keep their own list. Probed with two libraries.
8. NOT probed, reasoned: a DIAMOND (a library whose list includes another,
   and an app including both) replays the shared entries twice -> E0119.
   Fix idea: tag each replayed list with its crate and name (the proc macro
   reads `CARGO_CRATE_NAME` when writing the replay) and skip a tag seen
   twice. Dedup by type TEXT is wrong: `$crate::Markdown` reads the same
   from two crates.
9. PROBED the same day, WORKS: since `form!` knows its `FormTypes`, a form can carry
   its list's run-time registry (a static per list) in its `FormSpec`,
   instead of a process-wide `install`. Then nothing needs installing,
   a library's forms can never meet an app that forgot to include them, and
   client and server agree by construction. `path!(…[= v])` would need its
   list too (take `FormTypes` from scope, like `form!`) — that part NOT probed.
   How, in the scratch copy of formoxus: `FormSpec.types: Option<&'static
   TypeRegistry>` + `with_types`; `Walk.types`; the walk's registry arm asks
   `walk.types` first, else the global; `Items` and `VariantNode` keep
   `walk.types` because they walk again at run time (`distribute_values`
   on the server, Add, choosing a variant). `types!` writes
   `AppTypes::registry()`, a `LazyLock` static. Verified with NO `install`
   in the process: two lists giving `Markdown` different fields in one
   process each got their own; a server-style rebuild (`empty_form` +
   `distribute_values`) built a `Vec<Markdown>` item through the kept list
   and validated; a library's form over a type the app never lists worked;
   control: the same form with no list panics naming `Markdown`.

**Open, untested:** how `form!` learns which `AppTypes` (a key per form, or
a fixed path like `crate::FormTypes`); replacing a built-in (macro_rules
cannot omit a listed built-in; a proc macro matching type text is fragile
with aliases, so an explicit `replace String => …` form is likelier);
(libraries: see above).

Related: [[transparent-not-followed]], [[const-shape-walk-blocked]],
[[models-stay-plain]].
