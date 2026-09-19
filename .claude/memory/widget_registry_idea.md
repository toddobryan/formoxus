---
name: widget-registry-idea
description: "A design sketched 2026-09-19, paused mid-scoping (Todd stopped to extract formoxus into this standalone repo first): a type-keyed default-widget registry, so a 'clean' model crate that doesn't depend on formoxus can still get a custom widget by default, without every form re-declaring it. NOTHING BUILT — this is the write-up of a conversation, not a record of work done"
metadata:
  type: project
---

## The question that started it

Came up designing the question editor in apcsp-dioxus: `api` (server-fn
crate) can't depend on `ui` (widgets), but a `FormSpec` for a question form
needs to live in `api` (for `Submission::accept`) AND wants `Markdown`
fields to render with `ui::MarkdownWidget`. First answer on the table: split
the spec — `api::question_form()` owns model/title/labels/validator, `web`
layers `.with_custom_control(path, …)` on top for the Markdown fields.

**Todd reframed it, and the reframing is the useful part:** not "how does
*this app* route around its own crate graph" but "how would a project
depending on formoxus want to define clean models — no formoxus dependency
— that can still declare which custom inputs they want, without every
consumer of that model re-declaring the widget by hand at every form?" That's
a formoxus API question, not an apcsp-dioxus one — part of why formoxus got
extracted to its own repo right after this conversation.

## Why a plain trait impl doesn't work

The obvious shape: formoxus defines `trait CustomControl { fn control() ->
ControlType; }`, and whatever crate CAN see both the widget and the model
type (e.g. `ui`, which depends on both `formoxus` and `models`) does `impl
CustomControl for Markdown`. **This is a hard orphan-rule violation** — E0117
— because neither the trait (`formoxus`'s) nor the type (`models`'s) is
local to the crate writing the impl. Exactly the same wall `form2!`'s design
already hit for a different reason (see this repo's `facet_form_design_decisions.md`,
"form2! beats facet attributes" — orphan rule, solved there by generating
code in the CONSUMING crate instead of relying on a trait impl). A
cross-crate trait-based registry is a dead end here for the identical
reason.

## The workable shape: a runtime registry keyed by type identity

Not a trait impl — a **value** being built, so no orphan-rule problem.
Sketch:

```rust
// wherever both the model type and its widget are in scope (once):
registry.register::<Markdown>(MarkdownWidget::render);
```

- **Consulted as a fallback default, not a hard override.** `FormField::control()`
  (`formoxus/src/reflect/fields.rs`) already has the right shape for this —
  `self.custom_control.clone().unwrap_or_else(|| self.default_control())`.
  The registry slots in as a THIRD, middle tier: explicit per-field
  `custom_control` (set via `form2!`/`with_custom_control`) wins if present;
  else a registered default for the field's TYPE; else the existing
  scalar-kind `default_control()`.
- **The key is `wrapper.unwrap_or(T::SHAPE)`, not `T::SHAPE` alone.**
  Because of how newtypes-as-leaves works (`facet_newtypes_and_custom_widgets.md`):
  a `Markdown` field is actually a `FormField<String>` with `wrapper:
  Option<&'static Shape>` carrying `Markdown`'s own shape separately — the
  field's compile-time `T` is the INNER scalar, not the newtype. So the
  registry has to be consulted against `wrapper` first (the newtype, if
  there is one) and only fall back to `T::SHAPE` for a field with no
  wrapper — otherwise every `String` field would collide with whatever's
  registered for `Markdown`.
- **`&'static Shape` is a legitimate, ready-made registry key — verified,
  not assumed.** Checked `facet-core-0.46.5`'s actual source
  (`types/shape.rs`): `Shape` has real `PartialEq`/`Eq`/`Hash`, keyed on a
  stable `id` field (not raw pointer identity, not `Debug`-string hackery).
  So `HashMap<&'static Shape, ControlType>` (or wrapping value) just works —
  no typetag-style name registration, no per-scalar-type boilerplate to
  keep in sync. This is the same identity mechanism `ListSet`/`VariantSet`
  already lean on (`shape: &'static Shape` fields), just reused for a new
  purpose.
- **Reuses `ControlType::Custom` wholesale, no new variant needed.**
  `Custom { name: &'static str, render: fn(ControlProps) -> Element }`
  already exists (`facet_newtypes_and_custom_widgets.md`) — registration
  just produces one of these to stash in the map.

## What this buys, concretely

In the apcsp-dioxus case that started this: `api::question_form()` stays
**entirely** clean — no custom controls named anywhere in `api` — and `web`
never has to remember `.with_custom_control("data.$TrueFalse.text", …)` at
every call site either, since `Markdown`'s widget is registered once
(wherever `ui` gets initialized) and just applies everywhere that type
appears, forever. See apcsp-dioxus's `question_editor_plan.md` — its "Open
decision" section now points back here.

## Open, unresolved when the session paused: how the registry reaches `control()`

`FormField::control()` currently takes no external parameters — it's a
plain method consulting only `self`. Two ways to get a registry to it,
genuinely undecided:

1. **Global `OnceLock<Mutex<HashMap<&'static Shape, ControlType>>>`,
   populated by `.register::<T>()` calls at app startup** (e.g. in `web`'s
   `main.rs` before `launch!`). `control()` just consults it directly — zero
   signature changes to `render()`, `RenderCtx`, or any of `Form<T>`'s
   several methods that build a fresh `RenderCtx::root(...)`
   (`render_fragment`/`render_title`/`render_fields`/`render_errors` all do
   this independently, which is what makes option 2 costly). Only ever
   touched client-side in practice — the server rebuilds `FormState`
   (`Submission::accept`) but never calls `.render()` on it, so the
   registry can simply be unpopulated/irrelevant server-side. `Mutex` (not
   `RefCell`) so it compiles for both the wasm client (single-threaded
   anyway, lock is uncontended) and a multi-threaded server target without
   a `#[cfg]` split — even though the server path never actually touches it.
2. **Threaded through `RenderCtx`, like `Provider<C>`.** More explicit, and
   matches the stated design principle behind `Provider` — "supplied at the
   render call site rather than baked into the form's declaration." But
   `RenderCtx` gets rebuilt fresh at the top of EVERY one of `Form<T>`'s
   render-family methods, not just `render()`, so this means adding a
   parameter to all of them, not one.

**Leaning toward option 1** for the ergonomics (genuinely zero call-site
disruption) and because the server-side irrelevance removes the biggest
objection to a global (the CLAUDE.md-style "no global DB" concern doesn't
really apply — this is populate-once, read-only after that, no
connection/session state, no test-isolation risk beyond "don't register two
different widgets for the same type across tests in one binary," which is a
much smaller hazard class). Was about to ask Todd to confirm this when the
session pivoted to doing the extraction first — **ask again before
building.**

## Where this connects to other open threads

- [[next_up_two_todos]]'s deferred `Path`/`Prefix` newtype — unrelated
  mechanically, but both are "we now have enough real usage to justify a
  small type instead of a bare `String`/free functions" moments; worth
  doing them as separate passes, not combined.
- The registry, if it lands, is a stronger answer than `next_up_two_todos`'s
  planned `Form::apply_errors(FormErrors)` promotion trigger for a
  *different* problem (that one's about server-error wire shape, this one's
  about default widget selection) — don't conflate them when picking either
  back up.

## UPDATE 2026-09-19: the open fork is answered elsewhere

The "global `OnceLock` vs threaded through `RenderCtx`" question above is
superseded. It turned out to be one instance of a general problem — `novalidate`
and `label_case` want the same mechanism — and the answer is a third option
neither bullet considered: **Dioxus context**, provided at the app root and read
with `try_use_context` inside the leaf component, which is where rendering
already happens. It needs no signature changes (the objection to `RenderCtx`)
and is properly scoped with no static mutable state (the objection to the
global). See [[config-cascade]]; read it before picking this back up.
