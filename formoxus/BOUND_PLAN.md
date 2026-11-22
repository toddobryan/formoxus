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

**Where they live: a new `pub mod stores`, `src/stores.rs`** (decision J). The views
and the path-keyed maps and stores they read move out of `nodes.rs` and `form.rs`
into it, so the form's per-path state sits in one module, apart from the node tree.

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
**`validity_at(&NodePath) -> Validity`**: `ErrorsStoreExt`'s two methods, the first
renamed from `errors_at` (Todd, 2026-11-19; he briefly chose `validity` for the
second, then kept `validity_at`). `errors.errors_at(path)` repeats itself, and
`WireForm::errors_at` already exists returning `&[ValidationMessage]`, so two
`errors_at`s returning different things would confuse a dev. `validity_at` keeps its
`_at` because the view has no validity of its own, and `BoundField::validity()`
(no argument, the node's own path) is a different method. A newtype can have
inherent methods, which the alias couldn't (the orphan rule), so the extension trait
goes.

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
   crate, reading `ctx.errors.at(path)`. They may switch to a `BoundField<List>` or
   `<Variant>` built from the `NodeContext` now or later: it is internal and breaks
   nobody, so box 4 does not require it. **No `Struct` kind (2026-11-22):** it was
   planned for the struct node and had no methods; nothing used it, a struct has no
   edits, and no widget is handed one, so it was deleted in 4.11. Adding it back is
   additive (`NodeKind` is sealed), for a dev-replaceable fieldset or a node that
   reads through one.
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
  ([[prelude-is-the-one-list]]). The six kind markers are `pub` where they are
  defined and re-exported nowhere.

**Raised by F, decided 2026-11-19 (Todd: "probably right"):**

- **I. Private fields with getters.** `WidgetCommon`'s fields are `pub` today
  (`WidgetProps::new`'s doc: "a caller sets them afterwards"), while `FieldHandle`'s
  are private. On a bound field, a `pub path` could be changed after it is built,
  and every write would then go to the wrong node. So the fields are private, read
  through the getters in Part 2 item 2, and `WidgetProps` gets `with_label` and
  `with_help_text` builders for the one caller that sets them, a widget tested
  alone.

**Decided 2026-11-19, at the start of box 3 (Todd):**

- **J. A new `pub mod stores` (`src/stores.rs`) holds the views and the path-keyed
  types**, not `nodes.rs`. `nodes.rs` is the node tree; these are the form's state,
  a different job. Named for its contents (`types` would describe every module;
  `state` competes with `form/state.rs` and `FormState`). It is a `pub mod`, like
  every other top-level module, because a public name is reached through its
  defining module ([[prelude-is-the-one-list]]) and a dev must be able to name a
  view: `form.values()` returns one.

  | Moves in | From | Visibility after |
  |---|---|---|
  | `ValuesView`, `ErrorsView` | new | `pub` |
  | `ValuesStore`, `ErrorsStore` | `nodes.rs` | `pub(crate)` (Part 1) |
  | `FilesSignal` | `nodes.rs` | `pub(crate)`, as now |
  | `ValuesByPath`, `FilesByPath` | `nodes.rs` | `pub`, as now: the suite and the wire use them |
  | `ErrorsByPath` | `form.rs` | `pub`, as now |
  | `ErrorsStoreExt` | `nodes.rs` | gone: the errors view's inherent methods |

  **Staying put:** `Handler` and `UncheckedHandler` in `form.rs` (submit-handler
  types, not per-path state), and `SpecsByPath` in `nodes.rs` (spec-side, keyed by
  `String`, not `NodePath`).

  **The cost:** about ten suite files import `formoxus::nodes::ValuesByPath` (and
  `FilesByPath`, `ValuesStore`); they become `formoxus::stores::…` in box 5, from the
  compiler's list. No box 2 test imports a view by module path (they come through
  `widgets::*`), so nothing written for this branch moves.

**Decided 2026-11-19, at the start of box 4 (Todd):**

- **K. `BoundField` and its six kind markers get their own top-level `pub mod
  bound_field`** (`src/bound_field.rs`), neither in `widgets` (where `FieldHandle`
  lives today, `widgets/field_handle.rs`) nor in `fields`.

  1. **Not `fields`:** in this crate a field is the Django-style `Field`, and `fields`
     holds one per value family (`StringField`, `BoolField`, `ChoiceField`,
     `NumberField`): the side that parses and validates. A bound field is not one of
     those. It binds a NODE to the live form state (its path, the two views, the
     `on_edit` and `on_field_event` writers), and under F most of its kinds
     (`Variant`, `List`, `Set`, `Items`) are not fields at all. Putting it
     there would blur the Field/Widget split the design came from
     ([[django-parallel-design]]).
  2. **Not `widgets`:** under F it is not only a widget's. The container nodes may hold
     a `BoundField<List>` or `<Variant>` too, so widgets are its main
     users, not its owner.
  3. **Django does the same:** its `BoundField` lives in `django.forms.boundfield`,
     beside fields and widgets rather than in either.
  4. **What stays the same:** the `widgets::*` kit still exports `BoundField` itself, as
     H says, a swap for `FieldHandle`, so no new re-export. The markers are reached at
     their defining module only: `formoxus::bound_field::{Leaf, Variant, List, Set,
     Items, SetItem}`.
  5. **The cost:** three imports in the set-aside box 2 tests change from
     `formoxus::widgets::bound_field::{Set, SetItem, Variant}` to
     `formoxus::bound_field::…` (box 5), and the crate's intra-doc links to
     `crate::widgets::FieldHandle` go to `crate::bound_field::BoundField`.

**Decided 2026-11-21, during box 4 (Todd):**

- **M. The six markers implement a sealed trait, `NodeKind`**
  (`formoxus::bound_field::NodeKind`), and `BoundField<K: NodeKind = Leaf>` is
  bounded by it. Todd asked whether an empty trait should keep anyone from writing
  `BoundField<String>`.

  1. **Sealed, not just empty.** A dev can implement a plain `pub trait` with no
     items for any type, `String` included, which would undo the point. It follows
     the crate's existing pattern in `fields/number_fields.rs`: a private
     `mod sealed { pub trait Sealed {} }`, with `NodeKind: sealed::Sealed`, so only
     this crate can add a kind.
  2. **What it buys is better errors and docs, not safety.** Nobody can get a value
     of type `BoundField<String>` anyway, because the constructor is `pub(crate)`
     (trap 7). With the bound, writing that type fails where it is written, and not
     later as a missing method. Rustdoc lists a trait's implementors, so
     `NodeKind`'s page is the one place a dev finds all six kinds; its doc carries
     Part 2's table of which kind each prop uses. Bounds also say what they mean:
     `field_row`'s `&BoundField<impl Sized>` becomes `&BoundField<impl NodeKind>`.
  3. **Named `NodeKind`, not `Kind`.** The markers name the kind of node a path is.
     `field_kind.rs` already has a private `enum Kind`, and `FieldEventKind` is in
     the same file as the trait, so a bare `Kind` would be ambiguous to read.
  4. **The cost:** every `impl<K> BoundField<K>`, and `WidgetProps<K>`, repeats
     `K: NodeKind`. The derives are unaffected (trap 6).

**Decided 2026-11-22, box 4.7 (Todd, as Claude recommended):**

- **L. Cancel travels inside the "being edited" value: `AddItemProps.editing`
  becomes `Option<Editing>`.** The plan missed this case. A set's built-in adder,
  `DraftAdder`, has a Cancel button (and Escape in the draft) that stops editing an
  item, by sending `Edit::EditItem { index: None }`. Lists and sets share one adder
  type, `ItemAdder(fn(AddItemProps) -> Element)`, so its field is the kind they
  share, `Items`, which has only `add` and `remove`, and `send_edit` is private.
  So nothing an adder holds can send `EditItem`.

  1. **The shape:** `pub struct Editing { text: String, set: BoundField<Set> }`,
     both fields private, with `text() -> &str` (for `Replace "rust"`) and
     `cancel()`, which calls `set.edit_item(None)`. It derives `Clone, Debug,
     PartialEq`, as `AddItemProps`'s derives need. It lives beside `AddItemProps`
     in `item_buttons.rs`, a `pub mod`, and is reached there,
     `formoxus::widgets::item_buttons::Editing`, NOT added to the `widgets::*`
     re-exports: no new re-exports until publish ([[prelude-is-the-one-list]]).
  2. **Why it can't be misused:** only the set node builds a `Some(Editing)`,
     because only it holds a `BoundField<Set>`. A list always passes `None`. So
     Cancel exists only while an item is being edited, and only a set ever has
     one. No new `From` is needed.
  3. **Rejected: `Items::stop_editing()`.** It is less code, but a list's adder
     could call it. That would compile, and the list would reject the edit at
     runtime (`list_node.rs`, "its items are edited in place"), logged only as a
     warning. That is the wrong call the kinds exist to make a compile error
     ([[feedback-hard-to-misuse]]).
  4. **Ordering:** `Editing::cancel` needs `BoundField<Set>::edit_item`, so that
     method moves from 4.9 to 4.8.

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

  **SET ASIDE 2026-11-19, and committed that way with box 3 (Todd's choice).** Todd
  asked to refactor the views without them, then chose to commit box 3 with them still
  aside, so the tip's `just ci` is green (1589). Box 3's commit deletes `bound.rs` and
  the five `ui-pending/bound_*.rs`, and its `suite.rs`, `sets.rs` and `vecs.rs` are the
  pre-box-2 versions (`6f02073`) plus box 3's fixes. Commit `2df77a1` still has box 2's.
  Box 5 brings them back:

  1. The six new files restore as they were:

     ```text
     git checkout 2df77a1 -- formoxus/tests/suite/bound.rs \
       formoxus/tests/ui-pending/bound_errors_view_is_read_only.rs \
       formoxus/tests/ui-pending/bound_set_has_no_move.rs \
       formoxus/tests/ui-pending/bound_set_item_has_no_input.rs \
       formoxus/tests/ui-pending/bound_values_view_is_read_only.rs \
       formoxus/tests/ui-pending/bound_variant_has_no_input.rs
     ```

  2. **`suite.rs`, `sets.rs` and `vecs.rs` are merged by hand, not checked out**: a
     checkout would undo box 3's fixes in them. From `git diff 6f02073 2df77a1 --
     <file>`, re-apply box 2's changes: `suite.rs`'s `mod bound;`, `ProbeAdder`
     (`vecs.rs`) and `TagChip` (`sets.rs`) rewritten to their bound fields. Box 3's
     interim view reads in those two go.
  3. `bound.rs`'s errors-view test calls `validity`; it is `validity_at` (item 2 of the
     API list below).

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
     `validity_at(&self, &NodePath) -> Validity`. (The tests were written calling
     `validity`; Todd chose `validity_at` in box 3, so box 5 renames the three calls
     in `bound.rs`'s errors-view test.)
  3. On every kind: `path() -> &NodePath`, `id() -> &str`, `label() -> Option<&str>`,
     `values() -> ValuesView`, `is_invalid() -> bool`.
  4. `BoundField<Items>::add(&self, Option<ItemValue>)`, appending; Remove and the
     `before:` position are not named by any test yet.
  5. `AddItemProps.field` and `SetItemProps.field`, `pub`, as the other props fields
     are; `props.field()` on `WidgetProps`, a clone.
  6. The markers at `formoxus::widgets::bound_field::{Leaf, Variant, List, Set,
     Items, SetItem}`, so `widgets/field_handle.rs` becomes a `pub mod
     bound_field` (it is a private `mod` today), and `BoundField` itself in the
     `widgets::*` kit (H). **Changed by K:** the module is top-level,
     `formoxus::bound_field::…`, so box 5 changes the tests' three marker imports.

- [x] **3.** (Todd) Part 1:

  1. A new `pub mod stores` in `src/stores.rs`, holding what decision J's table lists:
     `ValuesView` and `ErrorsView`, and the path-keyed maps and stores moved there
     from `nodes.rs` and `form.rs`.
  2. The store aliases `ValuesStore` and `ErrorsStore` become `pub(crate)`.
  3. `ErrorsStoreExt` becomes the errors view's inherent methods.
  4. Every row of the table in "Where the stores reach a dev today" takes or returns
     a view.
  5. `use_form_values` is deleted (C).
  6. `WidgetProps::new` takes maps (G).

  **As built (2026-11-19, Todd; the test side and a few loose ends Claude).**

  - **`src/stores.rs`**, a `pub mod`: `ValuesView` (`get`, `peek`, `paths_under`) and
    `ErrorsView` (`at`, `validity_at`; see Part 1 for the names), each a `Copy`
    newtype whose field is `pub(crate)`, which is the crate's way to the store inside.
    Moved in: `ValuesByPath`, `FilesByPath`, `ErrorsByPath` (all `pub`), `ValuesStore`,
    `ErrorsStore` and `FilesSignal` (all `pub(crate)`). `ErrorsStoreExt` and
    `use_form_values` are gone.
  - **The crate's own fields keep the stores** (`Contents.values`, `FormState`'s errors
    store, `apply_edit`'s parameter); a view is made where one is handed out:
    `values()`, `errors()`, `FormWide.errors`, `WidgetCommon.errors`, and
    `AddItemProps.values`/`SetItemProps.values` until box 4 removes them. A first try
    held views internally, and every internal write broke.
  - **`ErrorsByPath` is out of the prelude**, on purpose (Todd: "keep everything in the
    project pointing at where things actually live"); the pre-publish checklist,
    item 5, has it.
  - **`NodePath::root()` is `pub`** (Todd: devs may need it), with a doc.
  - **The suite**: imports moved to `formoxus::stores`; `with_store_keys`, `TagChip`
    and `ProbeAdder` read through the views; the variant left-behind check asks
    `paths_under(leaf)` for the key, not `peek` for a value; `plain_props` and
    `through_widget` pass maps; `PasswordWithEcho` builds its props once in `use_hook`
    and echoes `field_handle().current()`, because `WidgetProps::new` makes its own
    store and props built every render would hand the re-render an empty one.
  - Five intra-doc links fixed; the two in docs on public items now name the
    `ValuesView`, since a `pub(crate)` alias trips `private_intra_doc_links`.
  - `just ci` green, 1589.

- [x] **4.** (Todd) Part 2: `FieldHandle` → `BoundField<K = Leaf>` and the six
  markers, moved from `widgets/field_handle.rs` to a new top-level `pub mod
  bound_field` in `src/bound_field.rs` (K), with `BoundField` in the `widgets::*` kit
  in place of `FieldHandle` (H); `WidgetCommon` folded
  into it; `WidgetProps<K>`, `SetWidgetProps`, `AddItemProps`, `SetItemProps`,
  `VariantSelect` and the item buttons as in Part 2, item 5; the built-in widgets
  moved from `props.field_handle()` to the props or `props.field()` (E). The
  container nodes' own rendering may stay on `ctx.errors.at(path)` (F).

  In order, one kind at a time. Until 4.5 is done, the aim is the library alone,
  `cargo check -p formoxus --lib`. The suite, the examples and the e2e crate break at
  4.4 and stay broken until box 5 (Claude), which also restores box 2's tests.

  - [x] **4.1** Started (2026-11-20, `524e173` and `6ba3d3a`).
    `widgets/field_handle.rs` moved to `src/bound_field.rs`, and `FieldHandle` was
    renamed `BoundField` at every use. The six markers are `pub` unit structs
    deriving `Clone, Copy, Debug, Default, PartialEq, Eq` (trap 6). The struct is
    `BoundField<K = Leaf> { id, path, label, help_text, form: Rc<FormWide>, _data }`.
    `FormWide` derives `PartialEq`. A new `Contents::write_value` exists, and `absorb`
    and `reset` already call it.

  - [x] **4.2** Stop the form's own writes from going through a bound field
    (`form.rs`). These are the two errors in `form.rs`.

    1. `apply_edit` (around line 1153) is a free function, so it must call
       `contents.write_value(&path, raw)`, not `self.contents.…`.
    2. `absorb` (around line 744) needs `raw.clone()`.
    3. `Form::write` (around line 916) still calls `BoundField::new(&key, contents,
       OnFieldEvent { … })`. It should call `write_value`, then
       `OnFieldEvent { … }.send(&key, FieldEventKind::Written)`.
    4. Delete `BoundField::unreported`.
    5. Fix `Contents::write_value`'s doc: it is still `BoundField`'s old doc, and its
       first line is indented wrongly.

  - [x] **4.3** Give `BoundField` its constructor and the methods every kind shares
    (`bound_field.rs`, Part 2, items 1 and 2).

    1. Write one `pub(crate) fn new(id, path, label, help_text, form: Rc<FormWide>)`
       in `impl<K> BoundField<K>`. It is generic in `K`, because the node calling it
       knows its own kind (trap 7).
    2. In the method bodies, replace `self.contents` with `self.form.contents`, and
       `self.handler` with `self.form.on_field_event`.
    3. Put the getters in `impl<K>`, all `pub`: they are how a custom widget reads
       what were `WidgetCommon`'s `pub` fields (I). They return borrows: `path() -> &NodePath`,
       `id() -> &str`, `label()` and `help_text() -> Option<&str>`, and
       `classes() -> &Rc<Classes>`. Add `errors() -> ErrorsView` and
       `values() -> ValuesView`, which are both `Copy`.
    4. Move `WidgetCommon`'s three methods into `impl<K>`: `validity`, `is_invalid`
       and `part_class::<W>`. Leave `WidgetCommon` in place for now, because
       `SetWidgetProps` still uses it until 4.5.
    5. Add one private `send_edit(&self, edit: Edit)` in `impl<K>`, which calls
       `self.form.on_edit`. Every kind's edit method in 4.5 to 4.10 goes through it.
    6. Leave the current methods in `impl BoundField` (which means `Leaf`): `current`,
       `input`, `change`, `blur` and their `on_*` builders, `pick_file`,
       `on_pick_file`, `picked_file` and `resets`.

  - [x] **4.4** `Leaf`: `WidgetProps` holds a `BoundField` (Part 2, item 5.1). It
    stays non-generic for now, because `VariantSelect` still takes it until 4.6.

    1. In `widget_props.rs`, `WidgetProps` drops `common`, `contents` and
       `on_field_event`, and gains `field: BoundField`. `Deref` targets the
       `BoundField`. `DerefMut` goes, because the fields are private (I).
    2. `field()` returns a clone, replacing `field_handle()`.
    3. Add the builders `with_label` and `with_help_text` (I).
    4. **`WidgetProps::new` is renamed `WidgetProps::detached`** (2026-11-21). Its
       only callers are two suite tests (box 5 changes them). `new` reads as the
       normal way to build props, and it is not: the nodes build the real ones. A
       dev writing a widget that renders another would reach for `new`, and the
       inner widget would then lose its live check. Its doc warns about this, but
       the name should say it ([[feedback-hard-to-misuse]]): these props are
       connected to no form. The doc keeps the warning, and says to pass a clone of
       your own props on instead. Its body builds a throwaway `FormWide`:
       `Callback::new(|_| {})` for
       `on_edit`, `OnFieldEvent { callback: None, is_live: false }` (this replaces
       `unreported()`), a fixed form id, and `Formoxus::default().label_case`.
       Nothing reads that casing (a bound field never does, and `VariantSelect`
       gets its own as a prop), so any valid value works. Taking it from
       `Formoxus::default()` keeps the built-ins in one place. `LabelCase` gets no
       `Default` (2026-11-21): a `LabelCase::default()` would read as the right
       answer anywhere, and a node calling it while rendering would skip the
       config cascade without anyone noticing.
    5. `current_attrs` and `control_class` read through the getters.
    6. Make `field_row` take `&BoundField<K>`, generic in `K`, in place of
       `&WidgetCommon`. The set widgets and `VariantSelect` use it too.
    7. Change the nodes that build a `WidgetProps`: `field_node.rs` (around line
       338), `file_node.rs` (around 139) and `variant_node.rs` (around 273, still a
       leaf's props until 4.6). Each passes `ctx.form.clone()` to `BoundField::new`.
    8. Change the eight `field_handle()` callers to `field()`: `input.rs` (4),
       `textarea.rs`, `select.rs`, `radio_group.rs` and `file_input.rs`.
    9. Change about 50 direct field reads in `src/widgets` (`props.id`,
       `props.help_text`, `props.classes`, …) to the getters. The compiler lists them.
    10. The lib still does not compile here: `select_multiple.rs` uses `field_row` on
        `SetWidgetProps`. Doing 4.5 next is what fixes it.

  - [x] **4.5** `Set`: `SetWidgetProps` (Part 2, item 5.3).

    1. `SetWidgetProps` drops `common` and `on_edit`, and gains
       `field: BoundField<Set>`. Its `Deref` targets the bound field, and its
       `DerefMut` goes.
    2. In `impl BoundField<Set>`, add `add(Option<ItemValue>)` and `remove(index)`.
       `edit_item` waits for 4.9, when `EditItemButton` needs it.
    3. Move `checkbox_multiple.rs` and `select_multiple.rs` to `props.field.add(…)`
       and `props.field.remove(…)`, in place of `on_edit.call(Edit::…)`.
    4. `set_node.rs` (around line 974) builds the `BoundField<Set>`.
    5. Delete `WidgetCommon`, and its re-export in `widgets.rs`.
    6. **The lib compiles. Commit.**

  - [x] **4.6** `Variant`: `VariantSelect` (Part 2, item 5.2).

    1. Add `NodeKind` (M) in `bound_field.rs`: the private `sealed` module, the
       trait, and an impl of both for each of the seven markers. Bound the struct
       and `impl<K>` by `K: NodeKind`, and change `field_row`'s parameter to
       `&BoundField<impl NodeKind>`.
    2. `WidgetProps` becomes `WidgetProps<K: NodeKind = Leaf>`, holding
       `BoundField<K>`. `LeafWidget::render` still takes plain `WidgetProps`.
    3. In `impl BoundField<Variant>`, add `choose(Option<&str>)`, which sends
       `Edit::new_choose_variant` at its own path.
    4. `VariantSelect` takes `WidgetProps<Variant>`, and drops its `on_edit` prop.
    5. Keep trap 4 in mind: `variant_node.rs` passes the attributes with the error
       attributes already applied. The `aria-describedby` tests in `enums.rs` catch
       a mistake there in box 5.

  - [x] **4.7** Decide **L**, `DraftAdder`'s Cancel. Decided 2026-11-22:
    `AddItemProps.editing` becomes `Option<Editing>`, holding the item's text and a
    `BoundField<Set>`, with `cancel()`. See L under Decisions.

  - [x] **4.8** `Items`: `AddItemProps` and `RemoveItemButton` (Part 2, items 5.4
    and 5.6).

    1. `AddItemProps` drops `path`, `id`, `values`, `on_edit` and `classes`, and
       gains `field: BoundField<Items>`.
    2. In `impl BoundField<Items>`, add `add(Option<ItemValue>)` (which appends, so
       `before: None`) and `remove(index)`.
    3. In `impl BoundField<Set>`, add `edit_item(Option<usize>)`, which sends
       `Edit::EditItem` at its own path. It is here, not in 4.9, because L's
       `Editing::cancel` calls it.
    4. Add `Editing` (L) beside `AddItemProps`, and change `editing` to
       `Option<Editing>`. `set_node.rs` (around line 1081) builds it from the item's
       text and its `BoundField<Set>`, and `list_node.rs` keeps `None`.
    5. Add `From<BoundField<List>>` and `From<BoundField<Set>>` for
       `BoundField<Items>`, and no conversion back (trap 8).
    6. Move `Add` and `DraftAdder` to the field. `DraftAdder` builds its draft id
       from `field.id()`, takes Add from `field.add(None)`, and takes its Replace
       label and Cancel from `editing`: `editing.text()` and `editing.cancel()`.
    7. `RemoveItemButton` takes `field: BoundField<Items>` in place of `path`,
       `on_edit` and `classes`.
    8. Change the callers: `list_node.rs` (around lines 117 and 125) builds a
       `BoundField<List>` and calls `.into()`. `set_node.rs` (around 621 and 1073)
       does the same with its `BoundField<Set>`.

  - [x] **4.9** `SetItem`: `SetItemProps` and `EditItemButton` (Part 2, item 5.5).

    1. `SetItemProps` drops `path`, `values` and `classes`, and gains
       `field: BoundField<SetItem>`.
    2. In `impl BoundField<SetItem>`, add only a read-only `current()`. A set's item
       must not be written directly, because that would skip the set's duplicate
       check.
    3. `EditItemButton` takes `field: BoundField<Set>` in place of `path`, `on_edit`
       and `classes`, and calls the `edit_item` that 4.8 added.
    4. Change `set_node.rs` (around lines 594 and 612).

  - [x] **4.10** `List`: `MoveItemButtons` (Part 2, item 5.6).

    1. In `impl BoundField<List>`, add only `move_item(from, to)`. A list's adder
       and its Remove buttons go through `BoundField<Items>` (4.8), so a list's own
       `add` and `remove` would be near-duplicates with no caller. (`Set` keeps
       its own, because `SelectMultiple` takes a `BoundField<Set>`.)
    2. `MoveItemButtons` takes `field: BoundField<List>` in place of `path`, `on_edit`
       and `classes`.
    3. Change `list_node.rs` (around line 115), reusing the `BoundField<List>` from
       4.8.

  - [x] **4.11** Loose ends, then commit and push.

    1. Fix the doc links that still name `WidgetCommon`, `FieldHandle` or
       `field_handle`: `widgets.rs` (lines 62, 63 and 82), `field_attrs.rs` (299),
       `nodes.rs` (414 to 419), and `BoundField`'s own doc.
    2. `FormWide.classes`'s doc (`nodes.rs`, around 413) says a widget "must not
       hold the whole `FormWide`" because that would hand it the form's callbacks.
       That is no longer true: a bound field holds it, behind private fields. Say
       why `classes` is still an `Rc` of its own, or stop it being one.
    3. Give `BoundField<Set>::edit_item` a doc: what `Some(index)` and `None`
       do, and that `Editing::cancel` and `EditItemButton` are its callers.
    4. Run `cargo clippy -p formoxus --lib` and `cargo doc -p formoxus --no-deps`
       until both are clean. The rest of `just ci` is box 5's.
    5. Put `BoundField` in the `widgets::*` kit, `pub use
       crate::bound_field::BoundField;` in `widgets.rs`, as H decided. It is the
       swap for `FieldHandle`, which was there, so not a new re-export. Found
       2026-11-22: 1-4 are done (and the unused `Struct` marker is deleted), but
       this was not, so a dev's widget needs a second `use` to name the type.
       **Deferred (Todd, 2026-11-22)** to the re-export audit before publishing
       (`.claude/memory/pre_publish_checklist.md`, item 5), so 4.11 is done without it.

    The push waits for box 5 (Todd, 2026-11-22).

- [x] **5.** (Claude) The suite, the examples and the e2e crate compile and pass; the
  docs (each view, `BoundField` and each kind's methods, `WidgetProps`, the
  custom-widget walkthrough in `widgets.rs`, and the adder and item-view docs);
  memory (a `bound_field` file with the "why not one plain type" reasoning, the
  renames line in `MEMORY.md`, which also records the moves from `nodes::` and
  `form::` to `stores::`); a note for apcsp's catch-up when Todd opens it.

  **As built so far (2026-11-22, Claude).**

  1. **Tests:** box 2's `tests/suite/bound.rs` and the five `bound_*` goldens are back
     (`391b09d`), all green; each `.stderr` is "no method named …", the intended error.
     Two fixes to box 2's own test widget: its view takes `widget_props`, and it renders
     `FieldErrors`. The restored keystroke test found that `paths_under` subscribed
     deeply; Todd fixed it with the store's shallow `iter()`.
  2. **Docs:** the two views and their methods, `NodeKind` and each marker, the
     `BoundField` getters and every kind's methods, `Editing`, `WidgetProps` and
     `SetWidgetProps` with their fields, the props' `field()` methods, and
     `SetItemProps`. `cargo doc` and `-W missing_docs` are clean for `bound_field`,
     `stores`, `widget_props`, `item_buttons` and `set_item`. The `widgets.rs`
     walkthrough was redone in 4.11.
  3. **Memory:** `.claude/memory/bound_field.md` (with the "why not one plain type"
     reasoning), the renames line in `MEMORY.md`, and the handoff.
  4. **`just e2e`:** 85/85 green on this branch.

  **For apcsp, when Todd opens it** (it points at `../formoxus/formoxus` by path, so it
  builds against whatever branch is checked out here). It already fails against `main`
  for the 2026-11-11 `Widget`/`LeafWidget` split; on top of that, `bound` breaks two
  places, both small:

  1. `crates/ui/src/markdown.rs`, `MarkdownView`: `props.field_handle()` becomes
     `props.field()`, or drop `handle` and call through the props (`props.current()`,
     `props.on_input()`, `props.on_blur()`); `props.id` and `props.path` become
     `props.id()` and `props.path()`.
  2. `crates/web/src/views/form_demo.rs`, the "value store" debug table: `ValuesView` has
     no `read()`, so the whole map can no longer be read at once. Build the rows from
     `form.values().paths_under(&NodePath::root())`, each value by `get`. It still
     re-renders on every keystroke, since it shows every value, so its NOTE comment
     about moving the table into a child still holds.

  `crates/api`'s `form.values()` calls are `WireForm::values`, unaffected.

- [ ] **6.** (Todd) Merge `bound`; close #22 and #23.
