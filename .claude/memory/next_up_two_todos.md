---
name: next-up-two-todos
description: "Running queue on the facet branch (2026-09-16..18). The wire rework is FINISHED 2026-09-18 — option (a) leaves-over-the-wire, Submission<T> extracted, all three files migrated, cargo test --workspace green at 476. Remaining: 4 render_widget arms, and the Path/Prefix newtype (still deferred)"
metadata:
  type: project
---

## ▶ DONE 2026-09-18 — the wire rework is finished and `--workspace` is green

> **Verified end to end on 2026-09-18**, including `just e2e` clean. Getting
> that e2e run to complete took a detour: earlyoom kept killing VS Code windows
> mid-build — see [[oom_and_build_memory]], the cause was nowhere near the
> symptom.

All four steps landed; `cargo test --workspace` passes at **476 passed / 0
failed / 24 ignored**, the first green workspace since the `auth.rs` rewrite.
Option **(a)** was taken: the wire carries raw `(path, value)` pairs, the server
rebuilds the form from the same `FormSpec`, and `ChangePasswordProblem` is gone.

**`Submission<T>`** (`crates/formoxus/src/reflect/submission.rs`) is the
generalization that came out of it — Todd spotted that
`empty_form`/`apply`/`validate`/`collect_errors` would be boilerplate in every
server-side form handler. `Submission::accept(spec, values) -> Result<Self,
FormErrors>`, plus `model()`, `into_model()`, and `reject_field(path, msg)` /
`reject(msg)` which CONSUME self — so "don't call `validate` again after
pushing a server error" became a compile error instead of a comment. Panics on
an unknown path (a caller bug; a server panic unwinds to a 500, unlike wasm).
10 tests in `reflect/tests/submissions.rs`.

**Where each piece ended up:**
- `api/src/auth.rs` — `change_password` takes `HashMap<String, String>`,
  returns `ServerFnResult<Result<(), FormErrors>>`. `Result` rather than
  "empty `FormErrors` means success", because a cross-field failure has EMPTY
  `fields`. `perform_password_change` is one `Submission::accept` + two
  `reject_field`s; `FormState` no longer appears in the file.
- `web/src/views/auth/change_password.rs` — sends
  `form.values().read().clone()`; a new `show_server_errors` routes
  `FormErrors.fields` back through `form.push_field_error` and `.form` through
  `form.push_error`. An unplaceable path DEGRADES to a form-level error rather
  than being thrown or dropped (the user is still owed the message).
- `fixtures/tests/change_password.rs` — 6 tests, now the only end-to-end
  exercise of the wire shape. Todd added `FormState::as_hash_map()`
  (`leaves().into_iter().collect()`) which the test's `wire()` helper uses.
  Two NEW cases beyond the old matrix: a forged mismatch that skipped the
  client's validation, and a request omitting fields entirely.

**The mirror-image boilerplate is still open.** `show_server_errors` in the web
view is the client-side counterpart to `Submission` and will be identical in
every form. Deliberately left local: only one caller so far. When a second form
needs it, it becomes `Form::apply_errors(FormErrors)` in formoxus.

**Still open from before, untouched:** four unimplemented `render_widget` arms
(`SelectMultiple`, `CheckboxMultiple`, `RadioGroup`, `File`) and `Select`
beyond `Bool`; and the `Path`/`Prefix` newtype below, which gained a third
piece of evidence when `VariantSet::collect_errors` tripped over `my_path` vs
`child_prefix`.

---

Todd's own pick for the session after `9442235`, chosen over the bigger blocked
item (`change_password.rs`, which still needs a `push_field_error` API first —
see [[facet_form_design_decisions]]). Both surfaced while wiring buttons into
`login.rs`.

**Status as of 2026-09-16: both closed out**, `cargo test --workspace` green.
`form2!`'s `widgets!` table still accepts more widget names than
`render_widget` implements — that gap is not fully closed, just narrowed by
one. See "What's still open" below before picking this back up.

## 1. `Textarea` — done

`render_widget` (`crates/formoxus/src/reflect/widgets.rs`) had three arms
(`Input` family, `Checkbox`, `Select` on `Bool`); five names panicked at
render regardless of value kind. Added a fourth arm,
`(ValueKind::Text { .. }, WidgetType::Textarea)`, dispatching to a new
`TextareaInput` component — a straight copy of `HtmlInput`'s controlled-value
binding and label/error layout, minus the `InputType`-specific branches
(`Password` masking, bare `Hidden` markup) that don't apply to a `<textarea>`.
`textarea`'s `value` attribute is `volatile` in `dioxus-html` 0.7.10 just like
`input`'s, so the same `get_current`/`write_value`/`oninput` pattern carries
over unchanged.

Test: `a_widget_can_override_text_to_a_textarea` in
`crates/formoxus/tests/reflect.rs`, following the file's existing
`PasswordSecret`/`SelectedBool` pattern — asserts on `<textarea` and the
carried-over value directly, per the standing note that a render panic here is
survivable under SSR (a sibling field's markup keeps showing up while this one
silently fails), so the test can't just check the page rendered *something*.

**Still panic on render, whatever the value kind:** `SelectMultiple`,
`CheckboxMultiple`, `RadioGroup`, `File` — and `Select` still only works
against `Bool`. Same fix shape as `Textarea`: add the arm, don't shrink the
table (the `widgets!` doc comment in `formoxus-macros/src/form2.rs` argues
that case — the macro crate can't see `render_widget`'s arms, so gating the
table on them would be a worse sync hazard than the table being wider).

## 2. `form_demo.rs` migrated onto `buttons:` + `using_fns!` — done

`ReflectedForm` in `crates/web/src/views/form_demo.rs` now builds its spec
through `form2!` (a `demo_spec()` fn) instead of the imperative
`FormSpec::new().with_title(…).with_label(…)` builder, and declares
`buttons: { check: { type: button, invocation: unconditional } }` rather than
hand-writing a `<form onsubmit=prevent_default>` wrapper around
`render_fragment()` plus a bare `button`.

The one departure from `login.rs`'s worked example: `check` is declared
`invocation: unconditional` rather than left at `ButtonType::Button`'s default
(`IfModelValidates`). An `IfModelValidates` button's handler only runs — and
only ever sees the model — once validation already passed, which would have
silently dropped the demo's "does not validate (see field errors)" message on
an invalid submit. `unconditional` gets an `|| async move { … }` (arity-zero,
so `using_fns!` reads it as `Unchecked`) that calls `form.validate()` itself
and handles both outcomes, exactly reproducing the old onclick body.

Verified both by `cargo check -p web` under `--features web` and
`--features server`, and by starting `just serve-log` and curling
`/form-demo` (an unguarded route) — both the create (`empty_form`) and edit
(`form_for`) columns render the single `<button type="button" class="secondary">Check</button>`
inside `.formoxus-buttons`, with values populated identically to before the
migration.

## What's still open

Four more `render_widget` arms (`SelectMultiple`, `CheckboxMultiple`,
`RadioGroup`, `File`), and widening `Select` beyond `Bool`. Nothing currently
reaches for them — no page asks for a `radio_group` or `multiselect` widget —
so there's no test pressure driving which one to do next; whoever picks this
up should probably wait for a real caller rather than speculatively
implementing all four.

**`change_password.rs`'s migration is UNBLOCKED and underway as of
2026-09-16** — `push_field_error` landed (client-side, on `Form`/`FormState`),
`crates/api/src/auth.rs`'s `perform_password_change` is rewritten off the old
derive path, and `crates/formoxus/src/reflect/form.rs` got split into
`form/{spec,state}.rs` along the way. See [[facet_form_design_decisions]]'s
2026-09-16 sections for the reasoning (why `Form`/`FormState` can't cross a
server fn, the `prelude::Form` name-collision trap, the `push_field_error`
dispatch shape). Still open: `web/src/views/auth/change_password.rs` itself
still references the deleted derive-path types (`ChangePasswordFormState`/
`ChangePasswordFormHandlers`) and won't compile until it's migrated — that's
the next step, following `login.rs` (commit `9442235`) as the worked example.
Todd is writing that view himself now.

**Mid-flight, same session: generalizing away `ChangePasswordProblem`.**
`crates/api/src/auth.rs` currently returns a bespoke `ChangePasswordProblem`
enum (`Field(path, message)` / `Form(message)`) — Todd doesn't want every form
crossing the server boundary to invent its own `XProblem` type. The direction
being built toward instead: send `.leaves()`'s own `(path, value)` pairs
across the wire (already plain, already-serializable — no trait-object
problem), rebuild a real `FormState<T>` server-side via `empty_form(spec)` +
`.apply(&values)`, get full field- and form-level `.validate()` back for
free (including the spec's own cross-field validator, not hand-duplicated),
and collect errors generically via a new `FormMember::collect_errors`. **UPDATE 2026-09-17 — the formoxus half is DONE; the api/web half is not.**
`FormErrors { form: Vec<FormError>, fields: Vec<(String, Vec<FieldError>)> }`
exists and is `Serialize`/`Deserialize` (direction (1): ONE type for every
form, no codegen — not the per-form `XErrors` generation). `collect_errors` is
implemented on all five members plus `FormState`, and `FormErrors`/`FieldErrors`
are re-exported from `reflect`. Two defects found and fixed once tests existed
(`VariantSet` silently dropping its own errors; `FormField` emitting empty
`(path, [])` entries) — full writeup in [[facet_form_design_decisions]]'s
2026-09-17 section, 14 regression tests in
`crates/formoxus/src/reflect/tests/errors.rs`, `cargo test -p formoxus` green
at 187+.

**What is still NOT done — this is where to pick up.** Nothing calls
`collect_errors`. `crates/api/src/auth.rs` still returns the bespoke
`ChangePasswordProblem` and still sends the whole typed model. The open design
question, which decides the signature: does `change_password` take
`Vec<(String, String)>` leaves and rebuild server-side via `empty_form(spec)` +
`apply_form_values` + `validate` (which gets the spec's own cross-field
validator for free, instead of the hand-duplicated
`check_new_and_confirm_match` call at `auth.rs:201`), or keep taking the typed
model and merely return `FormErrors`? Todd's call, not yet made.

**TWO files fail to compile against the rewritten `auth.rs`, not one** (the
earlier note only caught the first):
- `crates/web/src/views/auth/change_password.rs:37` — `submit_change_password`
  still takes `Store<ChangePasswordFormState>` and still does
  `*form.write() = form_state`, the old whole-state round trip. The component
  body above it (lines 10-30) is already migrated to `use_form` + `using_fns!`.
- `crates/fixtures/tests/change_password.rs` — reaches for
  `ChangePasswordFormState` and treats `ChangePasswordProblem` as if it had
  `current_password`/`new_password`/`errors` fields. 7 errors. Means
  `cargo test --workspace` cannot currently pass even ignoring `web`.

## 3. A `Path` (and/or `Prefix`) newtype — deferred design idea (2026-09-16)

Raised while looking at `push_field_error`'s dispatch: paths, prefixes, and
bare field/row/variant names are all just `&str`/`String` throughout
`members.rs` and every member impl, which is why the code has to warn about
mix-ups IN PROSE instead of the compiler catching them — e.g. `list_set.rs`'s
"`my_path`, NOT the row's own path" and `variant_set.rs`'s "two paths are in
play, and they are NOT the same". A newtype (plausibly *two*: a growing
`Prefix` and a complete `Path`, since those are conceptually different things
sharing one representation today) could make that a compile error instead of
a runtime `no_such_path`, and could turn `qualify`/`owns`/`model_path`/
`variant_segment`/`row_segment` from free functions-on-strings into methods
with enforced invariants.

**Deliberately not started.** It touches every `FormMember` trait method
(`edit`, `push_field_error`, `collect_leaves`, `apply_leaves`,
`apply_specs`) plus every free function above, across all five member impl
files — a genuinely bigger, more cross-cutting change than anything else on
this list, and the `Prefix`-vs-`Path`-vs-bare-`Name` taxonomy needs its own
deliberate design pass rather than being decided as a side effect of
whatever's being worked on when it's picked up. Do it as its own pass, not
folded into another change.
