---
name: error-model-design
description: "2026-09-21 design conversation on formoxus's error model, BUILT 2026-09-30 except per-field validators (C4) and ErrorsStore. The gap: no custom check can attach its message to a field, because FormError carries no path. Decisions: merge the two message types, keep storage POSITIONAL, put the path on the PRODUCER where T is still in scope. ErrorsStore (renamed from ErrorsByPath 2026-09-29) is agreed-in-principle but deliberately deferred — it is a performance change whose payoff arrives with live validation"
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

**Named `ValidationError<T>`, settled 2026-09-30** — this section originally
proposed `Verdict`. Todd floated `FxError<T>`; both rejected, for different
reasons. `Verdict` implies the type could also carry a PASS, when it only ever
exists for a failure. `FxError` stutters: Rust types are already namespaced by
crate path, so `formoxus::FxError` says formoxus twice (the API guidelines warn
against exactly this, and std writes `io::Error`, not `IoError`), and it drags
the `fx-` CSS prefix — which exists only because class names share one global
namespace — into the Rust API. `ValidationError` is Django's own name for the
same thing (`raise ValidationError`, `add_error(field, …)`), and Django is
where formoxus's Field/Widget split came from. Checked clear of `dioxus`,
`facet`, `googletest`, `indexmap` and `serde`. A bare `Error` was worse still:
by convention `formoxus::Error` would be the crate's `Result` error, and that is
`FormAccessError`'s job.

**DECIDED 2026-09-30: two types, and the message is shared.**

- **`ValidationError<T> { path: Option<Path<T>>, message: ValidationMessage }`** —
  the PRODUCER type, what a validator returns. Generic, never serialized, never
  stored.
- **`ValidationMessage`** — the STORED type. `FieldError` and `FormError` MERGE
  into it: one non-generic newtype over `String`, held by `FormField.errors`,
  `FormState.errors` and the wire.

Why the message field is a `ValidationMessage` and not a `String` (Todd's
question, and the sketch below originally said `String` for no reason):

1. **Routing becomes a move.** A `ValidationError<T>` is literally a message
   plus where it goes; routing splits it, the path picks the destination, and
   the message lands in storage untouched.
2. **It survives messages growing.** Translatable messages (Django's
   `error_messages`, keyed by an error code) are a recorded parity gap. If
   `ValidationMessage` ever gains a code or parameters, they reach the wire and
   the client for free. A `String` inside `ValidationError` would drop them at
   the one place every error is born.
3. **It forces the merge.** If `FieldError`/`FormError` survived, neither could
   be the message inside `ValidationError<T>` — one error may land on a field OR
   on the form, and which is unknown until routing. Only a single type fits.
   Per-field validators (C4) point the same way: their path is implicit, so they
   return `Vec<ValidationMessage>`.

Ergonomics are unchanged: `ValidationMessage: From<&str> + From<String>`, and the
constructors take `impl Into<ValidationMessage>`.

**The wire format does not change.** A single-field tuple struct over a `String`
serializes as the bare string, so `FormErrors` is byte-identical before and after
the merge.

**`ValidationError<T>` never crosses the wire, and could not.** `Path<T>` holds a
`&'static str`, which deserialization cannot produce from arriving bytes. It does
not need to: every error is converted exactly ONCE, at routing, in whichever
process ran `validate` (`Submission::accept` runs the same `FormState::validate`
on the server). Todd's worry that `ValidationError<T>` would need converting in
both directions dissolves on that — the client only ever receives stored
messages and re-applies them.

**The client/server container Todd was looking for already exists: `FormErrors`**
(`{ form, fields }`, serde, returned by `Submission::accept`, carried by
`WireForm`, consumed by `Form::apply_errors`). Its `fields` map is the plain-data
counterpart of `ValuesByPath`, so the `FieldErrors` TYPE ALIAS
(`form.rs`, `IndexMap<String, Vec<FieldError>>`) becomes **`ErrorsByPath`**
(`IndexMap<String, Vec<ValidationMessage>>`). That also ends its clash with the
`FieldErrors` COMPONENT in `widgets/errors.rs`, which keeps its name.

**Decision 2, still open:** what routing does with a path that `path!` accepts
but that is not live — a field inside an unchosen variant. `push_field_error`
returns `Err(FormAccessError)` there. `Submission` panics on an unknown path, but
`validate` also runs in wasm, where a panic is not recoverable. Recommended:
fall back to a form-level message, so the error is shown rather than lost.

**Derive trap:** `ValidationError<T>` should hand-write `Clone`/`Debug`/
`PartialEq` like `Path<T>` does (`path.rs`). A derive adds `T: Clone`-style
bounds because it cannot tell `T` only appears inside `Path<T>`.

**Name it for the thing, not the place.** `FormError` reads badly on a field ("a
form error on the email input") and `FieldError` reads badly on the form.
`ValidationError` was the candidate.

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
pub struct ValidationError<T> { path: Option<Path<T>>, message: ValidationMessage }
impl<T> ValidationError<T> {
    pub fn form(message: impl Into<ValidationMessage>) -> Self;
    pub fn at(path: Path<T>, message: impl Into<ValidationMessage>) -> Self;
}

fn passwords_match(c: &Credentials) -> Vec<ValidationError<Credentials>> {
    vec![ValidationError::at(path!(Credentials.confirm_password), "Passwords don't match.")]
}
```

`FormState::validate` routes: an error with a path goes through the existing
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
pub validator: Option<fn(&dyn Any) -> Vec<ValidationError>>,
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

## ErrorsStore — AGREED IN PRINCIPLE, DELIBERATELY DEFERRED

**Called `ErrorsByPath` when this was written.** Renamed here on 2026-09-29,
when Todd split the values pair for the same reason: `ValuesByPath` became the
plain `HashMap`/`IndexMap` of raw values and `ValuesStore` became the
`Store<…>` handle, because he had forgotten three separate times that
`ValuesByPath` was a reactive store rather than data. The errors projection is
the exact parallel — a `Store`, not a map — so under that convention it is
`ErrorsStore`, and the name `ErrorsByPath` is free to mean the plain data if it
is ever wanted. **Do not reintroduce `ErrorsByPath` for the store.**

Todd's observation: errors distributed by path parallel the values store.

**The real asymmetry it fixes.** `FieldProps` carries `errors` as a prop, and
those props are computed in `Form::render`, which does `self.state.read()` — a
subscription to the WHOLE `FormState`. So one pushed error re-renders every
field, which is precisely the failure mode the values store exists to prevent. A
widget reads its value reactively from a store but receives its errors as a prop
from a whole-tree read: two reactive inputs, two mechanisms.

**The obstacle.** `FormState::validate` runs on the SERVER inside
`Submission::accept`, where there is no Dioxus runtime and no `Store`.
`FormState` is deliberately plain data and that property is what makes the whole
server side work. So a store can NOT be the only home for errors.

**The resolution that is not a drift hazard.** `FormState` stays the authority;
`ErrorsStore` is a client-side PROJECTION with exactly one writer.
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
