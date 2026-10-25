---
name: variant-paths-in-form-macro
description: "#15, 2026-10-25: `$Enum::Variant` path segments in form!/path! (enum NAMED so the checks can match), tuple fields by index, nested `prefix => { … }` blocks, and FormSpec refusing string paths that name no node"
metadata:
  type: project
---

**The grammar.** `data.$QuestionData::TrueFalse.text => { … }`; the key is
`data.$TrueFalse.text`, the runtime's own spelling (the enum is dropped).
Tuple fields are indices: `reading.$Reading::Pair.1`. `path!` takes the same
segments. A nested block shares a prefix:
`data => { label: "Type", $QuestionData::TrueFalse => { text => {…}, answer => {…} } }`,
flattened at parse (`form.rs` `flatten`) into one entry per path with keys of
its own, BEFORE the duplicate check, so flat + nested repeats are caught.

**Why the enum is named (Todd chose option A over B).** The checks are a
`match`: `probe` binds the field after the variant in a pattern
(`Q::TrueFalse { text: __v0, .. }`), `project` does the same with
`field_kind::other_variant()` in the catch-all arm. A pattern must name its
enum and `form!` sees no types. B (bare `$TrueFalse`, const name checks) would
lose the widget-`ACCEPTS` check, see [[const-shape-walk-blocked]].

**Gotchas hit:**
1. The catch-all arm carries `#[allow(unreachable_patterns)]`: a ONE-variant enum (apcsp's case) makes it unreachable, and that would warn in the dev's crate.
2. A tuple field binds as `{ 1: __v0, .. }` (braced pattern, numeric member), so no `_` padding count is needed.
3. `.0.1` lexes as the float `0.1`; `SpecPath::parse` splits it back.
4. A variant segment can't end a path or follow another variant (it has no node; the field holding the second enum is skipped). Nested-block prefixes MAY end in one, and their entries may start with one: `check_names_a_node` / `check_starts_at_a_field` / `under` split those checks.
5. In a body, a nested entry is told from a key by `starts_nested`: `$`, an index, or a name followed by `=>`/`.`/`[`. So a field named `widget` nests fine.

**Runtime check (option C).** `FormSpec::field` (every `with_*` funnels through
it) calls `build::spec_key_error(T::SHAPE, path)` and PANICS with why, like
`with_attrs` already did. It mirrors the node walk: `Option` transparent,
newtype one leaf, `name[]` rows, `$V` enters a variant, a top-level enum's
fields carry no `$V`. A `#n` row key is refused with a pointer to `[]`.

**Top-level enum models: DROPPED (Todd, 2026-10-25).** Naming the enum makes the
macro side trivial, but the runtime is broken for them (`empty_form` panics in
`fields_from_enum`, no root variant picker, write fails "must select variant
before selecting enum fields", flat keys collide across variants). Devs wrap the
enum in a one-field struct. Don't re-raise.

**`Option` paths: `?` is REQUIRED on every `Option` in a path** (Todd,
2026-10-25: "explicit is better than implicit"), the last segment too, in
`form!` AND `path!`; the key drops it (string builder unchanged). A path spells
every container: `[]` list, `?` Option, in written order (`maybe?[]` vs
`rows[]?`). `addr? => { label: …, city => {…} }`: everything in the braces,
label included, is about the Option field. Rejected along the way: autoref
"peeling" (implicit), and `?` meaning "only if Some" (an `Option<Struct>`
always renders its fieldset, blank = None, so nothing is conditional on screen).

How each mistake is caught: `?` on a non-Option is a type error in
`field_kind::some(__p)` (arg bound first so ALL of it carries the field's span,
else rustc underlines the whole `form!`; witness and projection borrow one level
each so the error dedups). Missing `?` before `.field`/variant: rustc's own.
Missing `?` at the end or before `[]` (where `Option::iter` would make the whole
list the "row") type-checks, so `option_checks` emits a `const _` asserting
`!is_optional` on the place with every `?` peeled. The projection keeps the
final `Option` (widget/required checks need it).

Goldens: `form_path_through_an_option` (pinned the old limitation) deleted,
superseded by `form_option_struct_without_question`. The seven older goldens on
Option fields got a `?` added (2026-10-25, Todd approved the outputs) so each
still tests one thing; the missing-`?` error has its own goldens.
