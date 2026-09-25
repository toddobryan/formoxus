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

## Step 5 — the pattern check

> **Done 2026-09-25**, and it goes further than the plan below. It follows
> HTML's "compiled pattern regular expression" algorithm exactly: compile the
> BARE pattern, then the wrapped one, both with the `v` flag. Bare-first
> rejects `a)|(b`, which compiles only once wrapped and which a browser
> ignores. `v` is what browsers use, and `ValueKind::check` now uses it too.
> Without it, `\p{L}` means a literal `p{L}` on the server and a letter in the
> browser.

> **Revised 2026-09-24.** This step used to include `min > max` and
> `min_length > max_length` as parse-time comparisons. It does not: those belong
> in the const witness with the rest of step 6, for the reason below. Step 5 is
> now just the regex.

**Where:** at the end of `FieldBody::parse`, in `formoxus-macros/src/form/field.rs`,
just before `Ok(fb)`.

**Why there and not in `expand`:** `expand()` returns `TokenStream2`, not a
`Result`. `impl_form` only turns an `Err` into a compile error, and only from
parsing. There is no error channel after that point.

**The check:** `pattern` compiles. `regress` is already a dependency of
`formoxus-macros`, and it is the same engine `ValueKind::check` runs at
runtime — so "compiles at build time" really does mean "will compile at
runtime". Wrap it the way `check` does: `^(?:{pattern})$`.

This one *cannot* move to a const block: regex parsing allocates, and const
forbids allocation. That is the whole reason `pattern` is a `LitStr` while the
bounds are `Expr` — the macro has to read the string itself.

`syn::Error::new_spanned(&lit, …)` puts the caret on the author's pattern. The
field name is not in scope in `FieldBody::parse` — it lives in
`Entry::parse_field` one level up — but the span points at the right line, so
try without it first.

**Checkpoint:** a unit test in `field.rs`, plus a trybuild golden in
`formoxus/tests/ui/` pinning the rendered message. Regenerate goldens with
`TRYBUILD=overwrite`, then *read the diff*.

### Why the bound comparisons moved

A parse-time comparison can only work when both sides are literals, because
`min: MIN_AGE` and `min: 2 * N` are legal and the macro cannot evaluate them. A
`const {}` assertion has no such limit — rustc evaluates the expression.
Verified:

```rust
const { assert!((MIN_AGE as f64) <= ((2 * N) as f64), "min must not exceed max") }
```

- named consts and const arithmetic: **work**
- mixed int and float literals, via `as f64` on both sides: **work**
- a violated bound: `E0080: evaluation panicked: min must not exceed max`
- a *runtime* expression: `E0015: cannot call non-const function` — rejected,
  which is arguably correct, since a bound that is not a compile-time constant
  cannot be checked at all

So emit these alongside the step-6 asserts, in the same generated fn:

```rust
const { assert!((#min as f64) <= (#max as f64), "min must not exceed max") }
const { assert!(#min_length <= #max_length, "min_length must not exceed max_length") }
```

The `as f64` on both sides is what lets `min: 3, max: 120.5` compare at all.
Lengths are both `usize`, so they need no cast.

**The one thing lost:** a const panic message takes a `&'static str` and nothing
else, so it cannot say *"min (10) exceeds max (5)"*. If the fixed sentence proves
annoying in practice, layer a parse-time comparison on top for the
both-are-literals case purely to get the better message — but do the const
assert first, because it is the one that is actually complete.

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

## A cleanup worth doing, independent of all of the above

`FormAttribute`'s duplicate tracking can collapse the way `FieldBody`'s did, and
it needs no macro table — the enum is not the obstacle it looks like, because
**`Entry::name()` is already the variant-to-string map**. It returns `"title"`,
`"label_case"` and so on for attributes, and `path.key()` for a field.

So the five `bool` fields and both twelve-arm matches inside
`check_seen_and_set` reduce to one line:

```rust
struct FormAttribute { seen: HashSet<String> }

// `insert` returning false IS the duplicate check.
if !self.seen.insert(entry.name()) { /* the existing message branch */ }
```

`check_duplicate` still branches on `Entry::Field` for its message and span,
which is already there and stays. This also removes the
`#[allow(clippy::struct_excessive_bools)]` on `FormAttribute`.

**Why it is worth it:** adding a form-level keyword today touches the `kw`
module, an `Entry` variant, a `parse_*` fn, the dispatch chain, a
`FormAttribute` field, **two** arms in `check_seen_and_set`, an arm in `name()`,
and an arm in `expand`. Afterwards it touches five, and four of those five are
exhaustive matches that rustc forces you to fill. Only the dispatch chain is
left unguarded, and any test of the new keyword catches that.

A `macro_rules!` table over the entries was the other option and is not worth
it: unlike `FieldBody`'s keys, each form-level entry parses *different syntax*,
not merely a different type, so the bespoke `parse_*` fns stay either way and
the table would only generate what this does for free.

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
