# Lists and sets

Written 2026-10-31 by Claude, from a conversation with Todd, for Todd to
build. It replaces `allow_reordering` (`ORDERED_LISTS_PLAN.md` boxes 2–3)
with a distinction that was missing: a **list**, whose order matters, and a
**set**, which has no duplicates and is shown in a canonical order. Both keep
box 5's `adder:`.

Who does what, as usual here: Todd writes the runtime; Claude writes the
macro side, the tests, the e2e forms, the docs and the gallery example.

**On a branch, as one breaking change** (D below): removing a `form!` key
breaks every form that uses it, and `main` keeps what works until the whole
change is ready.

## Why

`allow_reordering` asked a dev to say, for each `Vec`, whether its order
could be changed. The real question is whether its order **means**
anything:

- If it does (the questions in an assignment), the user needs a way to
  change it, so every list gets ↑/↓, not just the flagged ones.
- If it doesn't (tags), there should be no order for the user to manage.
  The members show in one canonical order, which makes it easy to see what
  is and isn't there, and a duplicate is a mistake.

Django splits the same way, though not as fields (a Django form is flat;
repeats are formsets). Ordered lists: core formsets with `can_order` (a
typed `ORDER` rank per row) and django-sortedm2m's `SortedManyToManyField`
(drag and drop). Sets: core `MultipleChoiceField` (tick boxes, choices
order) and django-taggit's `TagField`, whose parser dedups and sorts: "a
sorted list of unique tag names". None lets a user edit a set's member in
place, which is why E below edits through the adder.

## Decisions (Todd, 2026-10-31)

A. **Two kinds: list and set.** `allow_reordering` goes away. Every list
   renders as an `<ol>` with ↑/↓ on each row; a set renders as a `<ul>`.

B. **Type-driven, with one override.** `Vec<T>` is a list;
   `BTreeSet<T>`/`HashSet<T>` (facet's `Def::Set`) is a set. `set` in
   `form!` makes a `Vec` a set: the model then gets a sorted, deduplicated
   `Vec`. There is no override the other way, since a set type cannot be
   a list.

C. **A duplicate is refused with a message**, shown beside the adder.
   Silently ignoring it would make Add look broken.

D. **On a branch.**

E. **A member is edited through the adder, so nothing moves while the
   user types.** Each member has Edit and Remove. Edit copies the value into
   the adder's text box (Claude's refinement: and marks the member as being
   edited, instead of removing it straight away). Adding the edited text
   replaces the original; cancelling leaves the original as it was. So a
   user who clicks Edit and then submits loses nothing.

F. **Display is customizable.** The default is a plain `<ul>`; an inline,
   alphabetized row of tags is a matter of classes (CSS), and a member's
   own markup is a row widget.

G. **The order: one rule for every type, no exception for strings.**
   Tried in turn:

   1. The dev's comparator, if `form!` names one.
   2. `T`'s own `PartialOrd`, through facet (`Peek::partial_cmp`;
      the vtable's `has_partial_ord`). Numbers sort as numbers, and a unit
      enum deriving `PartialOrd` sorts in declaration order.
   3. `T`'s `Debug` string, when `T` has no `PartialOrd` or
      `partial_cmp` returns `None` (a float NaN).

   So strings sort by `Ord`, which is byte order: `Zebra` before `apple`.
   For tags that is usually not what a dev wants, so formoxus exports a
   `case_insensitive` comparator, and using it is one key in `form!`.

## Scope

**Sets of leaves only, for now:** strings, numbers, unit enums, newtypes
over those. That covers tags, it makes display-only members natural, and
"a set of structs sorted by …" can wait until something needs one. A `set`
on a `Vec` of structs or enums with data is a compile error.

## Checked already (2026-10-31)

- facet-core 0.46.5: `HashSet` and `BTreeSet` are `Def::Set`
  (`types/def/set.rs`), whose vtable has `insert`, `contains`, `len`, an
  iterator and `from_slice`.
- facet-reflect 0.46.5: `Partial::init_set`/`begin_set_item` mirror the
  list API (`partial/partial_api/sets.rs`), and `Peek::partial_cmp` exists
  (`peek/value.rs:270`).
- formoxus does not walk `Def::Set` at all today.

## Settled (Todd, 2026-10-31)

H. **A duplicate is by the model's own equality** (facet's `PartialEq`
   through `Peek`), never the comparator. The comparator only orders, so
   with `case_insensitive` both `Apple` and `apple` can be members, as a
   `BTreeSet<String>` would also hold both.

I. **The duplicate message: `on_duplicate`, a closure with a default.**
   The set calls it when an `AddRow` would add a duplicate. The default
   puts a message into `AddRowProps`, which an adder is expected to show,
   so formoxus works with nothing configured. A dev replaces it in `form!`
   (`on_duplicate: …`); apcsp will, with one calling its `add_message` once
   it has messaging.

J. **A set keeps its members' typed values**, if that is not too hard, so
   a member rebuilt from the wire sorts as `T` without reparsing its raw
   string every time. If it turns out hard, parsing as `build` does is the
   fallback.

K. **`set`** is the override's name: `tags => { set }`.

L. **`sort_by`** is the comparator key: `sort_by: case_insensitive`.

M. **Classes:** `fx-set-members` on the `<ul>`, `fx-set-member` on each
   `<li>`, `fx-button-edit-member` on Edit.

N. **Separate nodes, shared rows.** `ListSet` becomes `ListNode` and
   `VariantSet` becomes `VariantNode`, like `OptionNode` and `FieldNode`:
   their "Set" meant fieldset, which reads wrong once a set is a real
   concept. A set gets its own `SetNode` rather than a kind switch on the
   list's node, because the two differ in more than they share (a list:
   moves, a blank Add, rows edited in place; a set: sorting, duplicates,
   edit by draft, typed members, a `<ul>`, `init_set`). What they DO share
   (row keys never reused, the structure entry on the wire, refusing a
   forged key, the store cleanup on `RemoveRow`, the adder, the list's own
   errors under its legend) moves into `nodes/rows.rs`, which both hold,
   so it is written once.

O. **The classes and `FixedKey`s that say "set" are renamed too**, since
   `fx-list-set` would read as "a list or a set": `FixedKey::ListSet`,
   `ListSetLegend`, `VariantSet`, `VariantSetLegend` become `List`,
   `ListLegend`, `Variant`, `VariantLegend`, beside a new `Set` and
   `SetLegend`; the classes `fx-list-set` and `fx-variant-set` become
   `fx-list` and `fx-variant`, beside `fx-set`. Breaking for anything
   styling the old names, which is why it is on this branch. Added the
   same day (Todd): `FieldSet` → `StructNode`, `FixedKey::FieldSet`(+
   `Legend`) → `Struct`(+`Legend`), class `fx-field-set` → `fx-struct`,
   since "field set" also reads as HTML's `<fieldset>`. Its `classes_plus:`
   name is `struct`, a keyword, so that selector parses with
   `Ident::parse_any`, as `buttons:` reads `type`.

## Boxes

- [x] **1.** Branch `sets`.

- [ ] **2.** Renames, the shared rows, and a set node (N, O). First,
  so 3a and everything after it is written against the new names.

  - [x] **2a.** (Claude, mechanical; Todd reviews) `ListSet` → `ListNode`,
    `VariantSet` → `VariantNode` and `FieldSet` → `StructNode`, their files
    with them (`nodes/list_node.rs`, `variant_node.rs`, `struct_node.rs`), and every comment,
    doc and memory that names them. Done while Todd is not editing those
    files. No behavior changes: the suite stays as it is (the ten 3b tests
    still red).

  - [x] **2b.** (Claude) The `FixedKey` and class renames (O), across
    `formoxus-spec`, `classes.rs`, the presets, the suite, the gallery's
    CSS and the memory. apcsp's SCSS is checked for `fx-list-set` and
    `fx-variant-set` first. It doesn't style them, but three e2e
    selectors in `crates/e2e/tests/questions.rs` use `.fx-list-set`;
    those are fixed when work moves back to apcsp (box 11), per Todd.

  - [ ] **2c.** (Todd) `nodes/rows.rs`: the keyed rows (keys never reused,
    `next_key`), the structure entry on the wire, the forged-key refusal
    and the list's own errors, moved out of `ListNode`, which then holds
    one. No behavior changes; the suite says so.

  - [ ] **2d.** (Todd) `field_kind::is_set` (`Def::Set`), and a `SetNode`
    holding the same rows: built in the shape walk, read and written
    through `init_set`/`begin_set_item` in `build`, and its members in
    `distribute_values`/`collect_values`. A `MoveRow` or a blank `AddRow`
    aimed at a set is refused with an error that says why.

  - [ ] **2e.** (Todd) `SetNode` renders a `<fieldset class="fx-set">` with
    its legend, a `<ul class="fx-set-members">` of display-only
    `<li class="fx-set-member">`s, each with Remove (Edit comes in box 8),
    then the adder.

  - [ ] **2f.** (Claude) `form!`: `set` on a `Vec` of leaves (K); a compile
    error for `set` on a set type ("already a set"), on a non-list, or on
    a `Vec` of non-leaves. Parse tests and goldens.

  - [ ] **2g.** (Claude) Tests for 2d–2e: a `BTreeSet<String>` and a
    `HashSet<String>` round-trip, a set's markup, a refused move and blank
    add, a removed member leaving the store, and a set behind an `Option`
    and inside an enum row.

- [ ] **3.** Every list is ordered.

  - [ ] **3a.** (Todd) `ListNode` always renders `<ol>` and ↑/↓;
    `FieldSpec.allow_reordering` and `with_allow_reordering` go.

  - [x] **3b.** (Claude) `form!`: the `allow_reordering` key goes, so
    `list_checks` keeps only `adder`. The two `form_allow_reordering_*`
    goldens go (the `[]` one became `form_adder_on_a_lists_rows`), the
    `specs.rs` panics move to `with_adder`, `vecs.rs`'s reorderable-list
    tests become every-list tests, `/t/reorder` loses the key, and the
    docs and memory say so. Ten `vecs.rs` tests fail until 3a, on purpose.

  - [ ] **3c.** (Claude, after 3a) Re-bless any snapshot that gains an
    `<ol>` and ↑/↓, check those ten go green, run `just e2e`.

- [ ] **4.** The order (G).

  - [ ] **4a.** (Todd) An erased comparator, `SetOrder`, built from a
    `fn(&T, &T) -> Ordering`, and the default chain. The set sorts its
    members with it once, however the order was chosen. Keeps the typed
    values, per J.

  - [ ] **4b.** (Todd) `formoxus::sets::case_insensitive`, exported in the
    prelude.

  - [ ] **4c.** (Claude) `form!`: `sort_by: path::to::comparator`, only on
    a set, its `T` inferred from the field.

- [ ] **5.** A `Vec` treated as a set reaches the model sorted and
  deduplicated. (Todd, Claude tests)

- [ ] **6.** Duplicates (C): an `AddRow` whose value equals a member is
  refused, with a message beside the adder, per H and I. (Todd,
  Claude tests)

- [ ] **7.** The default adder for a set: a text box and Add for a
  string-like `T`, a select over the choices for a unit enum or
  `choices:`. (Todd, Claude tests)

- [ ] **8.** Edit through the adder (E): a draft the set owns, handed to
  the adder (`AddRowProps` gains an optional `draft`; a list's adder
  ignores it); the member being edited is marked; Add replaces it
  atomically; Escape or Cancel restores it; the adder's text box takes
  focus. Edit appears only when the adder takes text. (Todd, Claude tests)

- [ ] **9.** (Claude) Display (F): the classes from M, a tags preset in
  the gallery's CSS, and a test that a member's row widget renders.

- [ ] **10.** (Claude) End to end: `/t/tags` and its e2e tests (add, a
  duplicate's message, edit then add, edit then cancel, the submitted
  order), and a gallery example.

- [ ] **11.** (Claude) Merge prep: `ORDERED_LISTS_PLAN.md` marked as
  superseded where it is (boxes 2–3), its boxes 6–8 checked against this
  plan, the memory updated, and `just ci` and `just e2e` green. apcsp
  builds formoxus by PATH, so it follows whatever branch is checked out
  here: its three `.fx-list-set` selectors (2b) become `.fx-list`, and
  its comment naming `ListSet` becomes `ListNode`.

## Not in this plan

- Sets of structs or of enums with data (Scope).
- A multiset, where order doesn't matter but duplicates are allowed. If
  apcsp ever needs one, it is a list for now.
