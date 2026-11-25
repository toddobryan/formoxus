---
name: transparent-not-followed
description: "DECIDED 2026-11-26 (W): formoxus does NOT follow #[facet(transparent)]; every newtype is registered explicitly, via the fields::Newtype<T, F> adapter; why, and the agreed fallback"
metadata:
  type: project
---

`CUSTOM_FIELDS_PLAN.md` decision W, amending N. A `#[facet(transparent)]`
`Markdown(String)` gets no special treatment: like a plain newtype or a struct,
it needs a registered field or a `field:`. The cheap route is the adapter
`formoxus::fields::Newtype<T, F>`, which passes everything to an inner field
and wraps/unwraps through facet's `Partial`/`Peek` (no `From` impls on the
model, so [[models-stay-plain]] holds).

**Why (Todd):** explicit beats implicit; less surprising. Also found while
weighing it:

1. A transparent type can validate through `try_from`; built through
   `begin_inner`, its refusal would surface in `write_value_into` AFTER
   `validate()` passed, with no field error for the user.
2. Following it needed an outer shape threaded through `LeafArgs` (a
   transparent type's widget is keyed on the outer type) and a `begin_inner`
   write-back. Without it, a leaf's shape is always its path's own type.
3. Transparent never gave settings anyway (`max_length:` on a newtype path
   needs `DefaultField`, which no newtype has); `field: Newtype::<..>::new(..)`
   does, once `form!` accepts that call (decision X).

**Cost accepted:** one registration line per newtype instead of one
attribute, and the type's widget/attribute checks move from compile time to
the build (V).

**How to apply:** do not "helpfully" re-add transparent support. Todd's
agreed fallback, only if it proves obnoxious in real use: follow a
transparent wrapper over a `String`, `bool` or number automatically, AFTER the
registry check so a registered field still wins. Related:
[[facet-proxy-hook]] (rejected for the same model-purity reason).
