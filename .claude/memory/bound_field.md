---
name: bound-field
description: "BUILT 2026-11-22 on branch `bound` (#22, #23): every widget gets a BoundField<K> of its node's kind, the only way it changes the form; the read-only views; why one type per kind, sealed NodeKind, Items, Editing, Rc<FormWide>; the paths_under shallow-read lesson"
metadata:
  type: project
---

`formoxus::bound_field::BoundField<K: NodeKind = Leaf>` replaced `FieldHandle` and
`WidgetCommon` on branch `bound` (`formoxus/BOUND_PLAN.md`, issues #22 and #23). It
holds a node's `id`, `path`, `label`, `help_text` and an `Rc<FormWide>` behind private
fields. Every prop type holds one `pub(crate)` and derefs to it, plus `field()` for an
owned copy a handler can capture. The views `stores::{ValuesView, ErrorsView}` are
read-only `Copy` newtypes over the stores.

**Why one type per kind, not one plain `BoundField` with every method** (decision F,
item 7): with one type the wrong call compiles: `input()` on the variant select
(writes a raw string over the enum's structure entry), `move_item` on a set,
`input()` on a set item (skips the duplicate check). With the kind parameter each is
a compile error, pinned by five trybuild goldens `tests/ui/bound_*.rs`. The six kinds
and their methods:

| kind | handed to | its methods |
|---|---|---|
| `Leaf` | `WidgetProps` | `current`, `input`/`change`/`blur` + `on_*`, file methods, `resets` |
| `Variant` | `VariantSelect` (`WidgetProps<Variant>`) | `choose` |
| `List` | `MoveItemButtons` | `move_item` |
| `Set` | `SetWidgetProps`, `EditItemButton` | `add`, `remove`, `edit_item` |
| `Items` | `AddItemProps`, `RemoveItemButton` | `add`, `remove` |
| `SetItem` | `SetItemProps` | `current`, read-only |

**`Items` exists because one adder type serves lists and sets** (`ItemAdder(fn(
AddItemProps) -> Element)`), so its field must be a kind both share. `From<List>` and
`From<Set>` into `Items`; nothing converts back. A list has no `add`/`remove` of its own
(4.10: they'd be near-duplicates with no caller); a set keeps its own for
`SelectMultiple`. `SetItem` is not a conversion of `Set`: it has its own path
(`tags.#1`) and id, built by the crate-private `BoundField<Set>::item`.

**`Struct` was planned and deleted (2026-11-22):** no edits, no holder. Adding a kind
back is additive since `NodeKind` is sealed.

**`NodeKind` is sealed (decision M)**, not merely empty: a dev could implement an empty
`pub trait` for `String`. It buys errors and docs, not safety (the constructor is
`pub(crate)`). Supertraits `Clone + Copy + Debug + Default + PartialEq + Eq` let the
derives on `BoundField<K>` work without bounds on `K`.

**Cancel travels in `Editing` (decision L):** `AddItemProps.editing:
Option<Editing>`, `Editing { text, set: BoundField<Set> }` with `text()` and
`cancel()`. The adder's `Items` field cannot send `EditItem`, and must not. But
`SetNode.editing` stays `Option<String>`: it is node state in `FormState`, which reaches
the server, so **an `Rc` must never go into `FormState` or `FormSpec`.** Editing is at
`formoxus::widgets::item_buttons::Editing`, no re-export.

**`Rc<FormWide>`, not five fields:** the form-wide parts (contents, errors, classes,
`on_field_event`, `on_edit`) are shared. `FormWide.classes` stays an `Rc` of its own
because `FieldErrors` and `WidgetProps::detached` take only the classes, as a prop.

**Lesson, found by a box 2 test (2026-11-22): a store read that only needs the KEYS
must use the store's `iter()`/`len()`, which track shallowly.** `paths_under` first
called `self.0.read().keys()`, which subscribes to every value, so an adder counting
its list's entries re-rendered on every keystroke. `Contents::write_value` already
avoided dirtying the shape on an existing key (`get_unchecked(..).set`, not `insert`);
both halves are needed. Test: `bound::paths_under_does_not_wake_on_a_keystroke_in_an_item`.

**`WidgetProps::new` → `detached`**: props connected to no form, for a test or a widget
shown outside one. A widget that renders another must pass its own props on, never
build detached ones (the inner widget would be outside the form).

**Not yet:** `BoundField` is not in the `widgets::*` kit, though decision H said it
would be; Todd deferred it to the pre-publish re-export audit ([[pre-publish-checklist]]).

Related: [[prelude-is-the-one-list]], [[django-parallel-design]] (Django's
`forms.boundfield` is why it is its own top-level module, decision K),
[[feedback-one-function-not-two]].
