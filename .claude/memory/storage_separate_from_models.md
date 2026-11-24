---
name: storage-separate-from-models
description: "DIRECTION, Todd 2026-11-23 (surreal work to its OWN REPO, after custom fields): named `ithell` (Ithell Colquhoun), split out after custom fields; models derive only Facet; form! makes forms, a new function-like table! does SurrealDB, so storage is pluggable; orphan rule means table! can't impl SurrealValue on a model, so conversion goes through facet"
metadata:
  type: project
---

**Decided 2026-11-23 (Todd):** the SurrealDB work splits out into its own repo,
as formoxus did from apcsp, and comes AFTER `formoxus/CUSTOM_FIELDS_PLAN.md`,
which assumes its storage-agnostic `Ref<T>` (that plan's decision I) and must
not add facet to today's surreal-table `Ref<T>`. Not planned yet.

**Todd's idea (2026-11-23):** in apcsp, rewrite
surreal-table so a model is a plain struct that derives `Facet` and nothing
else. `form!` makes it a form (as now), and a new **`table!`** handles the
SurrealDB side. Storage is then separate from the models, and a dev could plug
in their own storage layer.

**Why it fits:** it is formoxus's own shape. `form!` is function-like rather
than a derive precisely so it can expand at the call site, in a crate that does
not define the model (CLAUDE.md, "load-bearing rather than stylistic").

**The constraint found the same day:** the SurrealDB SDK (3.2.4) takes
`D: SurrealValue` in `create/insert/update/upsert().content()`. A `table!` in a
storage crate cannot `impl SurrealValue for models::X` (orphan rule: foreign
trait, foreign type). So, as `form!` does, `table!` must produce a VALUE or a
LOCAL type (a `TableSpec<X>`, a marker type, a `Stored<X>` wrapper), and the
model↔`surrealdb_types::Value` conversion is one generic facet walk, written
once, not a per-type impl.

**Payoffs noted:** one source of truth for an enum's representation (apcsp's
`QuestionGroup` today repeats `tag = "type", rename_all = "snake_case"` in both
`#[surreal(…)]` and `#[serde(…)]`; facet has the same attributes); and a
storage-agnostic `Ref<T>` with `Facet` + `formoxus::proxy = String`, mapped to a
`RecordId` by the storage layer, which is what
`formoxus/CUSTOM_FIELDS_PLAN.md` needs anyway.

**Open:** models probably still need serde for Dioxus server functions'
transport; per-column details (`type = "int"`, `assertion`, `value =
"time::now()"`, `readonly`, indexes) move into `table!`'s grammar.

**Name, front-runner (Todd, 2026-11-23): after Ithell Colquhoun,** British
surrealist (SurrealDB is UK-based) who spent years on occult CORRESPONDENCES,
which is what a storage layer is: a mapping between model and table. Crate names
`colquhoun` and `ithell` both FREE on crates.io, and no software project uses
either (web check, same day). **`ithell`** it is (Todd, 2026-11-23: easy to spell; fine that her first
name is less known), for repo and crate alike. Next after custom fields: split
the surreal work out of apcsp-dioxus into the `ithell` repo. Rejected the same day: `miro` (crate taken; Miro the
whiteboard company), `dali` (taken; DALL·E), `penrose` (taken, active).
`magritte` is free but is also a Smalltalk meta-description framework (Renggli
2006) driving views, editors AND persistence from one description: prior art
worth reading before designing `table!`.

Relates to [[facet-proxy-hook]].
