---
name: prelude-is-the-one-list
description: "DECIDED 2026-09-30: formoxus::prelude is the ONLY export list — the crate root is just `pub use prelude::*`, everything else is reached by its defining module, and custom-widget authors glob `formoxus::widgets::*`. Internal code and the macros use full module paths, never prelude names. 2026-11-09: NO NEW re-exports until the publish-time API pass"
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

## No new re-exports, until publish (Todd, 2026-11-09)

**From 2026-11-09 a new item is reachable only from its defining module.** Don't add it
to the prelude or to `formoxus::widgets::*`, and don't add any other `pub use`. Every
top-level module in `lib.rs` is already `pub`, so nothing becomes unreachable. Todd:
"maybe in the future, we should only expose things from one location and then when we get
ready to publish we can decide what the API should look like." Agreed: "Good rule going
forward."

**Why:** each re-export is a second name for one item, and the choice of which ones exist
belongs to the publish-time API pass ([[pre-publish-checklist]] item 5). Re-exports added
piecemeal now would only be undone or re-argued then.

**How to apply:**
1. A new public item gets no `pub use` anywhere. Callers, including the suite and the
   examples, name it by its defining module.
2. **The two existing lists stay as they are until that pass**, the prelude and the
   `widgets::*` kit. The suite, the examples and apcsp import through them, so trimming
   them now is churn the pass might reverse.
3. **The one re-export decided before the rule stays:** `Validity` in `formoxus::widgets`,
   added in ERRORS_PLAN 6f. Todd, asked whether to drop it: "We already decided to expose
   it from widgets. That's fine."
4. The internal rule above is unchanged and is what makes this cheap: inside the crate,
   `use crate::…` always goes to the defining module. A 2026-11-09 sweep found eight
   crate-root imports in `src/` and fixed them (`ValidationMessage` ×6, `Classes`,
   `ErrorsByPath`). Grep for `use crate::[A-Z]` and for capitalised top-level items in
   `use crate::{…}` to find any more.
