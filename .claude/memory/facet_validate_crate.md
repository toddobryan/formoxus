---
name: facet-validate-crate
description: "FOUND 2026-11-25: facet-validate (facet 0.50 only, rc.7) = consumer-agnostic field constraints on the model (validate::min/max i64, min/max_length, regex, email, url, contains, custom fn); formoxus could read them at build"
metadata:
  type: reference
---

`facet-validate` is facet's own attribute grammar (`ns "validate"`) for constraints
written on a model's STRUCT FIELDS (`#[target(field)]` only, never on a type):
`validate::min = 0` / `max` (both `i64` only: no floats, no u64 > i64::MAX),
`min_length` / `max_length` (`usize`, String/Vec/slices), `regex = r"…"`, `email`,
`url`, `contains = "…"`, `custom = fn` (`fn(&T) -> Result<(), String>`). facet-json
runs them when deserializing. Deps: facet, facet-core, regex. NOT formoxus, so a
model can carry them without depending on formoxus.

**Only exists for facet 0.50** (first release 0.50.0-rc.0; rc.7 latest on 2026-11-25),
none for 0.46.5, which formoxus stays on until 0.50 is stable
([[facet-050-compatibility]]).

Why it matters: Todd (2026-11-25) wants models free of formoxus-specific attributes
(so `#[facet(formoxus::proxy = String)]` is unwelcome even with no dependency), but
consumer-agnostic facts (bounds, lengths, patterns) ON the model are fine. formoxus
could read these from the field's attributes at build and fold them into the
default field's settings/check, as `form!`'s `min:`/`pattern:` do today. Read it
with `field.get_attr(Some("validate"), "min")` and `get_as::<facet_validate::Attr>()`,
by analogy with [[facet-proxy-hook]]'s `formoxus::field` probe (not yet probed for
this crate).
