---
name: formoxus-feature-parity
description: "Formoxus vs. Django forms and leptos_form, RE-SURVEYED 2026-09-25 against the reflection-path code (after constraints landed). leptos_form is effectively unmaintained (last release 0.2.0-rc1, Feb 2024). Ahead: compile-time checks, enums, one validation on both sides. Biggest gaps: a checkbox cannot be required to be TICKED (issue #6, a real Django divergence), 3 widgets form! accepts but cannot render, per-field validators, typed dates/decimals/uuids, help text and AUTHOR-SUPPLIED widget attrs, change tracking, formset bounds (formoxus's OWN constraint attributes landed 2026-09-27)"
metadata:
  type: project
---

Re-surveyed 2026-09-25, replacing a 2026-08-29 comparison of the OLD derive
path (deleted 2026-09-19, see [[derive-path-removal]]). leptos_form was read
from its 0.2.0-rc1 source (`leptos_form_proc_macros_core/src/form.rs` option
structs, `leptos_form_core/src/form_component/impls/`). Django is from
knowledge of its forms API. formoxus claims were checked in code, and the
widget dispatch is `formoxus/src/widgets/scalar.rs`.

**leptos_form has not released since 2024-02-05** (0.2.0-rc1, stable 0.1.8).
Useful for ideas, but Django is the real bar.

## Ahead of both
- Compile-time checks: paths (`__paths_exist`), constraint vs. type, bound
  ordering and fit, pattern validity ([[const-shape-walk-blocked]]).
  leptos_form has NO declarative constraints, only a per-type `validate` hook.
- Enums as a chosen variant (VariantSet), nested structs, `Option`, `Vec` rows,
  all from the shape, with no per-form type.
- One validation on both sides of the wire (`Submission`, `WireForm`).

## Gaps, as of this survey
1. **Accepted but broken:** `radio_group`, `select_multiple`, `checkbox_multiple`
   and `file` pass `form!` and hit the panic fallthrough in `ScalarWidget`, so
   the field vanishes ([[widget-table-and-choice]]).
2. **Per-field validators / field-attached errors** (Django `clean_<field>`,
   `validators=[]`, `add_error`). Designed, unbuilt ([[error-model-design]]).
3. **Custom and translatable error messages** (Django `error_messages` per code).
   Ours are fixed English strings in `ValueKind::check`.
4. **Dynamic choices:** `Provider` exists but nothing consumes it for choices.
   Plus the server-side membership check (Django `ModelChoiceField` validates
   membership).
5. **Value types:** only String, bool, i8–u64, f32, f64. No dates or times
   (Django Date/DateTime/Time/Duration, leptos_form chrono), Decimal, Uuid,
   multi-value (a leaf holds ONE string), or files.
6. **Presentation:** no help_text; no widget attrs (class, placeholder, style,
   rows; tier 2 is unbuilt); **constraint attributes now ARE rendered, 2026-09-27
   — `minlength`/`maxlength`/`pattern`/`min`/`max` reach the DOM via
   `ValueKind::attrs`, so the browser enforces them; what remains is emitting
   only the ones VALID for the element (issue #4) and letting an author pass
   arbitrary attributes through**; fixed error markup (leptos_form has 5 error
   modes); no
   label placement options; no disabled/readonly fields; no field exclusion.
6b. **DONE 2026-10-01 — `required: true` on a bool.** Originally: **A checkbox cannot be required to be TICKED** (issue #6, found 2026-09-27).
   Django's `BooleanField(required=True)` means the box must be checked;
   formoxus's `required` means presence, and for a `bool` unticked is a complete
   answer — so `Checkbox` drops `required` on purpose and nothing enforces
   must-agree on either side. A "I agree to the terms" box can be submitted
   unticked and formoxus accepts it. The fix is a value constraint
   (`ValueKind::Bool` gaining a payload, `check` + `attrs` + a compile-time gate),
   NOT forwarding `required` — the presence sense is load-bearing elsewhere
   (`RenderCtx.required` drives `Select`'s placeholder and `radio_group`'s refusal
   of an `Option`), so redefining it for bools would make one word mean two
   things depending on the field's type.

   **DECIDED by Todd 2026-10-01, overriding the lean above:** the `form!` key IS
   `required: true`, Django's spelling, legal ONLY on a non-optional `bool`
   field (compile-time gate; his draft message: "required is only allowed on
   bool fields and indicates that its value must be true; required is implied
   for all other fields unless the value is an Option<_>"). Any bool widget may
   carry it; RadioGroup/Select browser-side enforcement still to think about.
   BUILT 2026-10-01 differently from first planned: NOT a `FieldProps` field.
   The HTML attribute already arrives via `ValueKind::attrs` (a constraint like
   any other), so only the ` *` marker needed a signal, and `ScalarWidget`'s
   checkbox arm binds `required_true` from `ValueKind::Bool` and passes it as a
   `Checkbox`-only prop. `FieldProps` stays the same for every widget and
   `required` keeps one (presence) meaning in the Rust API. A bool `select`
   already gets the marker from presence. Compile gates `takes_required` +
   `required_is_not_optional` (field_kind) and a `checkbox`-on-`Option` refusal
   (widget.rs) also BUILT, with goldens. Semantics: on a Checkbox, `Empty` and `Valid(false)`
   are one answer and both fail; on Select/RadioGroup `Empty` already fails
   presence and `Valid(false)` fails the new rule. TRAP found scoping it: an
   unticked box is `FieldValue::Empty`, and `FormField::validate` returns on
   `Empty` before `ValueKind::check`, so a rule only in `check` never sees the
   unticked case. ALSO FOUND: `widget: checkbox` on an `Option<bool>` COMPILES
   today (`field_kind::kind` peels the `Option`; only `radio_group` has a
   `not_optional` assert, in `formoxus-macros/src/form/widget.rs`), though a
   two-state widget cannot express three states. Todd wants it refused. And
   `validate` can see `self.widget()` (pure data, fine server-side), so the
   message can vary by widget. The key is a `syn::LitBool` in `field_body!`
   (so only a literal parses), and **`required: false` is a compile error too**
   (Todd, 2026-10-01): on any non-bool field an explicit `required` would let a
   form disagree with its model — `false` on a non-`Option` field or `true` on
   an `Option` describes forms the model cannot hold — so presence comes from
   the type alone, and on a bool `false` is just the default. Division of labour: Todd writes the code, I
   review it, then write suite tests, trybuild goldens and a `/t/` form + e2e
   pair.

7. **State:** no change tracking (Django `has_changed`/`changed_data`,
   leptos_form `field_changed_class`); no draft persistence (leptos_form
   localStorage cache); no live/blur validation.
8. **Submission lifecycle:** no built-in pending, success or error states, and
   no `reset_on_success` (leptos_form has these). Handlers via `using_fns!` cover
   the rest.
9. **Formsets:** no min/max row counts, no delete/order flags for existing
   rows, and rows are index-named, not stable ids.
10. **Docs:** the `form!` grammar is undocumented (a `TODO` on `form!`).
