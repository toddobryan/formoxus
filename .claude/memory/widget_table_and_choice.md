---
name: widget-table-and-choice
description: "UPDATED 2026-09-25: every unrenderable (kind, widget) pair is now a COMPILE ERROR, except radio_group, which gets select's rule while Todd builds it. Original (2026-09-19): Only 45 of 84 (value kind, widget) pairs render; the other 39 fail by the field SILENTLY VANISHING, not by crashing. The blocker is not missing match arms: ValueKind::Choice/MultiChoice/File are never constructed anywhere, so there is nothing for an arm to match. The design fork — SHAPE choice vs VALUE choice — needs Todd before any code"
metadata:
  type: project
---

**UPDATE 2026-09-25: the silent vanishing is now a compile error.** `form!`
rejects every pair `ScalarWidget` cannot render, with the caret on the widget
name. `field_kind::renders` and `is_single_value` are const checks; the rules
come from the macro's `widget.rs::rule`, derived from the `WidgetType` variant.
`select_multiple`, `checkbox_multiple` and `file` are refused while the macro
parses. A widget on a struct, list or enum is refused too. On an enum it was
silently IGNORED before, since `VariantSet` stores `custom_widget` and never
reads it. ONE deliberate divergence: `radio_group` gets `select`'s rule
(`bool` always, anything else only with `choices`) because Todd is building
it; until that lands, it passes the check and still vanishes. `widget_matrix`
still measures the runtime match directly, so the two tables can be compared.
Also, `ValueKind::Choice`/`MultiChoice` were settled as DELETE rather than fill
in: choices attach to the widget ([[choice-fields-design]]).

## The measurement

`cargo run -p formoxus-examples --bin widget_matrix` prints it, and the
number is **45 of 84**. `form!` accepts all 84 today.

- all 14 `<input type=…>` work on Text/Int/Float, and **panic on Bool**
- `textarea` works only on a string
- `select` and `checkbox` work only on `bool`
- `select_multiple`, `checkbox_multiple`, `radio_group`, `file` never work

45 was predicted by reading `ScalarInput`'s match arms *before* the tool was
run, and the tool agreed exactly — that agreement is the reason to trust
either.

## The failure mode is worse than a crash

**Dioxus catches the panic inside the component's own scope and renders that
subtree as nothing.** So `catch_unwind` reports that all 84 pairs pass, and
what a user actually gets is the field *silently disappearing* from the form
while its siblings render normally. No error reaches an `ErrorBoundary`,
nothing is logged in release. This is why `widget_matrix` looks for
`name="<path>"` in the rendered HTML instead of trying to catch a panic.

(Two markers tried before that one were false positives, and both say
something true about the widgets: a `hidden` input renders bare with no label
at all, and a checkbox puts its text in a plain `<label>` rather than the
`field-label` span everything else uses.)

## Why this is NOT "add four match arms"

`ValueKind::Choice`, `MultiChoice` and `File` are declared in the enum and
**never constructed anywhere in the crate**. `FormField::value_kind()` derives
purely from `T::SHAPE.scalar_type()` and can only ever produce `Text`, `Bool`,
`Int`, `Float`. Since `ScalarInput` dispatches on `(ValueKind, WidgetType)`,
a choice field has no value kind to match against — the arm has nothing to be
written *for*. Something upstream has to start producing a `Choice` first.

## The fork, which needs Todd

There are already **two different notions of "choice"**, and only one is built:

1. **Shape choice — which variant does this value take?** Built:
   `VariantSet` + `VariantSelect`, on a path entirely separate from
   `WidgetType::Select`. See [[facet-form-design-decisions]] (typed
   `VariantChoice`, iterative disclosure, `choose_variant` dispatching by path
   containment).
2. **Value choice — which of these N runtime candidates?** NOT built. A
   `Ref<Source>` picked from a course's sources, a country from a list. This
   is what `radio_group` / `select_multiple` / `checkbox_multiple` would all
   render, and it is where the real parity gap with Django and leptos_form
   lives ([[formoxus-feature-parity]]).

The candidate list for (2) usually isn't knowable from the shape, so it
probably entangles with `Provider` — and possibly with
[[widget-registry-idea]], which is a different question about *default widget
selection* and should not be conflated with this one.

**Decide before writing arms: do (1) and (2) stay separate mechanisms, or get
reconciled?** `file` is a third thing again — it needs `ValueKind::File`, a
file-carrying type in the model, and multipart handling, and is much larger
than the other two.

## State of the repo when this was queued

`main` at `6d76dc4`, pushed, CI green on all five jobs. The derive path is
gone and `reflect` is flattened ([[derive-path-removal]]); `form2!` is `form!`;
metadata, README, dual MIT/Apache licenses and `.git-blame-ignore-revs` are in;
the workspace is rustfmt-clean at defaults; MSRV 1.90 is verified, not merely
derived. 317 tests.

**The other open publish item is the doc pass** — 179 undocumented public
items (60 struct fields, 51 variants, 41 methods). Deliberately deferred here:
it is a large mechanical job needing no decisions, so it is the thing to do
when tokens are plentiful, and it should end by turning on
`#![warn(missing_docs)]` so CI stops it regressing. The hard reasoning is
already documented; it is the ordinary surface that is bare.

## UPDATE 2026-09-19: the fork is partly answered

Todd proposed a design the same day — see [[choice-fields-design]]. In short:
value-choice stays separate from shape-choice, choices attach to the WIDGET
rather than the type (so `ValueKind::Choice`/`MultiChoice` get deleted, not
filled in), and a `Choice { display, raw_value }` reuses the ordinary parse
path. Still open there: how the choices callback is carried, because a
`Box<dyn Fn>` cannot live on `WidgetType` without killing its derives.
