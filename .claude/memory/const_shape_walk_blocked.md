---
name: const-shape-walk-blocked
description: "Compile-time checking against facet's Shape. CORRECTED 2026-09-25: use a FREE const item with a projection closure (shape_of), NOT a const{} block in a generic fn, which cargo check never evaluates and an uncalled fn never triggers. VERIFIED 2026-09-21: a const fn CAN classify one shape, and an inline const{} block in a generated generic fn sees both T::SHAPE and a literal baked into it — which closes BOTH the constraint-vs-value-kind check and the bound-fits-the-type check at compile time. What is blocked is only CROSSING a ShapeRef to reach a field's type, which classification never needs. Supersedes the earlier marker-trait recommendation"
metadata:
  type: project
---

**CORRECTION 2026-09-25 — read this before the rest.** The `const {}` block in
a generic fn below DOES fire, but only when that fn is monomorphized. That
means (a) never under `cargo check`, so never in rust-analyzer, and (b) never at
all when nothing calls it, which is true of `form!`'s `__paths_exist` witness.
Verified: placed in `__paths_exist`, 0 errors under check AND build. What
works, and what shipped as step 6a, is a FREE `const _` item calling a generic
const fn with a projection closure. It fires under `cargo check`, in libs and
bins, nested inside closures:

```rust
const _: () = assert!(takes_length(shape_of(|__m: &Model| &__m.bio)), "…");
pub const fn shape_of<M, F: for<'a> Facet<'a>>(_: fn(&M) -> &F) -> &'static Shape { F::SHAPE }
```

The classifier is `formoxus::field_kind`. Everything below about WHAT const
code can see (one shape yes; `OptionDef::t` yes, it is a plain `&Shape`; a
tuple struct's field shape no, it is a `ShapeRef`) still holds.

**Read the update first — this file's original conclusion was wrong.** It said
the const route was a dead end and to use marker traits. That over-applied a
real but much narrower blockage.

## What works — verified by compiling, 2026-09-21

An inline `const {}` block inside a macro-generated generic fn can see **both**
the caller's `T` and a literal baked into the emitted code:

```rust
fn check<'a, T: Facet<'a>>(_x: &T) {
    const { assert!(bound_fits(T::SHAPE, 1e50), "bound does not fit this field's type") }
}
check(&__s.ratio);          // emitted into form!'s witness
```

```
error[E0080]: evaluation panicked: bound does not fit this field's type
   |  evaluation of `main::check::<'_, f32>::{constant#0}` failed here
```

**The bound does not need to be a function argument**, which is what made the
earlier analysis conclude this was impossible — a runtime `f64` parameter is
invisible to `const {}`, and `f64` const generics are unstable. As a literal in
the generated source it is visible, and no const generics are needed.

So both compile-time checks land:

- **constraint vs. value kind** — `max_length` on a `bool` field
- **a bound that overflows the field's own type** — `max: 1e50` on an `f32`,
  which was previously expected to be a runtime panic in `apply_specs`

Classification itself is a const fn over ONE shape. `scalar_type()` can never be
const (it compares `TypeId`s, and `PartialEq::eq` is a trait method), so classify
on `shape.type_identifier` with a hand-written const `str_eq` — and pair it with
`module_path` so a user type named `String` cannot false-positive.

## Why this beats marker traits

1. **One source of truth.** `value_kind()` already classifies by shape. A
   `#[diagnostic::on_unimplemented]` marker trait needs a hand-kept `impl` list —
   exactly the sync hazard the `widgets!` table doc argues against.
2. **Three-valued, not two.** A trait bound can only pass or fail. A shape can
   say *"can't tell"*: `is_text(shape) || is_newtype(shape)` lets
   `Markdown(String)` through unjudged, which is CORRECT — at runtime
   `value_kind()` sees the inner `String`, so the constraint genuinely applies.
   This closes the newtype gap that was previously thought unavoidable.
3. **No orphan problem.** Marker traits would need `impl HasLength for Markdown`
   in the model's crate, dragging formoxus into it. A shape needs nothing the
   model does not already derive.

## What IS still blocked, and why it does not matter here

Crossing from a shape to a FIELD's shape:

```
error: function pointer calls are not allowed in constant functions
   |  Type::User(UserType::Struct(st)) => (st.fields[0].shape.0)(),
```

`ShapeRef` is `pub struct ShapeRef(pub fn() -> &'static Shape)` — a fn pointer
by design, because that indirection is what lets recursive types have shapes.
Marking `ShapeRef::get` const upstream would not help (the body still calls the
pointer) and removing the indirection would break recursive types.

**Classification never crosses anything.** It inspects one shape, which is why
the blockage does not apply.

## Limits of const-block diagnostics

- **No values in the message.** `assert!` in const takes a `&'static str`;
  formatting is `E0015: cannot call non-const formatting macro`. So "min_length
  must not exceed max_length", never "min_length 5 exceeds max_length 3".
- **A runtime expression is rejected** with `E0435: attempt to use a non-constant
  value in a constant` — which is a feature: it forbids `max_length: some_var`,
  matching the rule that a bound varying at runtime is a validator.
- **Named consts and const arithmetic DO work**, so this covers more than
  literals.
- **The span lands on formoxus's own source** unless `quote_spanned!` moves it
  onto the author's tokens — the trick `probe()` already uses for `[]`.

## The one thing that still needs a literal

`pattern`. Regex parsing allocates, and heap allocation is forbidden in const
evaluation, so `regress::Regex::new` cannot be a const fn and no const regex
parser exists. The macro must see a `LitStr` and validate it with the same engine
that will run it.
