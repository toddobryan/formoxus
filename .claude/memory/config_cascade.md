---
name: config-cascade
description: "2026-09-19 — three separate questions (novalidate, label_case, the widget registry) turn out to want ONE mechanism: an app-level default that a form, then a field, can override. Direction agreed with Todd: Dioxus CONTEXT, not a global OnceLock and not threading through RenderCtx. Supersedes the open fork in widget-registry-idea. BUILT as of 2026-09-26 (defaults.rs + provide_defaults + form! tier); two label_case leaks remain"
metadata:
  type: project
---

## BUILT — status as of 2026-09-26

**The mechanism shipped**, contrary to the "nothing built" above. `src/defaults.rs`
holds a `Formoxus` config struct (`label_case`, `use_browser_validation`),
`provide_defaults(Formoxus::new()…)` puts it in Dioxus context, and `defaults()`
reads it — the agreed design, via context rather than a OnceLock. `form!` carries
the per-form tier (`browser_validation: on|off`, `label_case: "…"`), and
`tests/suite/browser_validation.rs` pins all of it including an app default
reaching a silent form and a form overriding that default.

`LabelCase::Title` is no longer hardcoded at `members.rs:144`; that tier works.

**Both leaks FIXED 2026-09-26.** A textless button label and a `VariantSelect`'s
variant names now use the form's case.

The instructive part is what does NOT work: reading `defaults()` at the point of
use. `defaults()` is only the APP tier, so a form stating its own `label_case`
is still ignored — and the result is worse than the original bug, because within
one rendered form the field labels obey the form and the buttons do not.
`tests/suite/label_case.rs::a_textless_button_uses_the_forms_case` was written to
fail against exactly that, and does.

So both take the case as a parameter: `ButtonSpec::label(&self, case)` fed from
`Form::label_case()` (resolved ONCE per button row), and `VariantSelect` gets a
`label_case` prop fed from `ctx.label_case`. That also keeps `ButtonSpec` plain
data — `defaults()` needs a live Dioxus runtime and a `ButtonSpec` outlives any
render, which is the same reason the cascade is resolved on `Form` and not
`FormState`.

**The rule, for the next derived-text site:** resolve the cascade at the `Form`
boundary and hand the answer down. Never reach for `defaults()` from inside
something that renders.

The third customer named below, the **widget registry**, is still unbuilt — but
it no longer needs a design decision, only the work: it rides this mechanism.

## What forced the question

Todd noticed the gallery could not show its own error styling: submitting an
empty form does nothing, because the browser's constraint validation gates
submit before formoxus's `onsubmit` ever fires. formoxus renders the `<form>`
itself and sets **no `novalidate`**, while emitting `required` on required
inputs — so there are two validation systems and the browser wins.

The practical cost: formoxus's own error rendering is largely *unreachable*.
"This field is required." rarely renders, because the browser says "Please fill
out this field" first, as an unstylable tooltip in its own language and
position. A user meets two different error presentations depending on which
system caught the problem.

The fix is probably `novalidate` on the form while keeping the `required`
attributes (screen readers announce them, and they are semantically true). The
usual argument for native validation — the no-JS fallback — does not apply to a
wasm library. **But Todd's instinct was that this should be globally
configurable and per-form overridable, and that is the part worth recording.**

## The unification

Three questions that looked separate all want the same thing:

1. **`novalidate`** — app-wide policy, occasionally overridden for one form.
2. **`label_case`** — see below; formoxus currently has NO tier for it at all.
3. **The default-widget registry** — [[widget-registry-idea]], paused on
   exactly "how does an app-level default reach the render".

Answering them separately risks three different global mechanisms. **Decide the
cascade once.**

## What the competition does — checked, not remembered

`leptos_form` 0.2.0-rc1, read from the crate source and `docs/Form.md`:

- **Two tiers only: container (struct) → field.** Field attributes are
  documented as "falling back on the container default where needed".
- **`rename_all` is explicitly "only allowed at struct-level"** — casing is a
  whole-form consistency property, not a per-field one. Worth copying.
- **No global tier whatsoever.** No `provide_context`/`use_context`, no
  statics, no `thread_local`, no `OnceLock` anywhere in the crate. Its docs
  mention "inheriting styling from the create form", but that is a newtype
  wrapping the same struct so the derive re-runs over the same fields — not a
  config cascade.

So a global tier puts formoxus **ahead of** leptos_form rather than at parity.

**And formoxus is currently BEHIND it on label casing:** `LabelCase::Title` is
hard-coded at `members.rs:144`. The `label_case` container attribute was a
`#[derive(Form)]` feature and died with the derive path; `form!` never had one.
Zero tiers, not two.

## The mechanism: Dioxus context

[[widget-registry-idea]] framed the fork as global `OnceLock` vs threading
through `RenderCtx`, and leaned toward the global for ergonomics. **There is a
third option, and it is better than both.** Direction agreed with Todd
2026-09-19; the specifics below are not yet built or finally settled.

`provide_context` at the app root, `try_use_context` inside the leaf component
— which is exactly where rendering already happens, since formoxus spawns one
component per leaf ([[facet-form-spike]]).

Verified present in dioxus 0.7.10: `provide_context`, `provide_root_context`,
`use_context_provider`, `use_context`, `try_use_context`, `consume_context`,
`try_consume_context`.

Why it beats both earlier options:

- **vs. threading through `RenderCtx`** — that was costly because `RenderCtx`
  is rebuilt at the top of EVERY one of `Form`'s render-family methods
  (`render_fragment`/`render_title`/`render_fields`/`render_errors`), so it
  meant a parameter on all of them. Context needs no signature changes at all.
- **vs. a global `OnceLock`** — context is properly scoped, so two tests in one
  binary cannot collide, and "override for this subtree" is expressible. A
  `OnceLock` cannot do either. No static mutable state.
- **`try_use_context` returns `Option`**, which is what makes the bottom of the
  cascade safe: no provider means fall back to the built-in default. That
  matters for the server path, which rebuilds a `FormState` via
  `Submission::accept` but never renders, and so has no Dioxus runtime at all.

## The cascade

```
context default  ->  per-form (FormSpec)  ->  per-field (FieldSpec)
```

leptos_form's two tiers plus the global one it lacks. Not every setting takes
every tier — follow `rename_all`'s precedent and allow a setting only at the
levels where it is meaningful:

| setting | context | form | field |
|---|---|---|---|
| `novalidate` | yes | yes | n/a (it is a `<form>` attribute) |
| `label_case` | yes | yes | no — per-field casing is incoherent |
| default widget by type | yes | ? | already have `custom_widget` per field |

`FieldField::widget()` already has the right shape for the widget tier —
`self.custom_widget.clone().unwrap_or_else(|| self.default_widget())` — and
the registry slots in as the middle fallback, keyed on
`wrapper.unwrap_or(T::SHAPE)`, per [[widget-registry-idea]].

## Still open

- Whether the context carries one `FormoxusDefaults` struct or one context type
  per setting. One struct is fewer moving parts but makes every override
  restate the whole thing unless it is built with `..Default::default()`.
- Whether `FormSpec` grows a field per setting, or one `overrides:
  FormoxusDefaults`-shaped `Option`.
- `novalidate` is a behaviour change that reaches apcsp-dioxus — see
  [[repoint-at-formoxus-repo]], which is unstarted, so nothing breaks today.
- Nothing here is built. The gallery still cannot show its own error styling
  until either `novalidate` lands or an example uses a cross-field validator,
  which the browser cannot pre-empt.
