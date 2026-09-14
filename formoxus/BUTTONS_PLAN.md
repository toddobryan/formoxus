# Buttons on the reflection path

Design settled 2026-09-14, not yet built. Sibling to `REFLECT_PLAN.md`, which
covers the shape walk; this covers what wraps it.

The reflection path renders fields and nothing else. `FormState::render`
(`src/reflect/form.rs`) emits title -> members -> errors as a bare fragment, the
view hand-writes the `<form>` element and every button, and `form2!` has no
button syntax at all. `crates/web/src/views/auth/login.rs` is the whole of the
evidence: a `<form>` with an `onsubmit`, a hand-written submit button, and a
`div.formoxus-buttons` wrapper copied from what the derive macro emits.

Three decisions below. The first two are settled. The third is settled in
substance with its spelling still open.

## 1. `render` owns the `<form>` element

Todd's call: it makes the common case clean, and a view that needs something
formoxus can't express can always drop to manual rendering.

Consequence worth planning for: **`render()` currently returns the fragment, and
41 call sites assert on it** (`src/reflect/tests/`, `tests/reflect.rs`, plus the
web views). Changing what `render()` emits moves all of them.

Cheapest shape that gets the decision without the churn — two methods:

- `render()` — emits `<form>`, the fields, and the button row. The common case.
- a second method (`render_fields()`, or whatever reads best) — today's fragment,
  for a view that wants to place things itself.

The escape hatch then exists as an API rather than as a thing you're expected to
infer, and the existing tests re-point at the second method instead of being
rewritten.

`login.rs`'s role-picker `dialog` is already a sibling of the `<form>` rather
than a child, so that page needs no restructuring.

### What owning the element buys

The submit button stops needing an `onclick`. Native `type="submit"` already
routes both a click on it and Enter-in-a-field through the form's `onsubmit`, so
one handler covers both. That is the fork `ButtonInfo::button_tokens`
(`../formoxus-macros/src/form_meta.rs`) already turns on, and the reflection path
can only take the same branch if it emits the `<form>` itself.

## 2. Buttons are declared in `form2!`; handlers are supplied at `render`

`form2!` grows a `buttons:` block carrying **static knowledge only** — which
buttons exist, their text, class, display order, and whether each is validated or
unchecked. Behavior is supplied where the form is rendered.

This looks like a free choice and is not. Three things rule out putting the
closure in `form2!`:

**The obvious objection is the weakest one.** "A handler needs things declared in
the component" is true of `login.rs` today, but it is solvable by moving the
`form2!` invocation out of the free fn (`fn login_form() -> FormSpec<LoginForm>`)
and into the component body, where props and hooks are in scope. So it does not
decide anything on its own.

**The thunk runs exactly once.** `use_form` takes `impl FnOnce() -> FormState<T>`
and hands it to `use_signal`, which runs it on first render and never again —
that is the whole point of the change that introduced it. A closure built in
there captures first-render values. `Copy` reactive handles (`Signal`,
`Navigator`, `Form`) survive that; a plain prop does not. `login.rs` already
launders its `next: Option<String>` prop through a `use_signal` for exactly this
reason. Handlers supplied at render are rebuilt every render and cannot go stale.

**A handler that needs the form handle cannot capture it.** `login.rs`'s submit
handler calls `form.push_error(...)` for the server verdicts ("invalid
credentials", "inactive account"). `form` is what `use_form` *returns from* the
thunk, so a closure written inside that thunk cannot capture it. Passing
`Form<T>` into the handler instead (`Fn(Form<T>, T) -> impl Future`) does break
the cycle, but choosing that signature in order to rescue declaration-time
handlers is solving the wrong problem.

**Cost even if all three were solved:** `form2!` is invoked outside any component
in roughly fifteen test fns in `tests/reflect.rs`, with no Dioxus runtime. Bake
closures into `FormSpec` and that stops compiling — the spec stops being a plain,
testable value.

The principle is already written down, in the `Provider<C>` doc comment in
`src/form.rs`:

> …supplied at the `store.render(...)` call site rather than baked into the
> declaration, the same use-site-not-declaration-site split `Handler` already
> gives buttons.

## 3. One hidden slot struct, built by one macro

This replaces the derive path's two generated structs. Todd's two objections to
those, both of which the design has to answer:

1. **A name just appears.** `TrueFalseFormHandlers` and `TrueFalseFormProviders`
   are conventions the author has to know, and they can collide. Worse than
   cosmetic: the generated structs inherit the container's visibility
   (`#vis struct #providers_ident` in `../formoxus-macros/src/container.rs`), so
   a `pub` form puts **two `pub` names into the crate's public API** that nobody
   asked for.
2. **More verbose than it has to be.** `handler(...)` / `unchecked_handler(...)`
   wrap every closure, and nested providers need their own literals.

### Why the struct itself stays

A struct literal is Rust's only free "supply every one of these, by name, in any
order" check. Forget `cancel` and you get `missing field 'cancel' in
initializer`. Anything that drops the struct has to buy that property back.

Both objections are about **naming**, not about the struct — so keep it and fix
the naming.

### The design

**Merge handlers and providers into one slot struct per form.** One set of slots:
button slots typed `Handler<M>` or `UncheckedHandler`, provider slots typed
`Provider<C>`. This deletes `render_with_providers` and the `Self::Providers:
Default` bound on `render` (`src/form.rs`) — that bound exists only so the second
argument can be omitted.

**Hide the name.** Mangle the ident, `#[doc(hidden)]` it, never mention it in
docs. It stays reachable as `<T as FormState>::Handlers`, which is how the
library already refers to it — the associated types are already there, and the
only reason anyone writes the concrete name is that Rust has no anonymous struct
literal.

**Build it with one macro**, named per the open question below:

```rust
form.render(supply! {
    sign_in: |m| async move { … },
    cancel:  || async move { … },
    common.source: source_provider,
})
```

- Dotted paths flatten nested provider groups, the same way `[]` row selectors
  already flatten spec paths. No inner `CommonProviders { … }` literal.
- A missing slot is still `missing field 'cancel'`, pointing at the macro call.
- Compare the call site this replaces, in
  `crates/web/src/views/questions/edit.rs`: two struct names, a nested third, and
  a wrapper call per closure.

**One macro, not one per form.** The macro should not need to know which slot is
validated, unchecked, or a provider. Add a trait to formoxus:

```rust
trait IntoSlot<T> { fn into_slot(self) -> T; }
```

with blanket impls for each closure shape. The macro emits
`IntoSlot::into_slot(expr)` for every value and **the field's own type selects the
conversion** — a type the derive already chose from the button's `HandlerKind`.
The author writes a bare closure and never names a wrapper.

`impl From<F> for Handler<M>` will not work directly: `Handler<M>` is
`Rc<dyn Fn…>`, foreign on both sides, so coherence rejects it. A local trait
sidesteps that.

> **Unproven.** Whether inference actually resolves `T` from the field type in a
> struct-literal position needs a compile test before this is committed to. If it
> does not, the fallback is for `form2!` to generate a per-form macro that knows
> each slot's kind — which works, but costs a macro name per form and so gives
> back part of objection 1.

### Rejected: a builder

```rust
form.render().sign_in(|m| …).cancel(|| …).finish()
```

Methods take `impl Fn` directly, so no wrappers, and it adds **zero** type names
to the user's namespace — the strongest possible answer to objection 1.

It loses on the completeness check. `finish()` may only exist once every slot is
filled, which means typestate: a marker type parameter per slot, or a
const-generic bitmask (that one wants `generic_const_exprs`, still unstable).
Those parameters then surface in every error the author sees — a
`RenderBuilder<LoginForm, Set, Unset>` mismatch in place of a named missing
field.

Struct-plus-macro gets the builder's ergonomics with the struct's error messages.

## Open: the macro's name

`wire!` was proposed and rejected. Two candidates that come from the codebase's
own vocabulary rather than from nowhere:

- **`supply!`** — the verb already used in the prose this design comes from
  ("supplied at the `store.render(...)` call site"). Names the act.
- **`slots!`** — `slot` is already the code's noun for these fields
  (`FieldMeta::providers_slot`, "a provided-data slot" in `container.rs`). Names
  the destination.

Used `supply!` in the examples above as a placeholder. Still Todd's call.

## Build order

1. Split `render()` / fields-only, move the 41 call sites, no behavior change.
2. `render()` emits the `<form>`, still with no buttons.
3. `IntoSlot` plus the compile test that decides whether one generic macro is
   possible.
4. The slot struct and the macro, on the **derive** path first — it already has
   buttons and providers, so it is the real test, and the two generated structs
   disappear from the public API.
5. `buttons:` in `form2!`, and the reflection path's button row.
