---
name: facet-option-is-an-enum
description: "GOTCHA 2026-10-09: facet types Option<T> as Type::User(UserType::Enum(_)), so a shape.ty test calls every optional field an enum; ask field_kind's kind(), which peels the Option"
metadata:
  type: project
---

`facet-core` 0.46.5 (`src/impls/core/option.rs`) gives `Option<T>` a `ty` of
`Type::User(UserType::Enum(EnumType { … }))`, with `Some`/`None` as variants.
So `matches!(shape.ty, Type::User(UserType::Enum(_)))` is true for EVERY
`Option`, including `Option<String>`.

**Hit 2026-10-09** writing `field_kind::is_enum` for the enum-specific
`form!` message: `min: 3` on an `Option<String>` started reporting "`min`
cannot go on an enum field". The golden `form_min_on_a_string` caught it (and
`TRYBUILD=overwrite` silently rewrote it — read the diff).

**How to apply:** classify through `field_kind::kind`, which peels
`Def::Option` before looking at `ty` (and follows `#[facet(transparent)]`
newtypes). `is_enum` is `matches!(kind(shape), Kind::Enum)`; pinned by
`an_option_is_an_enum_only_when_it_holds_one`. Any new shape test on `ty`
needs the same care; check `shape.def` for `Def::Option` first.
