---
name: widget-trait-family
description: "DECIDED 2026-11-11 (WIDGETS_PLAN G'): `Widget` is the BASE (NAME, PARTS, EXTENDS only); `LeafWidget: Widget` renders one value (ACCEPTS, ACCEPTS_OPTIONAL, TAKES_CHOICES, value_when_empty, render); `SetWidget: Widget` next. Why not one trait, why not SetWidget: Widget-as-leaf, why not `SingleItemWidget`"
metadata:
  type: project
---

**Split by who reads what.** The class system (`Classes::part`, `has_part`,
`field_row`) and `form!`'s attribute checks read only `NAME`, `PARTS` and
`EXTENDS`, so those are `Widget`, and every kind of widget has them.
`field_kind::renders`/`optional_is_okay`, `AnyWidget` and the field node read the
rest, which only makes sense for a widget over ONE value, so that is
`LeafWidget: Widget`. A widget on a whole set (`checkbox_multiple`,
`select_multiple`) is `SetWidget: Widget`, with its own props. Todd's framing: a
family of traits for specific cases, rather than one generic `Widget` full of
items that do not apply.

**Rejected:** a supertrait `SetWidget: LeafWidget` (or `: Widget` meaning the
old full one). Every set widget would then BE a leaf widget, with a leaf `render`
it could only stub, and would type-check as `widget:` on a `String`.

**Names:** `Leaf` is already public vocabulary (`nodes::RawValue::Leaf`,
`field_node::LeafArgs`, "one component per leaf"). `SingleItemWidget` was set
aside because "item" means one element of a list or set (`#3`, `ItemValue`), and a
leaf widget renders no items. `FieldWidget` was the deleted derive path's name.

**`form!` emits `::formoxus::widgets::LeafWidget`** for `TAKES_CHOICES`/`ACCEPTS`
and `::formoxus::widgets::Widget` for `EXTENDS`; both are re-exported from
`widgets`, the glob a dev writing a widget imports. See [[widget-is-the-umbrella-word]]
(the vocabulary) and [[widgets-declare-parts-presets-own-classes]] (PARTS).
