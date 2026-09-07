# Plan: Form as schema, Store as live values

Decided 2026-09-05 against `ac09b6c` (74 tests green), after prototyping.
Supersedes two earlier drafts of this file; what they got wrong is recorded
under "Roads not taken", because that reasoning is the expensive part.
Merged into `crates/formoxus` on 2026-09-06 as the `reflect` module; this file
came along because its open items and its "Roads not taken" reasoning are still
live. It dies when the reflection path is either finished or abandoned.

## The architecture

Two things, with clearly separated jobs:

| | holds | changes | reactivity |
|---|---|---|---|
| `Form<T>` | members, structure, typed values, errors | rarely — only on a **structural** edit | `Signal<Form<T>>`, coarse |
| `Store<HashMap<String, String>>` | live edited raw strings, keyed by qualified path | on every keystroke | per-path, fine-grained |

`Form<T>` is **unchanged** — plain data, `FieldValue::Valid(T)` still typed,
serializable, testable with no Dioxus runtime. Signals stay at the widget
boundary; a `Store` wraps plain data rather than putting signals inside it.

**This is what makes the split work:** typing and structural change are
different operations, on different clocks. Typing is a store write that dirties
one path. Adding a row or switching a variant is a schema rebuild — discrete,
rare, and already the collect → rebuild → re-apply move.

### Verified, not assumed

Prototyped in `scratchpad/storeproto` (3 tests, all passing):

1. `Store<HashMap<String,String>>::get(path)` yields a child store addressed by a
   **runtime** string. The original objection — `#[derive(Store)]` generates
   *named* accessors, impossible for `Vec<Box<dyn FormMember>>` — was about the
   derive, not the store.
2. Inputs bind to per-path child stores (`value` + `oninput`).
3. **Writing one path re-renders only that leaf.** This is the objection that
   sent us uncontrolled ("we'd have to recreate the entire struct every time any
   input changes"). It cannot be passing by accident: the child props are
   identical across the write, so prop memoization would suppress *every*
   re-render — only the store subscription can mark that one leaf dirty.

Also verified: the whole crate, facet reflection included, compiles for
`wasm32-unknown-unknown`.

### Lifecycle

```
build      form_for(&model)  or  blank_form::<T>()          -> Form<T>
seed       form.leaves().into_iter().collect()              -> Store<HashMap<..>>
type       values.get(path).set(s)                          // one leaf re-renders
submit     form.apply(&values.read()); form.validate()      -> Option<T>
```

**Structural edits keep the value map.** It is keyed by path, and paths are
stable under growth — row *n*'s paths never move when *n+1* appears. So adding a
row inserts new keys and leaves existing values untouched. Switching a variant
rebuilds the subtree under a *different* `$Variant` prefix, so the old keys stay
put and stay inert; see the retired destructive-switch section below.

### Destructive-switch warning — RETIRED 2026-09-06

Two earlier answers, both now moot. The DOM-scan existed only because the
in-memory `Form` was stale under the uncontrolled design. Its replacement — "does
any key under this prefix have a non-empty value, and wipe the subtree on switch"
— assumed switching a variant destroys work.

**It no longer does.** A `VariantSet` namespaces its children under a `$Variant`
descriptor segment, so `Circle` and `Square` own `footprint.$Circle.size` and
`footprint.$Square.size` rather than both claiming `footprint.size`. Switching
away leaves the old keys inert (nothing asks for those paths) and switching back
finds them intact. Nothing is lost, so there is nothing to warn about and nothing
to wipe.

That started as a collision bug, not a UX wish: with a shared field name, the
value map — which survives structural edits *by design* — handed the Circle's
number straight to the Square, silently. `switching_variants_does_not_inherit_a_
same_named_field` is the regression net.

`$` cannot begin a Rust identifier, so a descriptor can never collide with a
field name. Strip the `$` segments and a leaf path is the model path again;
`model_path()` does exactly that, so the rule is executable rather than a comment.
Note this is NOT the rejected `$variant`-marker idea below: that one made the
variant a submitted *value* to be parsed back, and every objection to it was about
parsing names *in*. A descriptor segment is output only — the `Form` still owns
the choice, and nothing ever reads it back out of a name.

## What still has to be built

1. **`blank_form::<T>()` with a default of 1 row per list**, not 0. Zero rows
   gives the user nothing to type into and no hint the list is there.
2. **Rebuild-with-changed-structure**, preserving the value map: add a row,
   remove a row, switch a variant. One entry point, since all three are the same
   operation on the schema.
3. **An `Unchosen` variant state**, probably. With a reactive select, choosing is
   something the user does *in* the form, so `blank_form` wants to be infallible
   and start with nothing chosen. That means `VariantChoice` grows a third state
   distinct from `Absent` ("deliberately none"), and `validate()` reports an
   unchosen variant as an error like any other. **If that lands, `MissingVariants`
   / `VariantOptions` / `missing_variants` / `required_variants` and the whole
   iterative disclosure loop retire** — along with the "pre-flight empty ⟺
   construction succeeds" invariant that has bitten us twice.
4. **The widget layer**: per-path child stores, the reactive `<select>`, and
   add/remove row buttons.
5. **`Question`** — the motivating case, and the first real test of all of it.

Row counts and variants are back to being properties of the schema. That is fine
now in a way it wasn't before: changing them is cheap and lossless, because the
value map survives.

## Roads not taken

**Fully uncontrolled, DOM as the source of truth.** What we had. Values live in
the browser and come back via `FormData::values()` on submit. Gives up live
validation, conditional fields, and reading state mid-edit. Still *works* — the
existing tests keep it honest — but the store version is strictly more capable at
no cost to the data model, and the moment the variant select became reactive the
"no signals at all" premise was already spent.

**Recovering structure from submitted names** (`answers.0`, `answers.2` → count
the indices; `data.$variant` → the variant). Genuinely workable, and the naming
convention really is lossless. Dropped because with values in a store, the DOM is
no longer the source of truth for *anything*: the Form owns structure, the store
owns values, and neither needs to be reconstructed from name parsing. It also
made reading fallible in a new way (unknown variant, missing marker, malformed
path — failures with no field to attach to) and turned submitted names into a
trust boundary. Worth remembering if a non-Dioxus/plain-HTML target ever matters,
since it needs no client-side state at all.

**Raw-canonical `FieldValue::Valid(String)`.** Considered as the prerequisite for
lensing *into* `Form` with `SelectorScope::hash_child`. Unnecessary — the values
live beside the Form, not inside it, so no lens into `Form` is needed. Also
actively worse: `Valid(T)` keeps `form_for(&m).validate() == Some(m)` true **by
construction**, never passing through a string. Raw-canonical would convert that
structural guarantee into a dependency on `T -> String -> T` fidelity, which is
now pinned by `tests/roundtrip.rs` for built-in scalars but can never be enforced
for a user's custom type.

## Open questions

1. **Where does the value map live relative to the `Form`?** Two hooks side by
   side, or one wrapper type owning both? A wrapper can keep them from drifting
   (a rebuilt schema with a stale map is the obvious bug) at the cost of putting
   a `Store` next to plain data.
2. **Does the map keep stale keys after a shrink?** Removing a row leaves
   `answers.2.*` behind. Harmless for `apply` (unknown keys are ignored) but it
   leaks, and it makes "is anything under this prefix non-empty?" answer wrong.
   Probably prune on rebuild.
3. **Server-side rendering / no-JS.** The uncontrolled path needed no client
   state; this one does. Does that matter for this app? Probably not, but it is
   the thing the road-not-taken buys.
4. **Does `Form<T>` still need `T`?** Yes — `Partial::alloc::<T>()` and
   `materialize::<T>()` still need a concrete type.
