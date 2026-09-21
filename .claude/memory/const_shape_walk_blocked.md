---
name: const-shape-walk-blocked
description: "VERIFIED DEAD END, 2026-09-20. A `const fn` walk over facet's `Shape` cannot cross from a type to its field's type, because `ShapeRef` holds a `fn() -> &'static Shape` and function-pointer calls are forbidden in const fn. This kills compile-time newtype peeling, which was the whole reason to want it. Use the marker-trait route instead"
metadata:
  type: project
---

## What was being attempted

Compile-time gating of field constraints (`max_length`, `pattern`, `min`/`max`)
in `form!` — rejecting `max_length` on a `bool` field with a compile error
rather than letting it be silently ignored. `form!` has **zero type
information** (its witness is a bare `let _ = &__s.password;`), so the check has
to be delegated to rustc. Two candidate routes, and this memo is about the one
that lost. See [[widget-attribute-gating]] for the surrounding design.

## What works in a const walk

All of `Shape`'s fields are `pub`, and `Type` / `UserType` / `StructType` are
ordinary matchable enums. So this compiles and gives the right answer:

```rust
const fn is_newtype(shape: &'static Shape) -> bool {
    match shape.ty {
        Type::User(UserType::Struct(st)) =>
            matches!(st.kind, StructKind::TupleStruct) && st.fields.len() == 1,
        _ => false,
    }
}
```

`const _: () = assert!(is_text_shape(<String as Facet>::SHAPE));` also compiles.

And an inline `const {}` block CAN see the enclosing function's generic
parameter, which is the only way the macro could check a field whose type it
cannot name:

```rust
pub fn assert_text<'a, T: Facet<'a>>(_: &T) {
    const { assert!(is_text_shape(T::SHAPE), "this constraint applies to text fields") }
}
```

That produces a real compile error naming `assert_text::<'_, bool>`.

## What does NOT work, and why it cannot be fixed upstream

Crossing from a shape to a field's shape:

```
error: function pointer calls are not allowed in constant functions
   |  Type::User(UserType::Struct(st)) => (st.fields[0].shape.0)(),
```

`pub struct ShapeRef(pub fn() -> &'static Shape)` — a function pointer **by
design**, because that indirection is what lets recursive types have shapes
(facet's own doc: "enabling lazy evaluation for recursive types"). So:

- Marking `ShapeRef::get` as `const fn` upstream would NOT help — the body still
  calls a fn pointer.
- Changing `ShapeRef` to hold `&'static Shape` directly would break recursive
  types.

There is no other route to the inner type: a newtype's `scalar_type()` is
`None`, and `parse_from_str` fails even with `#[facet(transparent)]` (see
[[facet-newtypes-and-custom-widgets]]).

**Also note `scalar_type()` itself can never be const**: it works by comparing
`TypeId`s (`type_id == TypeId::of::<String>()`), and `PartialEq::eq` is a trait
method, forbidden in const fn.

## Why that kills the route

Newtypes are LEAVES whose `ValueKind` comes from the INNER scalar — `FormField`
carries the inner type with the newtype's shape on a side channel. So a check
that cannot peel would reject `Markdown(String)` for `max_length`, which is
precisely the kind of field custom widgets exist for.

## The decision

**Use the marker-trait route**: `trait HasLength {}` + `#[diagnostic::on_unimplemented]`,
asserted at the witness. Verified to give a fully custom message with the
primary span on the field:

```
error[E0277]: `max_length` applies to text fields, and `bool` is not one
20 |     assert_has_length(&p.active);
   |                        ^^^^^^^^^ this field is `bool`
```

It has the SAME newtype gap (trait resolution can't see through `Markdown`
either), but strictly better diagnostics — the const-block error puts its
primary span on formoxus's own source and relegates the call site to a `note:`.

Generate the impls from the same table as `value_kind()`'s `match scalar`
(fields.rs:92) so the two cannot drift — the sync hazard the `widgets!` table
doc already calls out.

**Newtypes remain unsolved.** The options are: the author writes
`impl HasLength for Markdown {}` (needs the model crate to depend on formoxus,
the dependency this design avoids), or constraints on newtype fields go
unchecked at compile time.
