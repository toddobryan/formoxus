# Read-only stores (#22) and a bound field (#23)

Written 2026-11-18 by Claude, after Todd asked for #22 and #23 next on the road to
publish (`.claude/memory/mvp_scope.md`): this, then A1 (the `form!` grammar docs),
then the doc pass. Both issues reshape the same public types, `WidgetProps`,
`AddItemProps`, `SetItemProps`, `FormWide` and `Form`'s getters, and both are
breaking, which is why they are one plan, and why it lands before publishing.

Who does what: Todd writes the runtime; Claude writes the tests (first, with no
implementation behind them), the docs, the memory, and the suite and examples
follow-ups. No macro work: `form!` names no store.

**One radical change on a branch `bound`**, with the compiler errors as the to-do
list ([[feedback-radical-refactors-on-a-branch]]). apcsp is left alone until Todd
opens it; it already fails to compile against `main` (its `crates/ui` widget predates
the 2026-11-11 `Widget`/`LeafWidget` split), and this will add to that.

## Why

1. **#22: a dev can write the value store and skip the live check.** Since
   `ERRORS_PLAN.md` 6c, a widget writes a value only through `FieldHandle`, which
   tells the form after every write. That is what live validation runs on. But the
   store itself is still handed out, writable, in three places a dev reaches
   (`Form::values()`, `AddItemProps.values`, `SetItemProps.values`), and
   `values.insert(path, raw)` compiles in all three. A write the form never hears
   about leaves the field unchecked until submit, and can leave `aria-invalid="false"`
   claiming a value nobody checked. The error store has the same hole, in more
   places (listed below), and there a stray write shows an error no check made.
2. **#22: reading is clumsy too.** With the store's missing-key rule (an absent key
   reads as `""`) crate-private, a dev's adder or item view reads another path with
   `values.read().get(…).cloned().unwrap_or_default()` by hand. Two suite tests do
   exactly that, and say so (`ProbeAdder` in `tests/suite/vecs.rs`, `TagChip` in
   `tests/suite/sets.rs`). And `values.read()` subscribes the reader to the WHOLE map,
   which is the wrong default for reading one key.
3. **#23: one field's live state is spread across loose values.** A widget gets
   `id`, `path`, `label`, `help_text`, the error store and the classes in
   `WidgetCommon`; the value store, the file table and the event callback in
   crate-private fields of `WidgetProps`; and a `FieldHandle` built from them on
   request. Reading one field's errors takes a path plus the store, every time.
   Django calls the thing that holds all of this a `BoundField`; formoxus has its
   parts and no name for the whole.

## Where the stores reach a dev today

| Where | What it hands out | Notes |
|---|---|---|
| `Form::values()` (`form.rs:407`) | `ValuesStore` | |
| `use_form_values` (`form.rs:232`) | a new `ValuesStore` | in the prelude; **no caller anywhere**, `use_form` does this itself |
| `AddItemProps.values` (`widgets/item_buttons.rs:44`) | the whole form's `ValuesStore` | a dev's adder (`adder:`) |
| `SetItemProps.values` (`widgets/set_item.rs:35`) | the whole form's `ValuesStore` | a dev's item view (`item:`) |
| `WidgetProps::new(…, values, errors, …)` (`widgets/widget_props.rs`) | takes both stores | for testing a widget alone |
| `Form::errors()` (`form.rs:411`) | `ErrorsStore` | |
| `WidgetCommon.errors` (`widgets/widget_props.rs:36`) | the whole form's `ErrorsStore`, `pub` | every widget, leaf and set |
| `FormWide.errors` (`nodes.rs:478`) | the same, `pub` | reached through `NodeContext`, which a custom `Node` gets |
| `FieldErrors`'s `errors` prop (`widgets/errors.rs:45`) | takes the store | a public component |
| `widgets::*` (`widgets.rs:144`) | re-exports both aliases | the custom-widget kit |

`FormWide.contents` (values and files) is already `pub(crate)`, so a custom `Node`
cannot write values today; only its errors leak.

## The design

**Part 1 (#22): two read-only views, and the stores go crate-private.** The aliases
`ValuesStore` and `ErrorsStore` become `pub(crate)`, and every place in the table
hands out a view instead. A view is `Copy`, wraps the store in a private field, and
has read methods only. The crate reaches the store inside through a `pub(crate)`
accessor, so the internal code changes little.

The values view:

1. **`get(&NodePath) -> String`**: one key, subscribing to that key only, with the
   missing-key rule built in. What `FieldHandle::current` does now, so that can
   call it.
2. **`peek(&NodePath) -> String`**: the same without subscribing, for an event
   handler (decision B).
3. **`paths_under(&NodePath) -> Vec<NodePath>`**: every key under a prefix, for an
   adder that reads its whole list (`ProbeAdder` counts entries; an adder leaving out
   the choices already chosen reads them). It reads the map, so it subscribes to the
   map's keys. Measured in 6c (`FieldHandle::write_value`'s doc): only an `insert`
   (a new key) wakes a whole-map reader, a `set` on an existing key does not, so this
   re-renders when an item is added or removed, not on every keystroke.

The errors view: **`at(&NodePath) -> Option<Vec<ValidationMessage>>`** and
**`validity(&NodePath) -> Validity`**, which are `ErrorsStoreExt`'s two methods. A
newtype can have inherent methods, which the alias couldn't (the orphan rule), so
the extension trait goes.

**Part 2 (#23): `FieldHandle` becomes the bound field.** The shape recommended in
decision D:

1. `WidgetCommon` stays what every widget, leaf or set, shares: `id`, `path`,
   `label`, `help_text`, `classes`, and now the errors VIEW. Its `validity()`,
   `is_invalid()` and `part_class()` stay where they are.
2. `FieldHandle` is renamed `BoundField` and holds a `WidgetCommon` plus what is
   crate-private today: `contents` (values, files, resets) and `on_field_event`.
   It keeps every method `FieldHandle` has (`current`, `input`, `change`, `blur`,
   the `on_*` builders, `pick_file`, `picked_file`, `resets`), and derefs to
   `WidgetCommon`, so it also answers `errors()`, `validity()` and `is_invalid()`
   for its own key, without being handed a path.
3. `WidgetProps` holds a `BoundField` instead of `common`, `contents` and
   `on_field_event`, plus what is about rendering, not the field's state:
   `base_attrs`, `extra_attrs`, `choices`. It derefs to the `BoundField`, so
   `props.on_input()` and `props.is_invalid()` both work, and a widget that renders
   another passes the props on whole, as now. `field_handle()` goes, or becomes
   `field()` returning a clone (decision E).
4. `SetWidgetProps` keeps a plain `WidgetCommon`. A set's widget has no value to
   write (it sends `Edit`s), so it gets no `BoundField`, and `current()` or `input()`
   at a set's path, which would write a raw string over its structure entry, cannot
   be called.

Containers (a struct, list, set or variant node, with errors and no value) keep
reading through `NodeContext`, now as `ctx.errors.at(path)` on the view: #23's open
question 1, answered "no new type" (decision F).

## Decisions (Todd)

- **A. A newtype, or dioxus's `ReadStore`?** dioxus-stores 0.7.10 has a read-only
  store, `ReadStore<T>` (`store.rs:25`), made from a `Store` with `From`. Handing that
  out closes the write hole with no new type. **Claude recommends the newtype**:
  (1) it keeps dioxus-stores out of formoxus's public API, so a dioxus-stores
  change can't break a dev, and the `insert`-versus-`set` subtlety stays internal,
  as with the file table (`FilesSignal` is crate-private "so that choice can
  change"); (2) it gets the missing-key rule and `paths_under`, which `ReadStore`
  would leave every dev to write; (3) inherent methods replace `ErrorsStoreExt`.
  `ReadStore`'s argument is less code and a type devs may already know.
- **B. Names.** Claude recommends `ValuesView` and `ErrorsView` (they say read-only,
  and the crate-private stores keep their names, so the internal diff stays
  small). And whether the values view has `peek`: recommended yes, since an adder's
  `onclick` reading a value should not subscribe anything (the trap `changed_paths`
  documents).
- **C. Delete `use_form_values`?** Recommended yes: it has no caller, is in the
  prelude, and hands out a store that nothing connects to a form.
- **D. The bound field's shape and name.** Recommended as above: rename
  `FieldHandle` to `BoundField`, holding `WidgetCommon`, and held by `WidgetProps`.
  The alternatives: (1) keep the name `FieldHandle` (already in every built-in
  widget and the docs; `BoundField` matches Django and reads better in docs: "a
  widget is handed a bound field"); (2) do only Part 1 and close #23 as "the
  `WidgetCommon` and the handle together are the bound field", documented, which is
  the least churn and leaves the split #23 complains about.
- **E. `props.field_handle()`.** With `WidgetProps` dereffing to the bound field,
  a widget calls `props.on_input()` directly. A widget still needs an owned
  `BoundField` to move into a closure of its own; recommended `props.field()`, a
  clone, and `field_handle()` goes.
- **F. Containers.** Recommended: no bound type for them now (`ctx.errors.at(path)`
  is short), revisited only if a custom container node becomes a thing devs write.
- **G. `WidgetProps::new` for a widget tested alone.** It takes the stores now,
  which a dev can no longer name. Recommended: it takes the seed maps
  (`HashMap<NodePath, String>` and the errors map) and makes the stores itself with
  `Store::new`, as it already makes its file table. A test then sees a write
  through the props (`props.current()`), not by reading a store it made.
  `PasswordWithEcho` (`tests/suite/widgets.rs:556`) is the one test that reads a
  write that way, and moves to that.
- **H. Re-exports.** `widgets::*` swaps the two store aliases for the two views.
  That is a swap, not a new re-export, so the "no new re-exports" rule
  ([[prelude-is-the-one-list]]) holds. `BoundField` replaces `FieldHandle` in the
  kit the same way.

## Traps

1. **`get` must read one key, never the map.** `self.store.read().get(path)` would
   subscribe the reader to every path, so one keystroke would wake every reader.
   `get_unchecked(path).try_read()`, as `FieldHandle::current` does now. A missing
   key still wakes its reader when `insert` first writes it.
2. **`BoundField` clones into every handler closure.** `FieldHandle` cloned a
   `NodePath` and two `Copy` values; a `BoundField` also clones `id`, `label`,
   `help_text` and an `Rc<Classes>`. Cheap, but per handler per render: if it ever
   shows, the closures can capture a slimmer private handle.
3. **`PartialEq` on props is what stops a re-render.** The views and `BoundField`
   must derive it, and compare the stores by identity, as `Store` already does.
   A hand-written `PartialEq` that compared contents would re-render every field
   whenever any value changed.
4. **`VariantNode` passes attributes with the error attributes already applied**
   (`WidgetProps.base_attrs`'s doc). Moving `common` into the bound field must keep
   that exception working; `enums.rs`'s `aria-describedby` tests catch it.
5. **The suite reads the whole store in one place that is not a dev's adder or
   view:** `with_store_keys` (`tests/suite.rs:142`, the keys a structural edit
   leaves behind). It moves to `paths_under(&NodePath::root())`. Every other
   `.values()` in the suite is a `WireForm`'s or a `FormEvent`'s, and is untouched.

## Boxes

- [ ] **1.** (Todd) Decide A to H.

- [ ] **2.** (Claude) Tests first, on branch `bound`, with no implementation:

  1. The values view: an absent key reads `""`; `get` subscribes one key (a
     keystroke in one field re-renders no reader of another, counted with the
     `Harness`); `paths_under` returns the keys under a list and wakes on an added
     item but not on a keystroke; `peek` subscribes nothing.
  2. The errors view: `at` and `validity`, the three states, one-key subscription.
  3. `ProbeAdder` and `TagChip` rewritten to read through the views, with their
     "as a dev's adder has to" comments gone.
  4. The bound field: a widget built only from `props.current()`, `props.on_input()`,
     `props.on_blur()` and `props.is_invalid()` is live-checked (the live-check tests
     already in `live_checks.rs`, run against such a widget).
  5. A trybuild golden: `form.values().insert(…)` does not compile. The one place
     "read-only" is pinned rather than just true.

- [ ] **3.** (Todd) Part 1: `ValuesView` and `ErrorsView` in `nodes.rs`; the aliases
  `pub(crate)`; `ErrorsStoreExt` becomes the view's inherent methods; every row of the
  table above takes or returns a view; `use_form_values` deleted (C);
  `WidgetProps::new` takes maps (G).

- [ ] **4.** (Todd) Part 2: `FieldHandle` → `BoundField` holding `WidgetCommon`;
  `WidgetProps` holds it and derefs to it; the built-in widgets move from
  `props.field_handle()` to the props or `props.field()` (E).

- [ ] **5.** (Claude) The suite, the examples and the e2e crate compile and pass; the
  docs (each view, `BoundField`, `WidgetProps`, the custom-widget walkthrough in
  `widgets.rs`); memory (a `bound_field` file, the renames line in `MEMORY.md`); a
  note for apcsp's catch-up when Todd opens it.

- [ ] **6.** (Todd) Merge `bound`; close #22 and #23.
