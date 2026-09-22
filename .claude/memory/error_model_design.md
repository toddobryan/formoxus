---
name: error-model-design
description: "2026-09-21 design conversation on formoxus's error model, NOTHING BUILT. The gap: no custom check can attach its message to a field, because FormError carries no path. Decisions: merge the two message types, keep storage POSITIONAL, put the path on the PRODUCER where T is still in scope. ErrorsByPath is agreed-in-principle but deliberately deferred — it is a performance change whose payoff arrives with live validation"
metadata:
  type: project
---

## The gap that started it

`FormSpec.validator` is form-level and returns `Vec<FormError>`, and `FormError`
carries **no path**. So there is currently no way to write a custom check whose
message renders next to the field it concerns. "That ISBN checksum is wrong" can
only appear in the form-level list at the bottom.

`Form::push_field_error` is the only workaround and it sits outside the validate
cycle, so the next `validate()` clears it.

## Decided: merge the message types

Today `FormError` and `FieldError` are both `pub struct X(pub String)` and the
*only* difference is which list they are stored in. That is a weak basis for two
types. (`FormAccessError` is genuinely different — a caller bug, not a validation
result — and stays.)

**Name it for the thing, not the place.** `FormError` reads badly on a field ("a
form error on the email input") and `FieldError` reads badly on the form.
`Verdict` was the candidate.

## Decided: storage stays POSITIONAL, the path goes on the PRODUCER

Todd proposed `Option<Path>` on the stored error. Rejected, for two reasons:

1. **A member already IS its path.** `collect_errors` pairs each member's errors
   with its position in the tree. A stored path is either always `None` at rest
   (dead weight) or populated and able to disagree with where the error actually
   lives (drift hazard), and neither buys anything.
2. **`Path<T>` is generic.** A stored error cannot carry `Path<T>` without making
   `FieldError` generic, which infects `FormField.errors`, `FormState.errors`,
   `FormErrors` and serde. In practice it would carry a `String` — exactly the
   untyped thing [[path-macro]] exists to eliminate.

The shape that keeps the win — the path lives where `T` is still in scope, so it
can be a real `Path<T>`:

```rust
pub struct Verdict<T> { path: Option<Path<T>>, message: String }
impl<T> Verdict<T> {
    pub fn form(message: impl Into<String>) -> Self;
    pub fn at(path: Path<T>, message: impl Into<String>) -> Self;
}

fn passwords_match(c: &Credentials) -> Vec<Verdict<Credentials>> {
    vec![Verdict::at(path!(Credentials.confirm_password), "Passwords don't match.")]
}
```

`FormState::validate` routes: a verdict with a path goes through the existing
`push_field_error`, one without lands in `self.errors`. The path erases to a
string exactly once, at routing, and is never stored redundantly.

This is also the first place a validator gets compile-checked paths.

## Per-field validators

Same conversation, same mechanism. `FieldSpec` is type-erased (one struct for
every field) so `fn(&T)` cannot live there — the answer is the one
`WidgetType::Custom` already uses: an **`fn` pointer, not `Box<dyn Fn>`** (a
boxed closure supplies none of `Clone + Debug + PartialEq`), with `form!`
emitting a non-capturing closure that downcasts:

```rust
pub validator: Option<fn(&dyn Any) -> Vec<Verdict>>,
// emitted for  age => { validator: must_be_even }
.with_field_validator("age", |v| must_be_even(v.downcast_ref().expect("…")))
```

`FormField<T>` already bounds `T: 'static`, so `&T as &dyn Any` is free.

**The `expect` can be made unreachable.** Emit `let _ = must_be_even(&__s.age);`
into the witness and rustc checks that the validator's parameter type matches the
field's — wiring the wrong function to the wrong field becomes a compile error
rather than a downcast panic.

Use the keyword `validator` at BOTH levels; context disambiguates (a top-level
entry vs. one inside a field body) and two words for one concept reads worse.

## ErrorsByPath — AGREED IN PRINCIPLE, DELIBERATELY DEFERRED

Todd's observation: errors distributed by path parallel `ValuesByPath`.

**The real asymmetry it fixes.** `FieldProps` carries `errors` as a prop, and
those props are computed in `Form::render`, which does `self.state.read()` — a
subscription to the WHOLE `FormState`. So one pushed error re-renders every
field, which is precisely the failure mode `ValuesByPath` exists to prevent. A
widget reads its value reactively from a store but receives its errors as a prop
from a whole-tree read: two reactive inputs, two mechanisms.

**The obstacle.** `FormState::validate` runs on the SERVER inside
`Submission::accept`, where there is no Dioxus runtime and no `Store`.
`FormState` is deliberately plain data and that property is what makes the whole
server side work. So a store can NOT be the only home for errors.

**The resolution that is not a drift hazard.** `FormState` stays the authority;
`ErrorsByPath` is a client-side PROJECTION with exactly one writer.
`Form::validate` already does `state.write().validate()` and would push the
result into the store in the same operation; `push_field_error` and
`apply_errors` likewise. A projection with one writer is not a second source of
truth — which is the distinction that makes this safe where a duplicated path on
every stored error was not.

`FieldProps` then loses `errors`, and the widget boundary becomes
`(path, label, required)` plus two stores. That changes the boundary the design
notes fix, but makes it more consistent rather than less.

Form-level errors have no path: either leave them on `FormState` (they render
once, at form level, where a whole-form re-render costs nothing) or give them a
reserved `""` key, which is safe because no leaf path is ever empty.

**Why deferred:** it is a PERFORMANCE change, not a correctness one. Errors
change once per submit today, and a whole-form re-render on submit is invisible.
The payoff arrives with blur/live validation — at which point the current
arrangement would re-render every field in the form on each character typed into
one of them. Build it then, together with live validation, not before.
