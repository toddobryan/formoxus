---
name: errors-here-vs-within
description: "FormMember::has_errors was split 2026-10-09 into has_errors_within (subtree, for validate) and has_errors_here (own errors, for rendering/aria-invalid); why they can never agree"
metadata:
  type: project
---

**2026-10-09, Todd's names, Claude's change.** `FormMember::has_errors` became
two methods:

- `has_errors_within()`: this member or anything under it. What
  `FormState::has_errors` (kept its name; at the root it means one thing) and
  so `validate` ask. A leaf also counts an unvalidated `FieldValue::Invalid`.
- `has_errors_here()`: this member's OWN errors only (`!self.errors.is_empty()`).
  What every render site asks: the messages shown and `aria-invalid`.
  `OptionMember` delegates both to `inner` (it is transparent).

**Why:** they answer different questions and CANNOT be made to agree for a
container. A `VariantSet`'s own error is "choose a variant"; a bad `radius`
inside the chosen variant is the child's. With one name, step 9 of
`formoxus/ATTRIBUTES_PLAN.md` passed `has_errors()` for the variant
`<select>`'s `aria-invalid` and marked a perfectly good choice invalid. Same
shape for `ListSet` (row-count error vs a bad cell).

**How to apply:** anything that decides how THIS member renders uses
`has_errors_here`. Anything that decides whether the form can build a model
uses `has_errors_within`. Pinned by
`enums::a_bad_field_in_the_variant_does_not_mark_the_choice_invalid`.
Related: [[facet-form-design-decisions]], [[error-model-design]].
