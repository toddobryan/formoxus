---
name: formoxus-feature-parity
description: "Formoxus vs. Django's forms library and leptos_form — what's ahead, at parity, and genuinely missing, as of commit 606aa76 (2026-08-29); leptos_form Vec handling re-checked against its source 2026-09-05"
metadata:
  type: project
---

Comparison drawn from an actual survey of both (Django's official docs; leptos_form's
docs.rs page — my prior knowledge of leptos_form specifically was thin, so I fetched it
rather than guessed), done right after the Provider mechanism landed. See
[[formoxus_roadmap]] for the chronological build log this snapshots against.

**Note on scope:** this compares the *macro-based* formoxus. The reflection rewrite
([[facet_form_spike]]) changes several of these answers — most of the "roughly at parity"
row below is now handled structurally rather than by attribute.

**Ahead of both, genuinely — the Provider mechanism.** Django's `ModelChoiceField`
evaluates its queryset at render time but it's baked into the ORM-aware field type, not
a general trait; leptos_form's docs don't mention an external-data-source concept at
all (a picker's choices appear to be expected static or hand-wired outside the derive).
`Provider<C>`/`ProvidedWidget` (`#[form(component = X, provided)]`) is an explicit,
typed, reusable mechanism for "this widget needs to fetch data from somewhere, supplied
at the use site" — a real differentiator, not just parity.

**Roughly at parity:**
- Repeating groups (`#[form(field_set)]` on `Vec<T>`) vs. Django formsets / leptos_form's
  `VecConfig` — both of theirs do more: min/max row-count bounds, an explicit
  ordering/delete-flag convention for *existing* rows on edit. Ours only has add/remove
  on an always-fresh list.

  **How leptos_form actually does it** (read from
  `core/src/form_component/impls/collections.rs`, 2026-09-05 — the docs.rs page doesn't
  show this):

  ```rust
  impl<T, El> FormField<Vec<El>> for Vec<T>
  where T: Clone + FormField<El>, <T as FormField<El>>::Signal: Clone + Debug
  {
      type Config = VecConfig<<T as FormField<El>>::Config>;
      type Signal = FormFieldSignal<IndexMap<usize, VecSignalItem<<T as FormField<El>>::Signal>>>;
  ```

  A **blanket impl**: `Vec<T>` is a form field whenever `T` is, so `Vec<NestedStruct>`
  works exactly like `Vec<String>` provided the inner type derives `Form`. `Config` and
  `Signal` are both built *recursively* from the inner type's — the same compositional
  move as our `OptionMember { inner }` and `ListSet { rows }`, done in the type system
  instead of at runtime. Configured per field with
  `#[form(config = vec_config())]` where the builder sets `item_label`, `size`, `remove`.

  **Two differences that bear on decisions still ahead of us:**
  1. `VecConfigSize::Bounded { min, max }` is enforced *at render*: it pads up to `min`
     with default rows and truncates past `max`. That's our still-deferred `multiple`/
     `min`/`max` attribute, and it's also a different answer to the row-count question
     than the one [[facet_form_spike]] is heading toward (a construction-time length
     choice, VEC_PLAN step 4) — worth comparing before committing to ours.
  2. Rows are keyed by a **stable monotonic `id`** (`VecSignalItem { id, signal }` in an
     `IndexMap`, with a `next_id` counter), NOT by position. We name rows by index, so
     removing row 0 renumbers everything below it. That's fine under our uncontrolled
     design — add/remove is collect → rebuild → re-apply — but it is exactly what would
     bite if rows ever became reactive. They pay for stable identity because their rows
     *are* signals; if we ever add per-row reactivity, this is the design to revisit.
- Type-driven required/optional via `Option<T>` — leptos_form does the same by inference;
  Django is explicit (`required=False`) but converges on the same UX.
- `FieldSet` composition has no real analog in either. Django forms don't embed other
  forms as a first-class concept; leptos_form's `group` attribute is purely *layout*
  grouping (which container a field's HTML lands in), not a reusable, independently
  validated sub-model like ours.

**Real gaps, roughly in priority order if closing them ever matters:**
1. **Configurable error rendering.** leptos_form has five pluggable modes per field or
   form (`component`/`container`/`default`/`none`/`raw`); ours (`FieldErrors`/
   `FormErrors`) is fixed markup, CSS-hookable but not swappable.
2. **Arbitrary field-level HTML attrs** (`class`, `style`, `id`, custom element) — both
   Django (`widget.attrs`) and leptos_form support this; formoxus only lets you swap the
   whole widget component via `#[form(component = ...)]`, not tweak one in place.
3. **No live/blur-time validation.** leptos_form parses on blur per field; formoxus only
   validates on an explicit `.validate()` call (typically wired to submit).
4. **No auto-wired submission flow.** leptos_form's generated component takes
   `action`/`on_success`/`on_error`/`reset_on_success`/`field_changed_class` —
   batteries-included. Ours requires hand-writing each `Handler` closure — more manual,
   but also more explicit about what actually happens on submit; a defensible tradeoff,
   not obviously a gap worth closing.
5. **File uploads** — neither Django's `ClearableFileInput` tri-state (keep/clear/
   replace) nor a Dioxus upload-signal integration exists in formoxus at all.
6. **Compound/multi-input fields** (Django's `SplitDateTimeField`, leptos_form/Django's
   `MultiWidget`/`MultiValueField`) — likely already expressible via `FieldSet` + a
   hand-written `#[form(model = ...)]`/custom validator (same escape hatch used for
   other exotic cases), but **untested** — don't assume it's covered without trying it.
7. **`validate()` never cross-checks a `provided` field's submitted value against what
   its `Provider` actually returned.** Found via a stress test modeled on Django's
   `inlineformset_factory` + `ModelChoiceField` in a repeating group
   (`crates/formoxus/tests/order_with_line_items.rs`). `Provider` is wired only into
   `render` — confirmed by reading `assign_to_vars`'s codegen, a `provided` field
   validates via the exact same `.required()`/`.optional()` path as any plain field,
   `Providers` never enters `validate()` at all.

   **Existence isn't the right check — scoping is.** First pass at this (see git
   history on this file) framed it as "does the row still exist," and proposed a plain
   existence check in `QuestionStore` before insert. Todd caught the actual failure
   mode: a `Provider` typically *filters* an otherwise-legal DB row out on purpose (e.g.
   `SourcePicker`'s provider only returns sources for the *current course* — a source
   from another course is real, exists, and would pass a bare existence check, but was
   never a legal choice for this field). The real requirement is membership in the same
   scoped set the provider computed, not mere existence — i.e. re-deriving the provider's
   own query and checking the submitted value is in it, which is exactly the "gather your
   own evidence from the DB, don't trust the client" shape this project's authz witnesses
   already use (the repo's `CLAUDE.md` `can_view` pattern) — just applied to a field
   value instead of a view permission.

   **Two-tier design Todd proposed, not yet built:**
   - **Server-side (the real boundary):** re-run the provider's own scoped query in the
     store/API layer and check membership before writing — same pattern as existing authz
     witnesses, and no architecture change needed since server handlers are already async.
     This is the only tier that can actually catch a fabricated/malicious submission.
   - **Client-side (cheap, UX-only):** cache the choice list a `ProvidedWidget` was last
     given (from its `Provider` call) and let `validate()` check the submitted value
     against that cache synchronously — catches honest staleness (picked something that's
     since disappeared) before submit, without making `validate()` async or threading
     `Providers` into it. Explicitly *not* a security boundary (a malicious client
     widgets its own cache), only a formoxus feature worth having for the fast-feedback
     case. Would need each `ProvidedWidget` to expose some way to check "is this value in
     this `Choices`" (e.g. a method on the trait), plus a place to stash the last-fetched
     `Choices` where `validate()` can reach it synchronously — not designed in detail yet.

   Not attempted on either tier — noted as a to-do.

**If picking one thing to close next:** #1 (pluggable error rendering) or bounds on
repeating groups (part of the gap in "roughly at parity" above) — both are additive to
what's already built, unlike the submission-flow or live-validation gaps, which would
be architectural changes. Separately, #7's server-side tier (a scoped-membership check
on `question.source` before insert, mirroring the provider's own course-scoped query) is
a small, independent fix worth doing regardless of whether formoxus itself ever grows
the client-side caching half.
