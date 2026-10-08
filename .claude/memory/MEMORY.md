> Renames to read through in older entries: 2026-09-19 `formoxus::reflect::X` → `formoxus::X` ([derive-path removal](derive_path_removal.md)); 2026-09-29 `SelectChoice`→`Choice`, `ValuesByPath`→`ValuesStore`, `ButtonType::Button`→`Action`, all CSS classes `fx-`-prefixed ([CSS vocabulary](css_class_vocabulary.md)).

**Index rule: one line per entry, under ~200 chars. Detail goes in the topic file, never here** — this file is truncated past ~24KB.

## Start here / how Todd works
- [**START HERE: next session**](next_session.md) — 2026-10-07: green; 3d parsing done, widget-side attrs next (Todd); read `formoxus/ATTRIBUTES_PLAN.md`; how Todd works
- [User role](user_role.md) — Todd writes core library code; I write tests, plans, memory, investigate; he edits the same files concurrently — re-read first
- [Show scope before writing code](feedback_show_scope_before_writing_code.md) — RECURRING: a "yeah" is not approval to write code; say what would change, then wait
- [Reply format](feedback_numbered_not_bulleted.md) — replies: NUMBERS for him to act on, LETTERS to respond to, no bullets
- [No PR per change](no_pr_per_change.md) — commit/push only when told; no standalone memory commits; `just hooks` once per machine for the fmt pre-commit hook

## Plan
- [MVP scope](mvp_scope.md) — agreed order to first publish: #6 (DONE 2026-10-01) → C6 → #9 → help_text → C4 → facet → grammar docs → doc pass
- [Pre-publish checklist](pre_publish_checklist.md) — RAISE WHEN PUBLISHING COMES UP: wasm-opt, the doc pass, facet version, macro-side `regress` out of the bundle
- [Parked: validate_values](parked_validate_values.md) — Todd wants these back: `validate_values`, `collect_values`→`ValuesByPath`, typed path keys (BREAKING, so pre-publish)
- [Formoxus feature parity](formoxus_feature_parity.md) — vs Django + leptos_form; **#6 DONE 2026-10-01: `required: true` on a non-optional bool = must be ticked**
- [Lint config](lint_config.md) — RAISE PERIODICALLY: why `unused_qualifications` is off (rsx! false positives) and how to sweep for real ones
- [Formoxus roadmap](formoxus_roadmap.md) — STALE, superseded by mvp_scope; kept for derive-path history
- [Running to-do queue](next_up_two_todos.md) — 2026-09-18 wire rework (leaves over the wire, `Submission<T>`); mostly historical

## Core design
- [Facet form design decisions](facet_form_design_decisions.md) — the big one: vtables not FromStr, "" IS absence, Form vs FormState, `[]` row selectors, buttons, Option peeling
- [Facet form spike](facet_form_spike.md) — origin of the reflection path: RenderCtx, one component per leaf, `$Variant` segments, retiring the derive path
- [Derive-path removal](derive_path_removal.md) — 2026-09-19: derive path deleted, `reflect` flattened to the crate root; three traps hit doing it
- [Prelude is the one list](prelude_is_the_one_list.md) — the prelude is the only export list; internal code and macros use full module paths
- [Widget is the umbrella word](widget_is_the_umbrella_word.md) — "widget", never "control"; why the 2026-09-19 rename was reversed
- [Error model design](error_model_design.md) — `ValidationError<T>`/`ValidationMessage` BUILT 2026-09-30; per-field validators and `ErrorsStore` still open
- [Config cascade](config_cascade.md) — app → form → field via Dioxus context; RULE: resolve at the `Form` boundary, never call `defaults()` while rendering
- [Const shape walk blocked](const_shape_walk_blocked.md) — compile-time checks are FREE `const _` items with `shape_of`, not `const {}` in a generic fn
- [Facet newtypes & custom widgets](facet_newtypes_and_custom_widgets.md) — newtypes are leaves; `WidgetType::Custom` is a fn pointer; `wrapper` side-channel
- [facet re-export needs a direct dep](facet_reexport_needs_direct_dep.md) — users still need `facet` directly; the derive hard-codes `::facet` (facet#2662)
- [facet 0.50 compatibility](facet_050_compatibility.md) — 0.50.0-rc.7 is a drop-in; stay on 0.46.5 until stable

## Widgets, choices, attributes
- [Widget table & what a Choice is](widget_table_and_choice.md) — unrenderable (kind, widget) pairs are compile errors; shape choice vs value choice
- [Choice fields design](choice_fields_design.md) — tier 1 (`select { choices: EXPR }`) BUILT; tiers 2–3 (dynamic) open; `raw_value` may never be `""`
- [Chooser ideas](chooser_ideas.md) — QUEUED: select-vs-radio by choice count, per-choice hint text
- [Enum as a value choice](enum_as_a_value_choice.md) — GAP: a unit enum can't take a widget (no radio group over an enum)
- [Constraint attributes gap](constraint_attributes_gap.md) — BUILT 2026-09-27 + e2e-proven; validity is issue #4; don't re-anchor `pattern`; no `step`
- [Shrink FieldProps idea](shrink_fieldprops_idea.md) — 2026-10-08: `required`/`aria_invalid` DO move into the typed map (3d); `path`/`choices`/`errors` stay props; RadioGroup hard-places aria-invalid
- [CSS class vocabulary](css_class_vocabulary.md) — DECIDED 2026-09-29: `fx-` prefix, `fx-control` exception, the full class list
- [Emitted classes and strings](emitted_classes_and_strings.md) — QUEUED: classes and English into `Formoxus`; error text runs server-side, can't read `defaults()`
- [Widget registry idea](widget_registry_idea.md) — PAUSED: type-keyed default widgets; superseded in part by the config cascade
- [Attribute rules design](attribute_rules_design.md) — build order in `formoxus/ATTRIBUTES_PLAN.md`; DECIDED 2026-10-02/03: ONE attribute table in a new `formoxus-attrs` crate; legality per widget; custom widgets declare ATTRS; C6 + #4
- [HTML attributes reference](html_attributes_reference.md) — 2026-10-02, from the WHATWG spec: valid attributes per form element and `<input>` type, where each widget spreads, vs dioxus-html; for C6 and #4
- [Formoxus widget survey](formoxus_widget_survey.md) — Django's Field/Widget split (constraints follow the value type); why a proc macro beat facet attrs

## Gotchas
- [Hand-written rsx gotchas](hand_written_rsx_gotchas.md) — no statements in an rsx `for`; missing `rsx!` reads as struct literals; spread goes LAST; `extends` names an element
- [Formoxus/darling gotchas](formoxus_darling_gotchas.md) — darling field defaults, `SpannedValue`, quote!/rsx! nesting traps
- [E2E harness](e2e_harness.md) — `just e2e`; use `get_by_role` with a name, not exact `get_by_label` (aria-hidden marker)
