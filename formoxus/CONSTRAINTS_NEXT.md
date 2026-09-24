# Constraints: where this stands, and what is left

Companion to `CONSTRAINTS_PLAN.md`, which is the **inventory** — what `form!`
should accept and which tier each setting belongs to. This file is the **build
order**: what is done, what is next, and the things already worked out or
verified so they do not have to be rediscovered.

Status as of 2026-09-24, commit `693ff6b`. `just ci` green at 38 + 282 + 100.

## What is built

Steps 1–4 of the original five-step plan are done and pushed.

| | |
|---|---|
| `Bound` | `Int(i128)` / `Float(f64)`, `Copy`, with `From` for 11 integer types and both floats. Two macro arms: nine go through `i128::from`, `usize`/`isize` need `as` because their width is target-dependent. |
| `Constraints` | `min`, `max`, `min_length`, `max_length`, `pattern`, all `Option`. Lives on both `FormField` and `FieldSpec`. |
| `value_kind()` | Combines the family read from `T` with the constraints read from the field. A `Bound::Int` on a float field widens; a `Bound::Float` on an integer field panics rather than guess a rounding direction. |
| `FormSpec::with_constraints` | One builder taking the whole struct. `apply_specs` replaces rather than merging — see the comment there for why. |
| `form!` | `min`, `max`, `min_length`, `max_length`, `pattern` as field-body keys, generated from the `field_body!` table. |

So a constraint written in `form!` reaches `ValueKind::check` on both sides of
the wire. **Nothing checks that the constraint makes sense for the field.**
That is all that is left, and it splits into three pieces.

---

## Step 5 — what the macro can check by itself

**Where:** at the end of `FieldBody::parse`, in `formoxus-macros/src/form/field.rs`,
just before `Ok(fb)`. Every key and its span is in scope there.

**Why there and not in `expand`:** `expand()` returns `TokenStream2`, not a
`Result`. `impl_form` only turns an `Err` into a compile error, and only from
parsing. There is no error channel after that point. Anything the macro wants
to reject has to be rejected during parse.

Three checks:

1. **`min_length > max_length`** and **`min > max`**.
2. **`pattern` compiles.** `regress` is already a dependency of
   `formoxus-macros`, and it is the same engine `ValueKind::check` runs at
   runtime — so "compiles at build time" really does mean "will compile at
   runtime". Wrap it the way `check` does: `^(?:{pattern})$`.
3. Duplicate and unknown keys — **already done**, via the `HashSet` in `parse`
   and `LEGAL_KEYS`.

### The catch on 1

The bounds are `Expr`, not literals. `min: MIN_AGE` and `min: 2 * N` are legal
and deliberate, and the macro cannot evaluate them. So the comparison is only
possible when **both** sides are literals:

```rust
// Roughly: syn::Lit::Int / syn::Lit::Float, else skip the check.
```

That is not a hole — an expression bound still gets caught by step 6b at
compile time, just with a worse message. Write the check as "compare when we
can" rather than trying to force literals, which would break `min: MIN_AGE`.

### Message and span

`syn::Error::new_spanned(&offending_expr, …)` puts the caret on the author's
tokens. The field name is *not* in scope in `FieldBody::parse` — it lives in
`Entry::parse_field` one level up. Try the span alone first; it points at the
right line, and the name may be redundant. If it reads badly, move the check up
to `parse_field`, which has both.

**Checkpoint:** unit tests in `field.rs` for each, plus one trybuild golden in
`formoxus/tests/ui/` so the rendered message is pinned. Regenerate goldens with
`TRYBUILD=overwrite`, then *read the diff*.

---

## Step 6a — constraint versus value kind

Rejecting `max_length` on a `bool` at compile time. This is the one that needs
`T::SHAPE`, and it is worth reading `.claude/memory/const_shape_walk_blocked.md`
first — its original conclusion was wrong and the file records the reversal.

**What works, verified:** an inline `const {}` block inside a macro-generated
generic fn sees both the caller's `T` and a literal baked into the emitted code.

**What is blocked:** crossing a `ShapeRef` to reach a *field's* shape.
`ShapeRef` is a fn pointer, and fn-pointer calls are forbidden in const fn.
Classification inspects one shape and never crosses, so this does not bite here.

### The shape of it

`expand` already emits `fn __paths_exist(__s: &Model)` holding one witness per
field. The constraint check goes in the same function, as a nested generic
helper applied to the field:

```rust
fn __c<'a, T: ::formoxus::facet::Facet<'a>>(_: &T) {
    const { assert!(::formoxus::constraints::is_text(T::SHAPE), "…") }
}
__c(&__s.bio);
```

### Three things already settled

- **The classifier must be `pub const fn` in `formoxus`**, not in the macro
  crate, because macro-generated code calls it from the consumer's crate.
- **Classify on `type_identifier` paired with `module_path`.** `scalar_type()`
  can never be const — it compares `TypeId`s, and `PartialEq::eq` is a trait
  method. Pair with the module path or a user type named `String` matches.
- **Const panic messages take a `&'static str` and nothing else.** No format
  args, no interpolated type name (`E0015`). Each check needs a fixed sentence.
  Use `quote_spanned!` so the caret lands on the author's key, not the whole
  `form!` invocation.

### The decision waiting for you

The emitted code has to name `Facet`, and **formoxus does not re-export
facet today** — the macro has never named it. Emitting `::facet::Facet` assumes
the consumer depends on facet under exactly that name, which a Cargo rename
(`facet_core = { package = "facet" }`) breaks. Re-exporting facet from formoxus
and emitting `::formoxus::facet::Facet` follows the rule the macro crate's own
docs already state about module-canonical paths. That is the recommendation,
but it is a public-API addition, so it is yours to make.

**Checkpoint:** a trybuild golden for `max_length` on a `bool`.

---

## Step 6b — does the bound fit the type

Independent of 6a; do it second only because it is fiddlier.

`max: 1e50` on an `f32` looks satisfiable and is not: `1e39` sits inside the
stated bound, but f32 parsing *saturates*, so it arrives as `inf`, and
`inf > 1e50` rejects it. The author allowed a value the form then refuses, and
the error says it is outside a range it is visibly inside.

### Two wrinkles

- **`bound_fits(T::SHAPE, 18)` will not compile.** `18` is an integer literal
  and Rust will not coerce it to an `f64` parameter. The macro has to emit
  `(#expr) as f64`, which is fine in const context.
- **Magnitude and integrality are different questions**, so probably two const
  fns: does the value fit the type's range, and (for an integer field) is it a
  whole number.

### The same check, read backwards

A **vacuous** bound — `min: -1000` on an `i8`, where `i8::MIN` is -128 — is the
same comparison against the type's range, failing in the other direction. One
check, two failure modes; closing 6b closes both. `fields.rs` has an
`#[ignore]`d test for it.

**That one needs a decision:** a vacuous bound is morally a warning, and a
`const {}` block can only hard-error. So closing it means deciding that
`min: -1000` on an `i8` fails the build. I think that is right — it is always a
mistake, usually a typo or a bound left over from a type change — but it is a
choice, not a freebie.

### The two ignored tests

`a_bound_that_overflows_f32_is_caught_before_it_can_bite` and
`an_integer_bound_too_big_for_f64_is_caught_before_it_is_rounded`, both in
`fields.rs`, plus `a_bound_no_value_of_the_field_type_could_violate_is_caught`.

**They will not flip green when 6b lands.** They are *demonstrations of the
gap*, written to fail — the f32 one asserts `1e39_f32.is_finite()`, which stays
false forever. When the const checks land they become trybuild goldens and
these three get deleted. Do not spend time trying to make them pass.

Two `#[expect(…)]` attributes in `fields.rs` are tied to this: the one on
`value_kind`'s float arm names
`an_integer_bound_too_big_for_f64_is_caught_before_it_is_rounded` in its
`reason`, and the one on that test covers the casts it performs deliberately.
Revisit both when the tests go — and note the runtime cast itself stays, so the
first `expect` probably stays with it.

---

## After that

In rough priority order, with the reasoning already written down elsewhere:

- **Document the grammar.** There is a `TODO` on `form!` in
  `formoxus-macros/src/lib.rs`. This is the only place a user can learn the
  form-level keywords or the field-body keys — `FieldBody` is `pub(crate)`.
  Deferred until the constraint work settles so the list is not written
  mid-flight. **Do this before publishing.**
- **Tier 2 widget attributes** (`class`, `placeholder`, `rows`) — the syntax
  exists (`widget: select { … }`), the attributes do not. See
  `CONSTRAINTS_PLAN.md`.
- **The error model.** Settled but unbuilt: merge `FormError` and `FieldError`,
  put the path on the producer as `Verdict<T>`, per-field validators via the
  fn-pointer + witness trick. `.claude/memory/error_model_design.md`.
- **`ErrorsByPath`**, deferred deliberately — it fixes a real reactivity
  asymmetry but the payoff needs live validation first.
- **Live validation** on `onblur` (verified: Dioxus maps `onchange` to the real
  DOM event and has `onblur`/`onfocusout`).

## Orientation, if it has been a while

- `just ci` — clippy at `-D warnings`, all tests, docs, and a `1.90` check.
- The lint table is in the root `Cargo.toml`; what is switched **off** is
  documented there with the reasoning. Site-level suppressions are
  `#[expect(…, reason = "…")]`, never `allow`, so a suppression that outlives
  its reason reports itself.
- `.claude/memory/MEMORY.md` indexes the accumulated design reasoning.
  `const_shape_walk_blocked.md` and `error_model_design.md` are the two that
  matter for what is left here.
