---
name: facet-newtypes-and-custom-widgets
description: "Two things built 2026-09-18 to unblock the question editor: ControlType::Custom (the custom-widget escape hatch) and newtype-as-leaf support. Includes the probed facet API facts — a newtype reports scalar_type() = None and CANNOT parse_from_str, even with #[facet(transparent)]"
metadata:
  type: project
---

Both landed while trying to port `ui::MarkdownInput` to the reflection path.
The widget turned out to be the easy half; see "The blocker nobody predicted".

## `ControlType::Custom` — the escape hatch

`form2!`'s `custom(MyWidget)` had been **parsing and expanding since the macro
was written, with nothing to expand against**: `ControlType` had no `Custom`
variant and `ControlProps` did not exist, so any use of it failed to compile at
the call site. Now:

```rust
Custom { name: &'static str, render: fn(ControlProps) -> Element }
pub struct ControlProps { pub values: ValuesByPath, pub props: FieldProps }
```

- **`fn` pointer, NOT `Box<dyn Fn>`, and the derives force it.** `ControlType`
  is `Clone + Debug + PartialEq`; a boxed closure supplies none of the three. A
  non-capturing closure coerces to a plain `fn`, which is `Copy`.
- **`Debug`/`PartialEq` are hand-written** because deriving `PartialEq` over a
  fn pointer earns a real rustc warning: identical functions may be merged to
  one address and one function duplicated across codegen units, so the
  comparison is meaningless. `Custom` compares by `name` and prints as
  `custom(MarkdownWidget)` — `form2!`'s own spelling, so `ScalarInput`'s panic
  quotes back what the author wrote.
- The dispatch arm matches **any** `ValueKind` on purpose: a custom widget
  exists precisely because the built-ins can't serve its type.
- The widget goes inside `rsx!` rather than being called, so it gets a
  **component scope** and may use hooks — load-bearing, since the source picker
  needs `use_resource`. Pinned by a test that calls `use_hook`.
- `get_current`/`write_value` in `reflect/widgets.rs` are now `pub`: a custom
  widget lives in the consuming crate and needs the same store access the
  built-ins use.

## The blocker nobody predicted: newtypes were not leaves

`Markdown` is `struct Markdown(String)`. The shape walk dispatches on
`shape.scalar_type()`, so it fell through to `struct_member` and rendered as a
**fieldset containing an input named `text.0`** — and `FieldSet` never reads
`custom_control`, so `custom(MarkdownWidget)` would have silently done nothing.

**Probed facet facts, all verified by running them:**

| probe | result |
|---|---|
| `Plain(String)::SHAPE.scalar_type()` | `None` |
| `#[facet(transparent)] Transparent(String)` `.scalar_type()` | `None` — transparent does NOT help here |
| `shape.inner` | `Some(String)` for transparent, `None` for plain |
| `parse_from_str` on either newtype | **ERR** "Type does not support parsing from string" |
| `begin_nth_field(0)` → `set` → `end` → `build` | **works**, no attribute needed |
| `begin_nth_field(0)` → `parse_from_str` (inner vtable) | **works**, and rejects bad input |
| `Peek::into_struct()?.field(0)` | **works** for reading back |
| `StructKind` | distinguishes `TupleStruct` from `Struct` |

## The rule, and why it is narrower than "one field"

**A `TupleStruct` with exactly ONE field whose shape is a scalar.**
`newtype_inner(shape)` in `reflect/build.rs`.

A one-field NAMED struct (`struct Config { name: String }`) is deliberately NOT
flattened. Flattening would rename its leaf from `config.name` to `config` —
and **leaf paths ARE the wire format** now, the thing `Submission::accept`
rebuilds against ([[next_up_two_todos]]). The day someone adds a second field
the paths would silently change back and every stored or in-flight payload
would mean something different. A tuple struct cannot grow that way without the
author rewriting it as a named struct, which is a visible act.

**The field carries the INNER type, not the newtype.** `FormField<T>` needs a
compile-time `T`, and a concrete type is unrecoverable from a runtime
`&'static Shape` — `scalar_member`'s macro can only name a closed set. So a
`Markdown` field becomes a `FormField<String>` plus
`wrapper: Option<&'static Shape>`, and **only `write_value_into` consults it**
(`begin_nth_field(0)` → set → `end`). Everything string-facing — parsing,
display, `ValueKind`, control derivation — keeps working untouched.

No `#[facet(transparent)]` anywhere. Requiring it would mean a newtype in a
crate we don't control could never be a form field.

## Fallout

- `models` gained a `facet` dependency, and `Markdown` derives `Facet`.
- `ui::MarkdownWidget` is the reflection-path Markdown editor (textarea + live
  preview). No trait, so **no orphan-rule problem** — the derive path's
  `MarkdownInput` needed `FieldWidget`; a custom control is just a component.
- **`FieldProps` collides the same way `Form` does.** `formoxus::prelude::*`
  brings the DERIVE path's `FieldProps` (`label`/`required`/`placeholder`) into
  scope and it shadows the reflection path's
  (`path`/`label`/`required`/`errors`). `ui/markdown.rs` imports it as
  `ReflectFieldProps`. Same family of trap as `prelude::Form` vs
  `reflect::Form<T>` in [[facet_form_design_decisions]].

## Still open for the question editor

`Ref<T>` is **not** a newtype — it's a 2-field struct (`id: RecordId` plus
`PhantomData`), so this rule does not catch it, and the source picker needs its
own answer before any question form can exist (`source` is on `Question`
itself). Worth checking first whether `Ref` even derives `Facet`; it did not
appear to.
