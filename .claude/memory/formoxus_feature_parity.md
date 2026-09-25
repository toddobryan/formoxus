---
name: formoxus-feature-parity
description: "Formoxus vs. Django forms and leptos_form, RE-SURVEYED 2026-09-25 against the reflection-path code (after constraints landed). leptos_form is effectively unmaintained (last release 0.2.0-rc1, Feb 2024). Ahead: compile-time checks, enums, one validation on both sides. Biggest gaps: 4 widgets form! accepts but cannot render, per-field validators, typed dates/decimals/uuids, help text and widget attrs, change tracking, formset bounds"
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
   rows; tier 2 is unbuilt); constraint attributes (maxlength, min, pattern)
   never rendered; fixed error markup (leptos_form has 5 error modes); no
   label placement options; no disabled/readonly fields; no field exclusion.
7. **State:** no change tracking (Django `has_changed`/`changed_data`,
   leptos_form `field_changed_class`); no draft persistence (leptos_form
   localStorage cache); no live/blur validation.
8. **Submission lifecycle:** no built-in pending, success or error states, and
   no `reset_on_success` (leptos_form has these). Handlers via `using_fns!` cover
   the rest.
9. **Formsets:** no min/max row counts, no delete/order flags for existing
   rows, and rows are index-named, not stable ids.
10. **Docs:** the `form!` grammar is undocumented (a `TODO` on `form!`).
