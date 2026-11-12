---
name: widget-slot-dispatch
description: How form! picks with_widget vs with_set_widget without seeing types: const-generic WidgetSlot<IS_SET> chosen by the field's shape
metadata:
  type: project
---

`form!` sees no types, so for `widget: SomePath` it cannot tell a `LeafWidget` from a
`SetWidget`, nor a `BTreeSet` field from a `String` one. Built 2026-11-11 (WIDGETS_PLAN
box 8): a built-in's name says its kind (`WidgetName::renders_a_set`), `set` beside a
dev's widget says set; otherwise `form!` declares `const __FX_IS_SET_n: bool =
field_kind::is_set(shape)` and calls `field_kind::WidgetSlot::<__FX_IS_SET_n>`.

**Why inherent impls on `WidgetSlot<false>` / `WidgetSlot<true>`, not a trait:** each
call type-checks only against the impl the const picked, so the other impl's bounds
and projections are never asked (`item(m.city.iter())` on a `String` would be a type
error otherwise). Two blanket `From<W: LeafWidget>` / `From<W: SetWidget>` impls into
one enum overlap (E0119); associated-type disjointness does not help.

**Why `failure` returns a `slot_failure` number** rather than panicking: a const fn
panic is reported across the whole `form!` with a toolchain-specific note inside
`field_kind`; `form!`'s own `assert!(failure != X, msg)` keeps each message's span.
One bounded const means a wrong-kind widget is refused once there, plus once at the
builder (both on the widget, via a `let __widget` binding spanned on it).

**How to apply:** a new per-widget check for a dev's widget goes into
`WidgetSlot::failure` + a `slot_failure` code + one `fails(...)` in
`WidgetRef::dev_widget_checks`; never a separate `<W as LeafWidget>` const, which would
break a `SetWidget` there. See [[lists-and-sets]], [[widget-is-the-umbrella-word]].
