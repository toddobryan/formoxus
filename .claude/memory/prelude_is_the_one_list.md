---
name: prelude-is-the-one-list
description: "DECIDED 2026-09-30: formoxus::prelude is the ONLY export list — the crate root is just `pub use prelude::*`, everything else is reached by its defining module, and custom-widget authors glob `formoxus::widgets::*`. Internal code and the macros use full module paths, never prelude names"
metadata:
  type: project
---

**Decided with Todd 2026-09-30**, as part of MVP step 1 ([[mvp-scope]]).

## The rule

- **`formoxus::prelude` is the one list.** It holds what a CONSUMER writing forms
  uses: the three macros, `Form`, `FormSpec`, `FormState`, `use_form`,
  `empty_form`, `form_for`, the handler/provider helpers, `Path`,
  `ValidationError`, `ValidationMessage`, `FormErrors`, `ErrorsByPath`,
  `Submission`, `WireForm`, and the `Formoxus` defaults.
- **The crate root is `pub use prelude::*;` and nothing more**, so
  `use formoxus::Form` still works for every prelude name.
- **Everything else is module-only** — `formoxus::members::{Edit, FormMember,
  RenderCtx, VariantChoice, …}`, `formoxus::fields::FieldValue`,
  `formoxus::buttons::ButtonSpec`, `formoxus::form::IntoSlot`.
- **Custom-widget authors get their own glob, `formoxus::widgets::*`** —
  `WidgetProps`, `FieldProps`, `Choice`, `get_current`, `write_value`,
  `ABSENT_DISPLAY`, and `ValuesStore` (defined in `members`, re-exported there
  because a widget cannot be written without it).
- **Internal code and the macros use full module paths** (Todd: "Inside the
  crate (and the macros), I'm willing to use the whole path"). The macros
  already emitted `::formoxus::form::FormSpec`-style paths; inside `src`, no
  `use crate::X` may go through a prelude name — otherwise trimming the prelude
  breaks the crate. Intra-doc links too: `crate::members::VariantChoice`, not
  `crate::VariantChoice`.

## Why

There USED to be two lists — a flat root re-export block AND a prelude — and
they drifted the first time an export changed: the `ValidationError` rename
dropped the error types from the root while adding them to the prelude, so
every `use formoxus::*` in the suite stopped resolving. One list cannot drift.

And `use formoxus::*` was the wrong glob anyway: it pulls in every `pub mod`
(`form`, `path`, `error`, `widgets`, …) along with the types. The prelude's own
comment already documented that hazard for `form` (module AND macro) and routed
around it by re-exporting the macros straight from `formoxus_macros`.

## The suite follows it

Every `use formoxus::*` in `tests/`, `examples/` and `e2e/` became
`use formoxus::prelude::*` plus explicit module imports. That is the point, not
churn: CLAUDE.md says the suite runs against the surface a dependent crate gets,
so the tests now prove the prelude is sufficient for real forms.
