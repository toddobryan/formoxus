---
name: models-stay-plain
description: "Todd 2026-11-25: NO formoxus-specific (or storage-specific) attributes on models, not even dependency-free `formoxus::proxy`; consumer-agnostic facts (bounds, lengths, patterns) MAY sit on the model in a neutral vocabulary"
metadata:
  type: feedback
---

A model is just the model. What formoxus or a storage layer needs is added IN
that layer (`form!`, the app's type registry, `table!`), never written on the
model, even when it costs no dependency: `#[facet(formoxus::proxy = String)]`
was rejected on 2026-11-25 for this reason alone, after the whole proxy design
had been planned and its tests written (CUSTOM_FIELDS_PLAN box 5, `a8463e0`).

The exception Todd drew the same day: facts about the model that many consumers
care about (min/max on numbers, min_length/max_length/pattern on strings,
constraints on relations) may be declared on the model in a consumer-agnostic
way, e.g. a neutral attribute grammar crate (plan box 10,
[[facet-validate-crate]]). facet's own `#[facet(transparent)]` counts as
consumer-agnostic.

**Why:** the same model feeds forms, storage and the wire; layer-specific
attributes couple it to every layer and are what `form!`/`table!` being
function-like (call-site expansion) exists to avoid ([[storage-separate-from-models]]).

**How to apply:** when a design puts a namespaced attribute or trait impl on a
model type for one consumer's sake, stop and find the layer-side route (registry
keyed by `Shape`, call-site macro, local wrapper type). Ask "would any other
consumer care about this fact?" before proposing it on the model.
