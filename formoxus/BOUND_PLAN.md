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
hands out a view instead (the `values` fields of `AddItemProps` and `SetItemProps`
go altogether, replaced by their bound field's `values()`, Part 2). A view is `Copy`, wraps the store in a private field, and
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

**Part 2 (#23): one `BoundField<K>` for every kind of node** (decisions D, E, F).
`FieldHandle` is renamed `BoundField` and gains a kind parameter, `BoundField<K = Leaf>`,
where `K` is a zero-sized marker naming what kind of node the path is.

**Why one type can serve every kind.** Every non-leaf widget already writes the same
way: it sends an `Edit` addressed to its own path. The variant select sends
`ChooseVariant`, a set widget `AddItem` and `RemoveItem`, an adder `AddItem`, and
Remove, Edit and ↑/↓ the rest. So a node's write is either a value at its path (a
leaf) or an edit at its path (everything else), and every kind of node has a path,
an id and errors. What differs is only which writes make sense.

1. **The members are the same for every kind.** `path`, `id`, `label`, `help_text`,
   `classes` and the `ErrorsView`, which is what `WidgetCommon` holds today, plus
   the form's three handles: `contents` (values, files, resets), `on_field_event`
   and `on_edit`. The handles are all `Copy`, so carrying one on a kind that never
   uses it costs nothing. A `PhantomData<K>` is the only thing that tells the kinds
   apart. `WidgetCommon` goes: its fields are these.
2. **Methods common to every kind**, in one `impl<K> BoundField<K>`: `path()`,
   `id()`, `label()`, `help_text()`, `classes()`, `errors()`, `validity()`,
   `is_invalid()` and `part_class::<W>()`, all for the node's own path; and
   `values() -> ValuesView`, the whole form's values, read-only, for a widget that
   shows more than its own value. That last one replaces the `values` fields of
   `AddItemProps` and `SetItemProps`: `ProbeAdder` becomes
   `field.values().paths_under(field.path())`.
3. **Methods per kind**, each in its own `impl BoundField<Kind>`. The edit methods
   fill in the node's own path, so a dev's adder no longer writes
   `Edit::AddItem { path: path.clone(), … }` by hand, and cannot address an edit to
   the wrong node.

   | Kind | Used by | Methods beyond the common ones |
   |---|---|---|
   | `Leaf` (the default) | `WidgetProps` | `current`, `input`, `change`, `blur` and their `on_*` builders, `pick_file`, `on_pick_file`, `picked_file`, `resets`: everything `FieldHandle` has now |
   | `Variant` | `VariantSelect` | `choose(Option<&str>)` |
   | `List` | a list's Remove and ↑/↓ | `add`, `remove(index)`, `move_item(from, to)` |
   | `Set` | `SetWidgetProps`, a set's Remove and Edit | `add`, `remove(index)`, `edit_item(Option<index>)`. No move: a set is sorted. |
   | `Items` | `AddItemProps` | `add`, `remove(index)`. One `ItemAdder` serves lists and sets alike, so its field needs a kind both share; `From<BoundField<List>>` and `From<BoundField<Set>>` make one. |
   | `SetItem` | `SetItemProps` | `current()`, read-only. Not `Leaf`: writing a set's item directly would skip the set's duplicate check. |
   | `Struct` | the internal nodes, for now | none |

4. **What stays on the props**: what the node works out on each render, which is
   not the field's live state. `choices`, `chosen`, `selected`, `item_count`,
   `max_items`, `required`, `draft`, `editing`, a set item's `text` and `index`, and
   the attributes.
5. **Who gets which:**

   1. `WidgetProps` becomes `WidgetProps<K = Leaf>`: a `BoundField<K>` plus
      `base_attrs`, `extra_attrs` and `choices`, dereffing to the bound field, so
      `props.on_input()` and `props.is_invalid()` both work, and a widget that
      renders another passes the props on whole, as now. `props.field()` returns a
      clone, for a closure to own; `field_handle()` goes (E). `LeafWidget::render`
      takes `WidgetProps`, that is `WidgetProps<Leaf>`.
   2. `VariantSelect` takes `WidgetProps<Variant>` and loses its separate `on_edit`
      prop. Today it is handed a leaf's `WidgetProps`, so under the old plan it
      would have had `input()`, which writes a raw string over the enum's structure
      entry. This is the case that decided F.
   3. `SetWidgetProps` holds a `BoundField<Set>` in place of `common` and `on_edit`.
   4. `AddItemProps` holds a `BoundField<Items>` in place of `path`, `id`, `values`,
      `on_edit` and `classes`.
   5. `SetItemProps` holds a `BoundField<SetItem>` in place of `path`, `values` and
      `classes`.
   6. The item buttons, `RemoveItemButton`, `EditItemButton` and `MoveItemButtons`,
      public components that take a path and an `on_edit` today, take a bound field
      of the kind their edits need: `Items`, `Set` and `List`.

6. **The container nodes** (struct, list, variant) render themselves inside the
   crate, reading `ctx.errors.at(path)`. They may switch to a `BoundField<Struct>`
   (and `List`, `Variant`) built from the `NodeContext` now or later: it is internal
   and breaks nobody, so box 4 does not require it.
7. **Not one plain `BoundField` with every method on it**, because the wrong call
   would then compile: `input()` on the variant select, `move_item` on a set. With
   the kind parameter each is a compile error.

## Decisions

**A to H decided 2026-11-19 (Todd)**, all as recommended except F, which went
further:

- **A.** Newtypes, not dioxus-stores' `ReadStore<T>` (`store.rs:25`): they keep
  dioxus-stores out of the public API, build in the missing-key rule and
  `paths_under`, and allow inherent methods in place of `ErrorsStoreExt`.
- **B.** `ValuesView` and `ErrorsView`; the values view has `peek`, so an event
  handler reads without subscribing.
- **C.** `use_form_values` is deleted.
- **D.** `FieldHandle` is renamed `BoundField`.
- **E.** `props.field()` returns a clone; `field_handle()` goes.
- **F.** One `BoundField<K = Leaf>` for every kind of node, as in Part 2 (Todd asked
  whether it could be "BoundField all the way down"; Claude's answer was this
  design). The public props all take it now; the internal container nodes may
  follow when convenient.
- **G.** `WidgetProps::new` takes the seed maps and makes the stores itself with
  `Store::new`, as it already makes its file table. `PasswordWithEcho`
  (`tests/suite/widgets.rs:556`) then sees its write through `props.current()`.
- **H.** `widgets::*` swaps the two store aliases for the two views, and
  `FieldHandle` for `BoundField`: a swap, not a new re-export
  ([[prelude-is-the-one-list]]). The seven kind markers are `pub` where they are
  defined and re-exported nowhere.

**Raised by F, decided 2026-11-19 (Todd: "probably right"):**

- **I. Private fields with getters.** `WidgetCommon`'s fields are `pub` today
  (`WidgetProps::new`'s doc: "a caller sets them afterwards"), while `FieldHandle`'s
  are private. On a bound field, a `pub path` could be changed after it is built,
  and every write would then go to the wrong node. So the fields are private, read
  through the getters in Part 2 item 2, and `WidgetProps` gets `with_label` and
  `with_help_text` builders for the one caller that sets them, a widget tested
  alone.

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
   (`WidgetProps.base_attrs`'s doc). Moving to `WidgetProps<Variant>` must keep that
   exception working; `enums.rs`'s `aria-describedby` tests catch it.
5. **The suite reads the whole store in one place that is not a dev's adder or
   view:** `with_store_keys` (`tests/suite.rs:142`, the keys a structural edit
   leaves behind). It moves to `paths_under(&NodePath::root())`. Every other
   `.values()` in the suite is a `WireForm`'s or a `FormEvent`'s, and is untouched.
6. **`#[derive]` on a generic type bounds its parameter.** `#[derive(Clone, Debug,
   PartialEq)]` on `BoundField<K>` adds `K: Clone + Debug + PartialEq`, the spurious
   bound `form.rs` hand-writes `Copy` for `Form` to avoid. Either every marker
   derives `Clone, Copy, Debug, PartialEq, Eq` (they are unit structs, so that is
   free), or the impls are written out. The first is less code.
7. **Only the crate builds a bound field.** Its constructor is `pub(crate)`, called
   by the node that knows its own kind, so a dev can never hold a `BoundField<Set>`
   for a leaf's path. `WidgetProps::new` (G) is the one public way in, and it makes
   a `Leaf`.
8. **The conversions go one way.** `From<BoundField<List>>` and
   `From<BoundField<Set>>` for `BoundField<Items>`, and nothing back, and nothing
   between a leaf kind and a non-leaf one. Each new `From` widens what a field can
   be used as.

## Boxes

- [x] **1.** (Todd) Decide A to I. All decided 2026-11-19.

- [x] **2.** (Claude) Tests first, on branch `bound`, with no implementation:

  1. The values view: an absent key reads `""`; `get` subscribes one key (a
     keystroke in one field re-renders no reader of another, counted with the
     `Harness`); `paths_under` returns the keys under a list and wakes on an added
     item but not on a keystroke; `peek` subscribes nothing.
  2. The errors view: `at` and `validity`, the three states, one-key subscription.
  3. `ProbeAdder` and `TagChip` rewritten to read through their bound fields
     (`field.values()`, `field.current()`), with their "as a dev's adder has to"
     comments gone.
  4. The leaf kind: a widget built only from `props.current()`, `props.on_input()`,
     `props.on_blur()` and `props.is_invalid()` is live-checked (the live-check tests
     already in `live_checks.rs`, run against such a widget).
  5. The edit kinds: a dev's adder that calls `field.add(…)` adds to its own list
     and to its own set (one `ItemAdder` for both, through `Items`); Remove, Edit and
     ↑/↓ built from bound fields still edit the right item; `VariantSelect`'s
     `choose` reaches the state.
  6. trybuild goldens, the one place "you can't" is pinned rather than just true:
     `form.values().insert(…)` does not compile; nor does `input(…)` on a
     `BoundField<Variant>`, `move_item` on a `BoundField<Set>`, or `input(…)` on a
     `BoundField<SetItem>`.

  **As built (2026-11-19, Claude).** The suite does not compile until boxes 3 and 4
  exist, as with 11h's tests.

  - **`tests/suite/bound.rs`, 13 tests**: the values view (an absent key, `get`
    after a write, `get` subscribing one key and `peek` none, both counted by
    render, `paths_under` a list beside a field whose name shares its prefix, and
    from the root, and not waking on a keystroke), a dev's adder adding through
    `field.add` to a list and to a set, the errors view's three states (seeded
    through `FormState::field_errors`), and a leaf widget written only against the
    props (`label()`, `id()`, `path()`, `current()`, `on_input()`, `on_blur()`,
    `is_invalid()`, and a handler owning `props.field()`), live-checked.
  - **`ProbeAdder`** (`vecs.rs`) and **`TagChip`** (`sets.rs`) read through their
    bound fields. The rest of the suite's old calls (`TagAdder`'s `props.path`,
    `plain_props`, the live-check `Probe`, …) are box 5's, from the compiler's list.
  - **`tests/ui-pending/bound_*.rs`, 5 cases**, no `.stderr`: writing through
    `form.values()` or `form.errors()`, `input` on a `Variant` or a `SetItem`,
    `move_item` on a `Set`. **They fail to compile today for the wrong reason**
    (nothing they name exists), so box 5 moves them to `tests/ui/`, blesses them
    with `TRYBUILD=overwrite`, and reads each `.stderr` to see it is the intended
    error: no such method, not an unresolved import.

  **The API the tests assume**, for box 3 and 4 to match or change on purpose:

  1. `ValuesView::{get, peek}(&self, &NodePath) -> String`,
     `paths_under(&self, &NodePath) -> Vec<NodePath>`, with `paths_under` segment-wise
     (`answers_note` is not under `answers`), as `NodePath::starts_with` is.
  2. `ErrorsView::at(&self, &NodePath) -> Option<Vec<ValidationMessage>>`,
     `validity(&self, &NodePath) -> Validity`.
  3. On every kind: `path() -> &NodePath`, `id() -> &str`, `label() -> Option<&str>`,
     `values() -> ValuesView`, `is_invalid() -> bool`.
  4. `BoundField<Items>::add(&self, Option<ItemValue>)`, appending; Remove and the
     `before:` position are not named by any test yet.
  5. `AddItemProps.field` and `SetItemProps.field`, `pub`, as the other props fields
     are; `props.field()` on `WidgetProps`, a clone.
  6. The markers at `formoxus::widgets::bound_field::{Leaf, Variant, List, Set,
     Items, SetItem, Struct}`, so `widgets/field_handle.rs` becomes a `pub mod
     bound_field` (it is a private `mod` today), and `BoundField` itself in the
     `widgets::*` kit (H).

- [ ] **3.** (Todd) Part 1: `ValuesView` and `ErrorsView` in `nodes.rs`; the aliases
  `pub(crate)`; `ErrorsStoreExt` becomes the view's inherent methods; every row of the
  table in "Where the stores reach a dev today" takes or returns a view; `use_form_values` deleted (C);
  `WidgetProps::new` takes maps (G).

- [ ] **4.** (Todd) Part 2: `FieldHandle` → `BoundField<K = Leaf>` and the seven
  markers, in `widgets/field_handle.rs` (renamed to match); `WidgetCommon` folded
  into it; `WidgetProps<K>`, `SetWidgetProps`, `AddItemProps`, `SetItemProps`,
  `VariantSelect` and the item buttons as in Part 2, item 5; the built-in widgets
  moved from `props.field_handle()` to the props or `props.field()` (E). The
  container nodes' own rendering may stay on `ctx.errors.at(path)` (F).

- [ ] **5.** (Claude) The suite, the examples and the e2e crate compile and pass; the
  docs (each view, `BoundField` and each kind's methods, `WidgetProps`, the
  custom-widget walkthrough in `widgets.rs`, and the adder and item-view docs);
  memory (a `bound_field` file with the "why not one plain type" reasoning, the
  renames line in `MEMORY.md`); a note for apcsp's catch-up when Todd opens it.

- [ ] **6.** (Todd) Merge `bound`; close #22 and #23.
