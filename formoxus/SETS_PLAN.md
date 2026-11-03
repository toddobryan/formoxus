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
  The items show in one canonical order, which makes it easy to see what
  is and isn't there, and a duplicate is a mistake.

Django splits the same way, though not as fields (a Django form is flat;
repeats are formsets). Ordered lists: core formsets with `can_order` (a
typed `ORDER` rank per row) and django-sortedm2m's `SortedManyToManyField`
(drag and drop). Sets: core `MultipleChoiceField` (tick boxes, choices
order) and django-taggit's `TagField`, whose parser dedups and sorts: "a
sorted list of unique tag names". None lets a user edit a set's item in
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

E. **An item is edited through the adder, so nothing moves while the
   user types.** Each item has Edit and Remove. Edit copies the value into
   the adder's text box (Claude's refinement: and marks the item as being
   edited, instead of removing it straight away). Adding the edited text
   replaces the original; cancelling leaves the original as it was. So a
   user who clicks Edit and then submits loses nothing.

F. **Display is customizable.** The default is a plain `<ul>`; an inline,
   alphabetized row of tags is a matter of classes (CSS), and an item's
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
over those. That covers tags, it makes display-only items natural, and
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
   with `case_insensitive` both `Apple` and `apple` can be items, as a
   `BTreeSet<String>` would also hold both.

I. **The duplicate message: `on_duplicate`, a closure with a default.**
   The set calls it when an `AddRow` would add a duplicate. The default
   puts a message into `AddRowProps`, which an adder is expected to show,
   so formoxus works with nothing configured. A dev replaces it in `form!`
   (`on_duplicate: …`); apcsp will, with one calling its `add_message` once
   it has messaging.

J. **A set keeps its items' typed values**, if that is not too hard, so
   an item rebuilt from the wire sorts as `T` without reparsing its raw
   string every time. If it turns out hard, parsing as `build` does is the
   fallback.

K. **`set`** is the override's name: `tags => { set }`.

L. **`sort_by`** is the comparator key: `sort_by: case_insensitive`.
   (2026-11-01) Its value is any expression of type `fn(&T, &T) ->
   Ordering` (a fn, `String::cmp`, a non-capturing closure, a `const`),
   optionally paired with a name: `sort_by: (String::cmp, "ascending
   byte-order")`. The name is any expression, as `label:`'s is; without
   one it is `stringify!` of the order. A two-element tuple is always
   `(order, name)`, which cannot be ambiguous since a tuple is never a
   `fn` pointer; any other tuple is a parse error. The name is
   `SetOrder`'s, for its `Debug`.

M. **Classes:** `fx-set-items` on the `<ul>`, `fx-set-item` on each
   `<li>`, `fx-button-edit-item` on Edit. (Written `member` until
   2026-11-01, when box 11 made "item" the one word.)

N. **Separate nodes, shared rows.** `ListSet` becomes `ListNode` and
   `VariantSet` becomes `VariantNode`, like `OptionNode` and `FieldNode`:
   their "Set" meant fieldset, which reads wrong once a set is a real
   concept. A set gets its own `SetNode` rather than a kind switch on the
   list's node, because the two differ in more than they share (a list:
   moves, a blank Add, rows edited in place; a set: sorting, duplicates,
   edit by draft, typed items, a `<ul>`, `init_set`). What they DO share
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
    those are fixed when work moves back to apcsp (box 12), per Todd.

  - [x] **2c.** (Todd) `nodes/items.rs`: a `pub(crate) struct Items`
    that `ListNode` holds, and `SetNode` will. No behavior changes: `just
    test` gives exactly what it gives now (the ten 3b failures), and the
    three `parse_row_keys` tests pass in their new home.

    Done 2026-11-01, with every field private and the names already
    items (`add_item`, `ItemValue`, `ITEM_SIGIL`, `item_segment`,
    `parse_item_keys`). One thing deferred: `Items::edit_within`. The
    forwarding loop is still in `ListNode::edit`, to move when `SetNode`
    (2d) needs the same loop.

    **Moves into `Items`:**

    1. The fields `shape`, `rows`, `next_key`, `rejected_keys`, and
       `errors` (the list's own: a set's work the same way, and the
       rejected-keys message is one of them). `ListNode` keeps `name`,
       `label`, `optional` and `adder`.
    2. `Items::new(shape, rows)`, setting `next_key` to `rows.len()`, so
       `build.rs`'s `list_node` (the one place a `ListNode` is built) no
       longer restates the never-reuse-a-key rule.
    3. `add_row` → `Items::add`: the row-type check, the bounds check on
       `before`, the next key, the insert, the returned path. A set calls
       it with `before: None` after its own duplicate check, then sorts.
    4. `new_row`, `set_rows`, `remove_row`, as they are.
    5. `parse_row_keys` and its three tests.
    6. The halves of the `Node` methods that only walk the rows and the own
       errors, so each node's method becomes a delegation:
       `collect_values` (the `#0,#2` entry, then each row),
       `distribute_values` (parse the keys, `set_rows` or `rejected_keys`,
       then each row), `collect_errors`, `distribute_errors`,
       `clear_errors`, `validate` (clear, the rejected-keys message, each
       row), `has_errors_here`, `has_errors_within`, `is_present`, and
       `edit`'s second half, forwarding to the row that owns the path, as
       `Items::edit_within`, with its prefix-trap comment.

    **Stays in `ListNode`, as list-only:** `move_row`; `edit`'s first half,
    the match on an edit aimed at the list itself (a set has its own,
    refusing `MoveRow` and a blank `AddRow`); `rows()` and `render`, with
    the `<ol>`, ↑/↓ and the `List*` classes; `write_value_into`
    (`init_list`, where a set's is `init_set`: a four-line loop, not worth a
    parameter); `name`, `label`, `raw_value`.

    **Not shared yet:** building the adder (`AddRowProps` and the
    `VComponent`). A set's props grow a duplicate message (I) and a draft
    (box 8), so sharing it before `SetNode` exists would be guessing.

    **Unchanged:** the store cleanup on `RemoveRow` in `form.rs` finds the
    row through `collect_values`' entry, so any node writing that entry
    gets it.

  - [ ] **2d.** (Todd) `field_kind::is_set` (`Def::Set`), and a `SetNode`
    holding the same rows: built in the shape walk, read and written
    through `init_set`/`begin_set_item` in `build`, and its items in
    `distribute_values`/`collect_values`. A `MoveRow` or a blank `AddRow`
    aimed at a set is refused with an error that says why.

    2026-11-01: `SetNode` (`nodes/set_node.rs`) and `Items::edit_within`
    are in. Left: `kind()` returning `Kind::Set` for `Def::Set`,
    `node_for_shape` calling `set_node`, `set_node` building a `SetNode`
    (it builds a `ListNode`), and `with_adder` accepting a set. Known and
    accepted (Todd): until box 7, a set's default Add sends a blank
    `AddItem`, which the set refuses, so it only logs.

  - [x] **2e.** (Todd) `SetNode` renders a `<fieldset class="fx-set">` with
    its legend, a `<ul class="fx-set-items">` of display-only
    `<li class="fx-set-item">`s, each with Remove (Edit comes in box 8),
    then the adder. Like a list, no `<ul>` when the set is empty.

    2026-11-01: with the `FixedKey`s `Set`, `SetLegend`, `SetItems` and
    `SetItem` (`set`, `set_legend`, `set_items`, `set_item`), which no box
    had listed: decision O named `Set` and `SetLegend`, and M's classes
    were left to box 9.

  - [ ] **2f.** (Claude) `form!`: `set` on a `Vec` of leaves (K); a compile
    error for `set` on a set type ("already a set"), on a non-list, or on
    a `Vec` of non-leaves. Parse tests and goldens. After 2g.

    - [x] **2f1.** `adder:` on a set: the `const` check becomes
      `is_list(..) || is_set(..)`, both messages say "list or set" (no
      article, as `with_adder`'s does), the parse tests and the two
      `form_adder_*` goldens follow, and a suite test that a set takes
      `adder:`. (Done 2026-11-01: "applies only to list or set fields";
      the `[]` message says "every ITEM of one". The suite test is
      `sets::a_set_takes_a_custom_adder`, now rendering both `with_adder`
      and `form!`; dropping `|| is_set` fails the suite's compile.)

    - [x] **2f2.** The bare flag `set`, with a near-miss suggestion, and
      its parse tests. (Done 2026-11-01: `FieldBody::set: Option<Ident>`,
      the key kept for 2f3's spans; refuses a value and a second `set`; a
      field named `set` still nests. Parsed but not yet expanded, so until
      2f4 a `set` does nothing.)

    - [x] **2f3.** Its three `const` checks, each spanned on the key and
      with a golden: not already a set ("already a set"), a list, and a
      list of leaves. The last needs a `const fn is_list_of_leaves` in
      `field_kind` (B below). A leaf is a string, number, bool, unit
      enum, or newtype over one; an enum with data is not. (Done
      2026-11-01, Claude writing `is_list_of_leaves` at Todd's go. Also
      not a leaf: an `Option` item, since a set refuses a blank one, so
      its `None` could never be added. An opaque newtype counts, as
      `Kind::Unknown` does everywhere. The checks are chained so only the
      first failure reports; goldens `form_set_on_a_set`,
      `form_set_on_a_string`, `form_set_on_a_list_of_structs`.)

    - [x] **2f4.** The expansion, `.treat_as_set("path")`, against `pub fn
      treat_as_set(self, path: &str) -> Self` (agreed 2026-11-01 as `with_set`, renamed 2026-11-03). Waits on
      box 5's builder: until it exists, nothing using `set` compiles.

  - [x] **2g.** (Claude) Tests for 2d–2e: a `BTreeSet<String>` and a
    `HashSet<String>` round-trip, a set's markup, a refused move and blank
    add, a removed item leaving the store, and a set behind an `Option`
    and inside an enum row. Until box 4 sorts, a `HashSet` test compares
    as a set or uses one item, never iteration order.

    - [x] **2g1.** Round-trips, `BTreeSet<String>` and `HashSet<String>`:
      `form_for` then `validate`, and across the wire with
      `Submission::accept`.

    - [x] **2g2.** Markup: `fx-group fx-set`, its legend, the
      `fx-set-items` `<ul>` of `fx-set-item` `<li>`s, each with Remove; no
      ↑/↓; no `<ul>` when empty, but still the adder.

    - [x] **2g3.** Edits: `MoveItem` and a blank `AddItem` refused with
      messages that say why, an `AddItem` with a value accepted, one of
      the wrong type refused.

    - [x] **2g4.** A removed item's store entries go, as a list row's do.

    - [x] **2g5.** A set behind an `Option`, and inside an enum row.

    - [x] **2g6.** `with_adder` accepts a set.

    Done 2026-11-01, in `tests/suite/sets.rs` (15 tests). `with_store_keys`
    and `store_keys` moved from `vecs.rs` up to `suite.rs`, shared. Checked
    by building a set as a `ListNode`: 12 fail; the 3 that still pass test
    what the two share (store cleanup, custom adder, no empty container).
    The markup tests leave what an item renders as alone: until box 8 it
    is the leaf's own, editable, widget (Todd: "leave it in a broken
    state").

- [ ] **3.** Every list is ordered.

  - [x] **3a.** (Todd) `ListNode` always renders `<ol>` and ↑/↓;
    `FieldSpec.allow_reordering` and `with_allow_reordering` go.

  - [x] **3b.** (Claude) `form!`: the `allow_reordering` key goes, so
    `list_checks` keeps only `adder`. The two `form_allow_reordering_*`
    goldens go (the `[]` one became `form_adder_on_a_lists_rows`), the
    `specs.rs` panics move to `with_adder`, `vecs.rs`'s reorderable-list
    tests become every-list tests, `/t/reorder` loses the key, and the
    docs and memory say so. Ten `vecs.rs` tests fail until 3a, on purpose.

  - [x] **3c.** (Claude, after 3a) Re-bless any snapshot that gains an
    `<ol>` and ↑/↓, check those ten go green, run `just e2e`.

    Done 2026-11-01. Three snapshots re-blessed; an empty list now renders
    an empty `<ol>`. Two `vecs.rs` counts grew to take in ↑/↓, plus a new
    `only_the_ends_of_a_list_are_disabled` over lists of 1, 2 and 3 rows.
    The three tests that took "the first new click listener" as Remove
    now find it by name, with a new `Harness::labelled`, since one row
    brings three buttons at once.

- [ ] **4.** The order (G).

  - [x] **4a.** (Todd) An erased comparator, `SetOrder`, built from a
    `fn(&T, &T) -> Ordering`, and the default chain. The set sorts its
    items with it once, however the order was chosen. Keeps the typed
    values, per J.

    **What gets sorted is the `Items` themselves, not a view in
    `render`** (Todd, 2026-11-01), because Remove addresses an item by
    its position on screen, and a sorted view over unsorted items would
    remove the wrong one. And it happens wherever items arrive, on the
    server as much as the client:

    1. In `set_node`, BEFORE the keys are assigned. A `HashSet` iterates
       in an order that differs between processes, so assigning `#0`,
       `#1`, … in iteration order would give the server's render and
       the client's hydration different keys for the same values.
    2. After `add_item`.
    3. After `distribute_values`, once the leaves are filled, not straight
       after `set_items`, whose new items are still empty. This is the
       server's case: it rebuilds the items in wire order and renders
       them again when a submit fails.

    **Which items sort by default** (decided 2026-11-01): only SCALAR
    items, `String`, the integers, `bool` and the floats. Every other
    item (a newtype, a unit enum, a struct, an `Option`, an enum with
    data) needs a `sort_by`; for an `Ord` type that is one line,
    `sort_by: Tag::cmp`. A spec without one panics in `set_node`, since a
    hand-built `FormSpec` broke the contract; `form!` catches it at
    compile time for the fields it names (4c4), but it never walks the
    model, so an unnamed set field reaches only the panic.

    Why: the build-time sort sees whole items as `Peek`s, and the later
    two see nodes, which can give back only what a leaf holds
    (`Node::peek_value`, built 2026-11-01). For a scalar the two agree.
    A newtype's leaf holds its INNER value, and cannot re-wrap it without
    a `T` (`alloc_shape` is `unsafe`, denied workspace-wide); a
    `VariantNode` or `StructNode` holds no value at all. The unwrapping
    is forced, not a mistake: `dispatch!` turns a shape into a type only
    for the scalars it knows. With `T` from a `sort_by`, every kind of
    item works safely.

    So `SetOrder` wraps `Arc<dyn ItemOrder>`, a trait with
    `sort_peeks(&mut Vec<Peek>)` (build, before keys) and
    `sort_nodes(&mut Vec<Box<dyn Node>>)` (after add and distribute),
    plus `name()` for `Debug`; `PartialEq` is `Arc::ptr_eq` and the name.
    Two impls:

    - **`DefaultOrder`**, scalars only: `partial_cmp` (total, since every
      non-float scalar is `Ord`), `total_cmp` for `f32`/`f64`. No `Debug`
      fallback: nothing reaches it any more. `sort_nodes` reads
      `peek_value`; an item without one (`Empty`, `Invalid`) goes last,
      in place (`Items::sort_by` does this today).
    - **`Custom<T>`**, from `set_order_for` (4c): `sort_peeks` compares
      `peek.get::<T>()`; `sort_nodes` rebuilds each item's `T` ONCE with
      the safe `Partial::alloc::<T>()` + `write_value_into` + `build` +
      `materialize::<T>()`, then sorts by the dev's fn. It must skip an
      item with `has_errors_within()` (or no value) before rebuilding:
      `FieldNode::write_value_into` hits `unreachable!` on an `Invalid`
      leaf. Skipped items go last, in place.

  - [x] **4b.** (Todd) `ascii_case_insensitive` and
    `unicode_case_insensitive` in `nodes/set_node.rs` (decided 2026-11-03:
    no `sets` module), re-exported from `nodes` and in the prelude. Each is
    generic over `S: AsRef<str>`, so the one fn instantiates at whatever
    `T` 4c infers (`String`, `Box<str>`, a newtype with `AsRef<str>`).
    Built on `unicase` (`Ascii`, `UniCase`), and TIE-BROKEN by plain
    `str::cmp`: a set may hold both "Rust" and "rust", and an `Equal`
    leaves them in arrival order, which for a `HashSet` differs between
    server and client and splits the keys on hydration.

  - [x] **4c.** (Claude) `form!`: `sort_by: path::to::comparator`, only on
    a set, its `T` inferred from the field. Waits on 4a's builder.

    **As built (2026-11-03), two changes from the sketch below**, both found
    in the goldens:

    1. TWO calls, `set_items_of(|__m: &Post| &__m.tags).sort_by(order,
       name)`, not one `set_order_for(projection, order, name)`. rustc
       checks a call's non-closure arguments before its closures, so with
       one call a fn of the wrong type pinned `T` first and the mismatch was
       reported on the projection, i.e. across the whole `form!`. A method
       on the result is checked once `T` is known: the error lands on the
       order, and an un-annotated closure still infers.
    2. The projection reaches the SET, not one item via `row`, and
       `set_items_of` takes `S: SetLike` (`HashSet`, `BTreeSet`, `Vec`),
       a trait with `#[diagnostic::on_unimplemented]`. So `sort_by` on a
       `String` is ONE error at the key, not a bare "`&String` is not an
       iterator" across the `form!` beside the `const` one. The `const`
       check is now only `!is_list` (a `Vec` without `set`). A path ending
       in `[]` gets its hint as the trait's `note`.

    An unnamed order is named by the macro, from its tokens with the
    spaces tidied (`String::cmp`), since `stringify!` of tokens that have
    passed through a proc macro prints `String :: cmp`. The 4c4 check is
    asked beside `set` only of a list of leaves, so `set`'s own refusals
    are not doubled.

    **Open, for 2f4:** `tags => { set, sort_by: … }` on a `Vec` passes
    every `const` check and then panics at run time, since `with_sort_by`
    asserts `is_set`. It must also accept a field marked `set` once
    `treat_as_set` exists.

    Checked by TYPE, not `const` (a `Shape`'s `ConstTypeId` wraps a fn
    const code cannot call). The expansion is

    ```rust
    ::formoxus::field_kind::set_order_for(
        |__m: &Post| ::formoxus::field_kind::row(&__m.tags),
        by_length,
        ::core::stringify!(by_length),
    )
    ```

    against `pub fn set_order_for<M, T: Facet<'static>>(_: fn(&M) -> &T,
    order: fn(&T, &T) -> Ordering, name: …) -> SetOrder`, which also
    builds the erased comparator (`peek.get::<T>()` on each side). A
    wrong `T` is rustc's mismatched-types error at the order. An
    `Option<BTreeSet<_>>` is written `tags? => { sort_by: … }`, per the
    2026-10-25 explicit-`?` rule, and peeled through `some()`; no trait
    looks through `Option`.

    - [x] **4c1.** The key, with `(order, name)` (L), the near-miss and
      given-twice tests, and the bad-tuple error.

    - [x] **4c2.** The expansion, and a `const` check that the field is a
      set or carries `set`, with a golden.

    - [x] **4c3.** Goldens for a mismatched `T` and a bad tuple; suite
      tests for a fn, `String::cmp`, an un-annotated closure (pins that
      the item type is inferred before the closure is checked), and a
      named order's `Debug`.

    - [x] **4c4.** A `const` check that a set's items are scalars when no
      `sort_by` is given: on every named field that is a set, or carries
      `set`, without `sort_by`. Spanned on the path (or on `set`), with a
      golden. Message along the lines of "the items of `tags` have no
      default order: give `sort_by: …` (for an `Ord` type,
      `sort_by: Tag::cmp`)". Needs a `const fn` in `field_kind` asking
      whether the item shape is a scalar, written directly rather than
      through `kind`, which peels both `Option` and transparent newtypes,
      exactly what must NOT count here. Suite tests that a newtype and a
      unit-enum set sort by their `sort_by`, before and after a submit.

- [ ] **5.** A `Vec` treated as a set reaches the model sorted and
  deduplicated. (Todd, Claude tests)

  Expanded 2026-11-03. A `Vec` marked `set` is built as a `SetNode`, not a
  `ListNode`, so it gets the `<ul>`, Remove only, the refusals and the
  order for free. What is new is the `Vec` underneath: the node must peek
  it as a list, write it as a list, and drop duplicates itself, which a
  `BTreeSet`/`HashSet` does on insert. Sorting needs nothing new, since the
  items already sit in order (box 4) and are written in that order.

  **Where duplicates come from, and where each is dropped:**

  1. Edit mode, a model `Vec` that already holds one (`["rust", "rust"]`).
     Dropped at build, among the `Peek`s, before the sort and the keys, so
     the screen shows what will be saved.
  2. An `AddItem` whose value equals an item. Refused with a message: box 6,
     not here.
  3. The wire: a scripted or forged payload naming two items with equal
     values. Dropped in `distribute_values`, beside the sort, so a failed
     submit re-renders without them. This applies to a `Def::Set` too:
     until 5d, its write collapses them silently but its nodes keep both.

  **After 5d the screen and the model always agree** (asked by Todd,
  2026-11-03): every drop removes a NODE, and the model is written from the
  nodes. A browser submit distributes into the live state
  (`Form::validate`), so the dropped item leaves the page in the same step;
  a server submit distributes into a fresh state, and a failed one
  re-renders that state. The one way a user can still make a duplicate is
  retyping an item in place, which items allow only until box 8; after it,
  every new value comes through the adder, which box 6 checks.

  "Equal" is the model's own `PartialEq` (H), never the comparator, and an
  item with no value (`Empty`, `Invalid`) equals nothing. The FIRST of
  equal items stays.

  **Equality needs `T`, and only the item order has it.** A scalar item's
  leaf gives a `Peek` (`Node::peek_value`), so `Peek::partial_eq` works. A
  newtype's leaf gives its INNER value, and a unit enum's `VariantNode`
  gives nothing, the same wall 4a hit for ordering. But every non-scalar
  set already has a `sort_by` (4c4), so a `CustomOrder<T>` that rebuilds
  `T` is always there when `peek_value` is not enough. So equality goes
  beside the order: the erased per-item object learns `dedup_nodes`, and
  box 6's duplicate check asks the same object.

  - [x] **5a.** (Todd) `FieldSpec.treat_as_set: bool` and `FormSpec::treat_as_set(path)` (named so 2026-11-03, without `with_`: every `with_*` builder sets a value it is given, and this one takes only the path; `form!` keeps `set`),
    agreed for 2f4. It asserts what `form!`'s three `set` checks assert,
    for a hand-built spec: not already a set, a list, a list of leaves
    (`field_kind::is_list_of_leaves` is a `const fn`, callable at run
    time). And `with_sort_by` accepts a field marked `set` as well as a
    `Def::Set`, closing box 4's open item. On a `Vec` not (yet) marked, its
    panic says to call `treat_as_set` first (B).

  - [ ] **5b.** (Todd) `build`: a `Def::List` field whose spec has `set`
    goes to `set_node`, not `list_node`, found with
    `walk.spec_for(prefix)`. `set_node` peeks a `Vec` with `into_list` and
    a set with `into_set`, deciding from the peek's own shape. `SetNode`
    itself is unchanged: it keeps NO record of its container (Todd,
    2026-11-03), because 5e's write reads it from the `Partial`.

    Also here (Todd, 2026-11-03), since it is the same file: `rows_of`
    takes a `Def::Set`'s item shape as well as a `Def::List`'s, so a spec
    path through a set's items (`labels[] => { … }`) stops panicking with
    "not a list, so `[]` names nothing". Test in `specs.rs` (Claude); see
    the note under box 9.

  - [x] **5c.** (Claude, at Todd's request) Dedup at build (source 1): drop a `Peek` equal
    (`Peek::partial_eq`) to an earlier one, before `sort_peeks` and before
    keys are assigned, so the keys stay `#0..#n` with no gaps. A `Def::Set`
    never has one, so this runs for a `Vec` only.

    Done 2026-11-03 as `build::without_duplicates`, called on the list
    branch only. Pairwise, since `Peek` has no `Eq` and its `Hash` panics
    on a type without one. Two tests in `sets.rs`, both failing without it.

  - [x] **5d.** (Todd) Equality on the item order: `ItemOrder` is renamed
    `ItemHandler` (A), and gains
    `dedup_nodes(&self, nodes: &mut Vec<Box<dyn Node>>)`, called right
    after `sort_nodes` in `distribute_values` (source 3), not after
    `AddItem` (box 6 refuses those).

    - `DefaultOrder`: `peek_value` on each side, `Peek::partial_eq`.
    - `CustomOrder<T>`: `value_of` on each side (it already validates a
      clone first) and `T`'s own `==`, so `T: PartialEq` joins its bounds
      and `SetItems::sort_by`'s. Every `BTreeSet`/`HashSet` item has it
      already (`Ord`/`Eq` imply it); a `Vec` of a type without it cannot
      take `set` at all, which is right, since its duplicates could not
      be found.

    An item dropped here leaves its store entries behind, which
    `ordered_values` already ignores. Left so on purpose (Todd, 2026-11-03):
    only a browser submit has a store to leave them in, item keys are never
    reused so nothing can pick one up, `Form::reset` leaves stale paths on
    the same reasoning, and after box 8 a user cannot make a duplicate on
    the client at all.

  - [x] **5e.** (Todd) `SetNode::write_value_into` for a `Vec`: branch on
    `partial.shape().def`, which is the collection's own shape by the time
    the node writes (`write_into` has called `begin_field`, and an
    `OptionNode` `begin_some`). `Def::List`: `init_list`, then
    `begin_list_item` / the item's write / `end` per item, in node order;
    otherwise `init_set` as now. No sort and no dedup here: 5c and 5d have
    done both by the time anything writes.

  - [ ] **5p.** (Claude, after 5d) Time large sets, for J (C): a
    `BTreeSet<String>` and a `Vec<Tag>` with `set` and `sort_by`, at 100,
    1,000 and 10,000 items, through build, `AddItem`, and a
    `distribute_values` from the wire. The suspects are `CustomOrder`
    rebuilding every `T` on each sort (`value_of` validates a CLONE of each
    item first), and `dedup_nodes`, which is pairwise, O(n²), since `T` is
    only `PartialEq`. A comparator need not agree with `==` (a by-length
    order puts "ab" and "cd" together), so sorting first and comparing
    neighbours would miss duplicates. Report the numbers and whether J (or
    a cheaper dedup) is worth building; nothing to fix in this box.

  - [x] **5f.** (Claude) 2f4: `form!`'s `set` expands to
    `.treat_as_set("path")`, emitted BEFORE `.with_sort_by(…)` so 5a's check
    sees the mark. Parse test for the order of the two calls.

  - [x] **5t.** (Claude) Tests, in `sets.rs` unless noted. (All done 2026-11-03; the dedup tests fail without `Items::dedup`.)

    1. A `Vec<String>` with `set` (through `treat_as_set` and through `form!`)
       renders a `<ul>`, Remove only, no ↑/↓, and refuses `MoveItem` and a
       blank `AddItem`, as a `BTreeSet` does.
    2. Edit mode, `["rust", "dioxus", "rust", "css"]`: built as three items
       in order, keys `#0,#1,#2`, and `validate` gives
       `vec!["css", "dioxus", "rust"]`.
    3. `AddItem` of a new value lands in order, and the model `Vec` follows.
    4. The wire with two equal values: one item after `distribute_values`,
       and the model has it once. The same for a `BTreeSet`, whose nodes
       keep both today (source 3).
    5. A `Vec<Tag>` with `set` and `sort_by`: sorted, deduplicated by
       `Tag`'s `==` (not the comparator: two tags of equal length both
       stay).
    6. An `Empty` item is not a duplicate of another `Empty` one: both
       stay, and the submit fails on them as it would anyway.
    7. `Option<Vec<String>>` with `set`, as `tags? => { set }`.
    8. `treat_as_set`'s three refusals as `should_panic` (`specs.rs`, beside
       `with_adder`'s), and `with_sort_by` on a `set`-marked `Vec`. (Done
       2026-11-03 with 5f: 8 spec tests, plus `form!`'s `set` building the
       same spec as `treat_as_set` and `set, sort_by:` on a `Vec` building
       at all.)
    9. `Vec<Tag>` with `set` and no `sort_by`: already a golden
       (`form_set_flag_without_sort_by`); add the run-time panic for a
       hand-built spec.

  **Decided (Todd, 2026-11-03):**

  A. `ItemOrder` becomes **`ItemHandler`**, since it orders and dedups (and
     answers box 6's duplicate check).

  B. **`with_sort_by` on an unmarked `Vec` panics with "call `treat_as_set`
     first".** Formoxus optimizes for `form!`, which emits the calls in the
     right order; a dev building a `FormSpec` by hand has to build it
     correctly.

  C. **J stays open**, pending 5p's timings of large sets.

  D. **A duplicate from the wire is dropped quietly, not refused.** In a set
     it carries no information, and `BTreeSet::from_iter` and serde both
     drop one silently, so formoxus is exactly as strict as the model type.
     A script is not told; the first of equal items (`"01"` before `"1"` in
     a `u8` set) is the one kept. Refusing is box 6's job, for the adder,
     where a person can act on the message.

- [ ] **6.** Duplicates (C): an `AddRow` whose value equals an item is
  refused, with a message beside the adder, per H and I. (Todd,
  Claude tests)

- [ ] **7.** The default adder for a set: a text box and Add for a
  string-like `T`, a select over the choices for a unit enum or
  `choices:`. (Todd, Claude tests)

- [ ] **8.** (Decided 2026-11-01: Edit MARKS the item, not removes it,
  so a user who clicks Edit and then submits loses nothing, and "the top"
  is the adder's text box.)
  Edit through the adder (E): a draft the set owns, handed to
  the adder (`AddItemProps` gains an optional `draft`; a list's adder
  ignores it); the item being edited is marked; Add replaces it
  atomically; Escape or Cancel restores it; the adder's text box takes
  focus. Edit appears only when the adder takes text. (Todd, Claude tests)

- [ ] **9.** (Claude) Display (F): a tags preset in
  the gallery's CSS, and a test that an item's row widget renders.

  **Found 2026-11-03, needed first:** a spec path through a set's items
  (`tags[] => { widget: … }` on a `BTreeSet`) panics, "`tags` is a
  `BTreeSet<String>`, not a list, so `[]` names nothing": `rows_of`
  (`build.rs`), which checks a path's `[]` segments, accepts only
  `Def::List`. `form!` does not catch it, since its projection's `.iter()`
  compiles on a set, so it is a run-time panic for a path that is right. The
  fix is `rows_of` taking a `Def::Set`'s item shape too, folded into 5b
  (Todd), with a `specs.rs` test (Claude). (Done 2026-11-03: renamed
  `items_of`, message now "not a collection"; test
  `specs::brackets_on_a_set_name_its_items`.)

- [ ] **10.** (Claude) End to end: `/t/tags` and its e2e tests (add, a
  duplicate's message, edit then add, edit then cancel, the submitted
  order), and a gallery example.

- [ ] **11.** (Claude) Rows become items, everywhere a dev or a Formoxus
  dev can see the word (Todd, 2026-10-31: "one term used internally and
  another externally just leads to confusion"). **Item**, after facet's
  `begin_list_item`/`begin_set_item`: not "element", which is Dioxus's
  `Element` and HTML's, nor "member", which was this crate's word for a
  node until 2026-10-16.

  Done in 2c (2026-11-01): `add_row`, `new_row`, `set_rows`,
  `remove_row`, `move_row`, `parse_row_keys`, `row_segment`, `ROW_SIGIL`,
  the `rows` field, all of item 2, `RowValue` → `ItemValue`, and the
  runtime messages. What is left in items 1, 3 and 6 is the rest.

  1. Internals: `row_path`, `new_blank_row` (both `form.rs`), and
     `widgets/row_buttons.rs`.
  2. [x] The edits: `Edit::AddRow`/`RemoveRow`/`MoveRow` → `AddItem`/
     `RemoveItem`/`MoveItem`.
  3. The widgets: `AddItemProps`' `rows` prop. Done 2026-11-01 (Todd):
     `AddRowProps` → `AddItemProps`, `RowAdder` → `ItemAdder`, and
     `AddItemButton`, `RemoveItemButton`, `MoveItemButtons`.
  4. The `FixedKey`s and their `classes_plus:` names: `ListRow`,
     `ListRows`, `AddRowButton`, `RemoveRowButton`, `MoveRowButtons`,
     `MoveRowUpButton`, `MoveRowDownButton`.
  5. The classes: `fx-list-row(s)`, `fx-button-add-row`,
     `fx-button-remove-row`.
  6. The messages: `form!`'s "a path ending in `[]` is every ROW", the
     runtime's "cannot remove row …", and the docs and tests around them.

  **Not touched:** `field_row` (a widget's caption/control/errors
  layout) and `ButtonRow`/`fx-button-row`, where "row" is a horizontal
  layout, not an item of a list.

- [ ] **12.** (Claude) Merge prep: `ORDERED_LISTS_PLAN.md` marked as
  superseded where it is (boxes 2–3), its boxes 6–8 checked against this
  plan, the memory updated, and `just ci` and `just e2e` green. apcsp
  builds formoxus by PATH, so it follows whatever branch is checked out
  here: its three `.fx-list-set` selectors (2b) become `.fx-list`, and
  its comment naming `ListSet` becomes `ListNode`.

## Not in this plan

- Sets of structs or of enums with data (Scope).
- A multiset, where order doesn't matter but duplicates are allowed. If
  apcsp ever needs one, it is a list for now.
