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

I. **Superseded 2026-11-03 by I′ below.** **The duplicate message: `on_duplicate`, a closure with a default.**
   The set calls it when an `Edit::AddItem` would add a duplicate. The default
   puts a message into `AddItemProps` (`message`), which an adder is expected to show,
   so formoxus works with nothing configured. A dev replaces it in `form!`
   (`on_duplicate: …`); apcsp will, with one calling its `add_message` once
   it has messaging.

I′. **Every set edit says what it did** (Todd, 2026-11-03, replacing I).
   A set has one live region, after its adder: `<div role="status">`,
   ALWAYS rendered (a status region is announced reliably only if it is
   already in the page when its text changes), class slot
   `FixedKey::SetMessage` (`fx-set-message`, plus `fx-invalid` for a
   refusal). Every edit to the set replaces its text. Sets only, not lists
   (a list's Add makes a blank row, which says nothing). Wording copied
   from Django's admin, without its model name, since a set has only a
   plural label: "“rust” was added successfully.", "“rust” was removed
   successfully.", and for a duplicate "“rust” is already there." The item
   is named by its collected values (a newtype's inner value, a unit enum's
   variant), never by a `Display` it may lack. Success is visible in `fx()`;
   a dev who wants it announced only hides `.fx-set-message:not(.fx-invalid)`.
   NOT auto-dismissed (WCAG 2.2.1): it goes when the next edit replaces it.
   The dev's hook (BUILT 2026-11-03): `feedback:` in `form!`, or
   `FormSpec::with_feedback(path, SetFeedback::new(…))`, a CLOSURE (Todd)
   over a `#[non_exhaustive]`, `Copy` `SetEvent<'_, T>` (`Added`,
   `Removed`, `Duplicate`; box 8's join later) carrying the item as the
   dev's own `T`, rebuilt from its node. It returns the region's text, or
   `None` to show nothing, e.g. after handing the event to the app's own
   toaster (DioxusLabs' `dioxus-primitives` has one; formoxus depends on
   none). It must be `Send + Sync`, since the spec reaches the server
   (`Submission`), so it can capture a `use_signal_sync`, an `Arc`, or a
   global signal, but NOT a plain `use_signal`, whose storage is a
   `RefCell`.

J. (CLOSED 2026-11-03 by 5p's timings: not needed.) **A set keeps its items' typed values**, if that is not too hard, so
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

  - [x] **5p.** (Claude, after 5d) Time large sets, for J (C): a
    `BTreeSet<String>` and a `Vec<Tag>` with `set` and `sort_by`, at 100,
    1,000 and 10,000 items, through build, `AddItem`, and a
    `distribute_values` from the wire. The suspects are `CustomOrder`
    rebuilding every `T` on each sort (`value_of` validates a CLONE of each
    item first), and `dedup_nodes`, which is pairwise, O(n²), since `T` is
    only `PartialEq`. A comparator need not agree with `==` (a by-length
    order puts "ab" and "cd" together), so sorting first and comparing
    neighbours would miss duplicates. Report the numbers and whether J (or
    a cheaper dedup) is worth building; nothing to fix in this box.

    **Measured 2026-11-03** (native, release; wasm is typically slower by
    a small factor). Milliseconds, from a scratch crate outside the repo:

    | items  | case                         | build | add  | distribute | validate |
    |--------|------------------------------|-------|------|------------|----------|
    | 100    | `BTreeSet<String>`, default  | 0.05  | 0.01 | 0.13       | 0.03     |
    | 100    | `Vec<String>` set, default   | 0.07  | 0.00 | 0.10       | 0.02     |
    | 100    | `Vec<Tag>` set, `sort_by`    | 0.06  | 0.03 | 0.13       | 0.02     |
    | 1,000  | `BTreeSet<String>`, default  | 0.27  | 0.02 | 5.05       | 0.23     |
    | 1,000  | `Vec<String>` set, default   | 5.24  | 0.02 | 4.91       | 0.14     |
    | 1,000  | `Vec<Tag>` set, `sort_by`    | 4.57  | 0.27 | 2.11       | 0.16     |
    | 10,000 | `BTreeSet<String>`, default  | 3.93  | 0.31 | 463.15     | 4.33     |
    | 10,000 | `Vec<String>` set, default   | 373.06| 0.31 | 464.72     | 2.42     |
    | 10,000 | `Vec<Tag>` set, `sort_by`    | 236.26| 2.97 | 112.83     | 3.37     |

    Reading: everything is under 6 ms up to 1,000 items. The only cost that
    grows is the pairwise dedup: build (a `Vec` only) and distribute (every
    set) grow ~70–90× for 10× the items, and nothing else in them is
    quadratic. Rebuilding `T` is NOT the cost: the `CustomOrder` case,
    which rebuilds every `T` for its sort and again for its dedup but then
    compares with `T`'s own `==`, is the fastest at 10,000, because a
    `Peek`'s `==` goes through a vtable per pair. Add stays under 3 ms.

    **J CLOSED (Todd, 2026-11-03):** "plenty fast for reasonable sizes",
    and typed values would not touch the cost anyway. **If a large set ever
    matters**, the fix for the default order is exact and O(n): that order
    is the type's own `Ord` (`total_cmp` for floats), which agrees with
    `==` for every scalar (`-0.0` sorts right before `0.0`, `NaN` equals
    nothing), so equal items are neighbours once sorted and `dedup_by`
    after the sort finds them all. At build that means sorting before the
    dedup, through a `dedup_peeks` on `ItemHandler`. A custom order stays
    pairwise, since a comparator need not agree with `==`, but could
    compare with `T`'s own `==` at build rather than `Peek`'s.

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

- [x] **6.** Duplicates (C): an `Edit::AddItem` whose value equals an item is
  refused, with a message beside the adder, per H and I. (Todd,
  Claude tests)

  Where (2026-11-03): the check goes in `SetNode::edit`'s `AddItem` arm,
  before `Items::add_item`, asked of the `ItemHandler`
  (`ItemHandler::position`, through `Items::position_of`).

  **Built 2026-11-03** (Todd started, Claude finished): a duplicate returns
  `Ok(None)` (refused for the user, not the caller) and the set's region
  says so; per I′, an add and a removal say what they did too.
  `AddItemProps` carries no message: the set renders the region itself,
  whatever the adder. Tests in `sets.rs`, mutation-checked. The dev's hook
  (I′) built the same day: 6 more `sets.rs` tests, 6 `form!` parse tests,
  goldens `form_feedback_wrong_type` and `form_feedback_on_a_list`.

- [x] **7.** The default adder for a set: a text box and Add for a
  string-like `T`, a select over the choices for a unit enum or
  `choices:`. (Todd, Claude tests)

  **Decided 2026-11-03 (Todd): a DRAFT item, not a dedicated text box.**
  The set owns one blank item that is never one of its items, rendered
  inside the adder by the item type's own default widget: a text box for
  a `String`, a number input for a `u8`, a select for a unit enum or
  `choices:`. Add turns the draft into an item. Parsing, constraints and
  `tags[] => { … }` overrides then apply to the draft as to any item, with
  no adder-specific widget code, and box 8 fills this same draft instead
  of building one of its own. `tag_group` (dioxus-primitives) was read
  first: it displays and selects tags but has no adder, so it shaped
  box 9, not this.

  Settled the same day (Todd):
  1. The commit is `Edit::AddItem { value: None }`, not a new edit: "add
     what the adder holds", a blank row for a list and the draft for a
     set.
  2. Text left in the draft at submit is ignored: it was never added, so
     it was never in the set.
  3. The draft's label is "Add to {set label}" ("Add to Tags"), unless
     `tags[] => { label: … }` gives one.
  4. A select draft does NOT leave out the choices already in the set,
     yet: picking one gets the duplicate message. Filtering needs the
     set's items passed down to a leaf widget; later.

  Found while planning: what is submitted comes from the node tree's walk
  (`FormState::ordered_values`), never from the DOM, so the draft is kept
  off the wire by the walk alone, whatever its input's `name` says.

  - [x] **7a.** The draft: `SetNode` gains `draft: Box<dyn Node>`, built
    blank by `Items::new_item` under a key taken from the same counter as
    an item's, so no item ever gets its key, and an `AddItem` with a value
    cannot land on it. `set_items`, which raises the counter to keys from
    the wire, gives the draft a fresh key if a wire key took its key (on a
    server, whose draft was built before the keys arrived).
  - [x] **7b.** Off the wire, in the store: the set's walk reports the
    draft's leaves as a third kind, `RawValue::Draft`. `leaf_values` keeps
    them (so the store is seeded with them, and `reset` clears what was
    typed), and `ordered_values` drops them (so neither `to_wire` nor
    `validate` sees them). The plan's recommendation; the alternative is a
    separate `draft_values` walk.
  - [x] **7c.** Committing the draft: `Edit::AddItem { value: None }` on a
    set (refused today, since a set has no use for a blank item). Before
    the edit, `apply_edit` distributes the draft's leaves from the store
    into it. `SetNode::edit` then validates the draft:
    1. Empty: nothing happens, no message (Enter in an empty box does not
       scold).
    2. Invalid (`300` in a `u8` set, a `max_length` broken): refused, the
       error shown by the draft's own widget, the text left for the user
       to fix.
    3. A duplicate: refused with I′'s message, the text left as it is.
    4. Otherwise the draft moves into the items under its key, the set is
       sorted, I′'s "added" message is set, and a new blank draft takes
       the next key. Nothing is copied in the store: the draft's leaves
       already sit at the item's path.

    `ItemHandler::position` takes a `Peek` today, which a node can give
    (`peek_value`) only for the default order; a custom order's
    `position` needs the draft as a node, through `item_value`.
  - [x] **7d.** The adder: `AddItemProps` gains `draft: Option<Element>`
    (the set renders the draft and hands it over; a list passes `None`).
    The set's default adder becomes a new `DraftAdder`: the draft, then an
    Add (`type="button"`) sending 7c's edit, and Enter in the draft caught
    (`prevent_default`) and sent as the same edit, since Enter in a text
    box would otherwise submit the whole form. Focus stays in the draft
    after Add: it is rendered in the same place, unkeyed, so Dioxus reuses
    its element. `adder:` still replaces `DraftAdder`, and an adder that
    sends `AddItem` with a value (a search-to-add) can ignore `draft`.
  - [x] **7t.** (Claude) Tests: each of 7c's four outcomes; the draft in
    neither `to_wire` nor `validate`; `reset` clearing it; an `AddItem`
    with a value never taking the draft's key; a server's draft re-keyed;
    a `tags[] => { max_length: … }` refusing in the draft; a unit-enum
    set's draft a `<select>`.

  **Built 2026-11-03** (Claude, at Todd's request while he was away; Todd
  had planned to build it). Uncommitted. What turned up beyond the steps:
  1. The label travels on `NodeContext::label_if_unnamed`, read by
     `FieldNode` and `VariantNode` when their own label is `None`:
     a set's label case is known only at render, so the text cannot be
     baked into the draft at build. `nested` and `optional` drop it.
  2. The draft renders under `ctx.nested(..).optional()`, so it is never
     `required`: an empty required box would block every submit.
  3. A unit enum's draft is a `VariantNode`, so two more things: an edit
     inside the draft (its `ChooseVariant`) is routed to it by
     `SetNode::edit`, and the draft's STRUCTURE entries are dropped from
     the walk, not made drafts. In the store, `apply_edit`'s sync would
     read the choice back as `""` and unchoose it.
  4. `ItemHandler::position` takes a node now, not a `Peek`: the draft
     and an `AddItem`'s value (built by `Items::probe`) are asked the same
     way, and a custom order rebuilds both with `item_value`.
  5. `use_form` no longer focuses the item a set's Add made
     (`FormState::is_set`); `DraftAdder` puts focus back in the draft
     after a click.
  6. Every added item's key is one higher than before, since the draft
     holds one: 8 existing `sets.rs` expectations moved by one, and the
     two "refuses a blank item" tests became "adds nothing from an empty
     draft".

  Tests: 17 in `sets.rs` (`Harness::press` added for `keydown`), all 12
  mutations caught. **Browser-only, for box 10's e2e:** that Enter in the
  draft does not submit the form (`prevent_default`), and where focus
  lands after Add.

  **Known gap:** a draft whose text breaks a constraint attribute the
  browser checks (`pattern`, a number's `min`/`max`) blocks a
  browser-validated submit, although decision 2 says leftover draft text
  is ignored. The fix is probably the draft's input taking a `form`
  attribute naming no form, which also takes it out of a native submit;
  not done, since a leaf widget has no way to be given one yet.

  Reviewed (Todd, 2026-11-03): `label_if_unnamed` on `NodeContext` is
  fine. The known gap waits for box 10, whose e2e tests confirm it (or
  not) in a real browser first.

- [x] **8.** (Decided 2026-11-01: Edit MARKS the item, not removes it,
  so a user who clicks Edit and then submits loses nothing, and "the top"
  is the adder's text box.)
  Edit through the adder (E): box 7's draft, filled with the item's
  values; the item being edited is marked; Add replaces it
  atomically; Escape or Cancel restores it; the adder's text box takes
  focus. Edit appears only when the adder takes text. (Todd, Claude tests)

  **Built 2026-11-03** (Claude, at Todd's "Go ahead and write box 8").
  Decisions Claude made, all accepted by Todd the same day:
  1. **Items are shown, not edited in place**: an item renders as its
     text (`item_text`), then its own errors (a dev's validator or a
     server can still name an item), then Edit and Remove. Before, each
     item was its own editable widget.
  2. **One new edit, `Edit::EditItem { path, index: Option<usize> }`**:
     `Some(i)` starts editing item `i`, `None` stops (one variant, not
     two). It returns the draft's path, so `apply_edit` copies the draft's
     new values into the store and `use_form` focuses it. A list refuses
     it ("its rows are edited in place").
  3. **Filling the draft**: rebuilt blank under the key it holds, then the
     item's values, structure included (a unit enum's variant), are moved
     from the item's path to the draft's and distributed into it. Whatever
     was typed into the draft goes.
  4. **The mark**: `SetNode::editing`, the item's NAME, not its position,
     since Save sorts. Shown as Edit's `aria-pressed="true"`: Edit is a
     toggle, so a second press stops the edit. No new class; the gallery
     lights `.fx-button-edit-item[aria-pressed="true"]`.
  5. **Save** (renamed **Replace** in box 10): while editing, the adder's
     Add reads "Save" (check icon)
     and replaces the item in one edit, so the set never holds both. The
     draft becomes the item under its own key, as an add does, and the old
     item goes. Equal to the item being edited is that item saved
     unchanged, not a duplicate, and says nothing; equal to ANOTHER item is
     refused as a duplicate, and the edit goes on. The old item's store
     entries are left (5d's decision A).
  6. **Cancel**: a Cancel button (the form's `fx-button-cancel` slot)
     beside Save, Escape in the draft, or Edit pressed again. Escape does
     nothing when no edit is under way.
  7. **Remove while editing** the edited item ends the edit; the draft
     keeps the text, so Add would put it back as a new item.
  8. **Message**: `SetEvent::Changed { from, to }` joins the
     `#[non_exhaustive]` enum, and formoxus says "“rust” was changed to
     “go” successfully." (Django names only the new value; naming both
     says more.) Internally `NodeEvent` replaces `SetEventKind`.
  9. **Edit appears only beside `DraftAdder`**: a dev's adder may leave
     the draft out, and Edit would fill a box nobody sees.
  10. New: `FixedKey::EditItemButton` (`edit_item_button`, class
     `fx-button fx-button-edit-item`), Lucide `pencil` and `check` icons,
     `AddItemProps::editing`, `Harness::unlabelled`.

  Tests: 19 in `sets.rs`, all 14 mutations caught (one only after a
  rendered test was added: removing the edited item, which the state alone
  cannot show). **Browser-only, for box 10:** focus in the draft after
  Edit, Save and Cancel, and Escape not reaching anything else.

- [x] **9.** (Claude) Display (F): a tags preset in
  the gallery's CSS, and a test that an item's row widget renders.

  **Built 2026-11-03** (Claude, at Todd's "Go ahead and do 9"):
  1. A set's Remove is named by its item, "Remove “rust”"
     (`RemoveItemButton`'s new `text: Option<String>`); a list's keeps
     "Remove row N".
  2. New `FixedKey::SetAdder` (`set_adder`, class `fx-set-adder`): a
     `<div>` around `DraftAdder`'s draft and its buttons, which were loose
     siblings in the fieldset, so a preset can lay them out as one row.
  3. The gallery's `main.css`: a default for every set (a plain list, each
     item its text then Edit and Remove; the adder a row, Add not sticky),
     and the tags preset, chips in a wrapping row, opted into with
     `classes_plus: { set: + ["fx-tags"] }`. `fx-tags` is the gallery's
     class, not formoxus's.
  4. "An item's row widget" no longer meant what it did when this box was
     written: box 8 made an item display-only text, so the widget `tags[]`
     names now renders the DRAFT (test
     `the_items_widget_renders_the_draft`). So, decided with Todd the same
     day: **`item:` in `form!`** (`tags => { item: TagChip }`), or
     `FormSpec::with_item_view(path, ItemView(TagChip))`, names a
     component taking `SetItemProps` (`text`, the item's `path`, `index`,
     `editing`, `values`, `classes`; no typed `T`, since a set's items are
     single values, so the text IS the value). It replaces the item's TEXT
     only (Todd): formoxus still renders the `<li>`, the item's errors,
     Edit and Remove. Default `ItemText`. Sets only: `const` checks refuse
     it on a list without `set` and on anything else not a set (goldens
     `form_item_on_a_list`, `form_item_on_a_string`,
     `form_item_wrong_props`), and `with_item_view` goes through
     `set_field`'s panics. `SetNode::render_item` now renders one `<li>`
     (clippy's 100-line limit).
  5. NOT done: Delete or Backspace on a focused item (optional). It needs
     focus moved somewhere after the item goes, which only a browser can
     check.

  Tests: 10 in `sets.rs`, 5 `form!` parse tests, 3 goldens. The CSS is
  unchecked in a browser until box 10's gallery example shows a set.

  From `tag_group` in dioxus-primitives (read 2026-11-03, Todd agreed):
  1. A set's Remove names its item: `aria-label="Remove “rust”"`, by
     `item_text`, not "Remove row 2", which in a sorted set names nothing
     a user can see. A list keeps "Remove row N".
  2. Optional polish: Delete or Backspace on a focused item removes it.
     Not its roving-focus `role="grid"`, which exists for selecting tags,
     which a set's items do not do.

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

- [x] **10.** (Claude) End to end: `/t/tags` and its e2e tests (add, a
  duplicate's message, edit then add, edit then cancel, the submitted
  order), and a gallery example.

  **Built 2026-11-03.** `/t/tags` (a `BTreeSet<String>`) and
  `/t/tags-pattern` (its items `pattern: "[a-z]+"`, browser validation ON)
  in `examples/src/test_forms.rs`; 11 tests in `e2e/tests/tags.rs`, the
  full e2e suite 52/52. Proven in a browser: Add and Enter add, Enter does
  NOT submit, focus is in the draft after Add, Edit, Save and Cancel, a
  duplicate's message, Escape cancels, a submit while editing keeps the
  item, text left in the draft is not submitted, Remove is named by its
  item, and `{:#?}` of the submitted set is sorted. Three of those
  mutation-checked (Enter's `prevent_default`, the focus after an Add click,
  the focus after Edit).

  **The gallery example is DEFERRED** (Todd, 2026-11-03): gallery examples
  for both a set AND a list come together, once this plan is finished. See
  box 13.

  **Box 7's known gap, CONFIRMED:** with "Rust" left in a draft whose items
  have `pattern: "[a-z]+"`, the browser blocks the submit, although formoxus
  ignores leftover draft text. Pinned as
  `a_draft_breaking_its_pattern_blocks_the_submit_yet`. Now box 14.

  **Found: two buttons named "Save".** While an item was being edited the
  set's Add read "Save", so a form whose own submit button is `save` (the
  test forms', and a common name) had two buttons announced identically.
  **Fixed (Todd, 2026-11-03):** it reads "Replace", named "Replace “rust”"
  for the item it replaces. Named by the ITEM, not the draft's text, so the
  name stays true while the user types (Todd's point about "Save “rust”");
  the visible word begins the name, for WCAG 2.5.3 (Label in Name).
  `AddItemProps::editing` is now `Option<String>`, the item's text.

  From box 8, browser-only: focus lands in the draft after Edit, Save and
  Cancel. From box 7, browser-only: Enter in the draft adds and does NOT submit
  the form; focus is in the draft after Add, by Enter and by click; and
  the known gap, whether a draft breaking `pattern` (or a number's
  `min`/`max`) blocks a browser-validated submit. If it does, fixing it
  becomes a box of its own (Todd, 2026-11-03).

- [x] **11.** (Claude) Rows become items, everywhere a dev or a Formoxus
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

  **Built 2026-11-03** (Claude):
  1. `FixedKey`s: `ListRow`/`ListRows` → `ListItem`/`ListItems`,
     `AddRowButton` → `AddItemButton`, `RemoveRowButton` →
     `RemoveItemButton`, `MoveRowButtons`/`MoveRowUpButton`/
     `MoveRowDownButton` → `MoveItemButtons`/`MoveItemUpButton`/
     `MoveItemDownButton`. Their `classes_plus:` names follow
     (`list_item`, `add_item_button`, …). The component names
     `AddItemButton` etc. are unchanged; a `FixedKey` is always written
     qualified, so the two never meet.
  2. Classes: `fx-list-row(s)` → `fx-list-item(s)`, `fx-button-add-row` →
     `fx-button-add-item`, `fx-button-remove-row` → `fx-button-remove-item`.
     `fx-move-buttons`, `fx-button-move-up`/`-down` never said row.
  3. `field_kind::row` → `field_kind::item` (what `form!`'s `[]`
     projection calls, so it shows in expansions and errors), the
     generated loop binding `__row{n}` → `__item{n}`, `AddItemProps::rows`
     → `item_count`, `row_path` → `item_path_at`, `new_blank_row` →
     `new_blank_item`.
  4. Messages: "every ITEM a widget", "one item's key — a spec names every
     item at once", `form!`'s "a spec applies to every item", "its items are
     edited in place".
  5. Prose in every doc comment and test, and about 80 test names; the
     `/t/struct-rows` route is `/t/struct-items`, and
     `e2e/tests/new_row_focus.rs` is `new_item_focus.rs`.
  6. **Kept, as other meanings of "row":** the textarea `rows` attribute,
     the attribute table's and `BUILTINS`'/`WidgetName`'s rows,
     `field_row`, the button row, a test model's field named `rows`, and
     "in a row". The user-facing labels changed too (Todd, the same day:
     "so we're consistent"): a list's "Remove item N", "Move item N
     up/down".

  `just ci` 1051 and `just e2e` 52/52 green.

- [ ] **12.** (Claude) Merge prep, **LAST: after boxes 13 and 14** (Todd,
  2026-11-03). `ORDERED_LISTS_PLAN.md` marked as superseded where it is
  (boxes 2–3), its boxes 6–8 checked against this plan, the memory updated,
  and `just ci` and `just e2e` green.

  **apcsp waits until this branch is merged into `main`** (Todd, the same
  day), so it is not part of this box. apcsp builds formoxus by PATH, so it
  follows whatever branch is checked out here, and breaks while `sets` is
  checked out. What it will need: its three `.fx-list-set` selectors (2b)
  become `.fx-list`, and its comment naming `ListSet` becomes `ListNode`.
  From box 11 (checked 2026-11-03):
  `crates/web/assets/main.css` has `fx-button-add-row` and
  `fx-button-remove-row` 20 times each and `fx-list-row` 3 times, and
  `crates/e2e/tests/questions.rs` uses `.fx-list-set`,
  `button.fx-button-add-row` and `.fx-list-row`.

- [ ] **13.** (Claude) Gallery examples for a set and a list (Todd,
  2026-11-03: both together, before box 12), using the
  `fx-tags` preset for the set. Replaces box 10's gallery example, and is
  where the box 9 CSS is first seen in a browser. **Decide here:** the gallery CSS sets
  `list-style: none` on `.fx-list-items > .fx-list-item` (for its grid of
  ↑/↓ | item | Remove), so a list's numbers are hidden on screen, and
  "Remove item 2" names a number only a screen reader announces. Show the
  numbers (a `::marker` or a counter in the grid's first column), or accept
  it.

- [x] **14.** A draft's constraint attributes must not block a submit:
  box 10 confirmed that a draft breaking its items' `pattern` does, under
  browser validation. Probably the draft's input takes a `form` attribute
  naming no form, which also keeps it out of a native submit; a leaf widget
  has no way to be given one yet. Design first.

  **Decided and built 2026-11-03 (Todd's design):** leave the constraints
  OFF the draft's input and check them on Add/Replace, which already
  validates the draft. A `Draft(...)` newtype for `validate` was not
  needed: the draft's values are `RawValue::Draft`, which `validate` and
  `to_wire` never see, so only the browser was blocking. `NodeContext`
  gains `in_draft` (set by `SetNode` for its draft; dropped by `nested`
  and `optional`); `FieldNode::render` then keeps only `maxlength` of the
  field's own attributes (`FieldAttrs::keeping`), since typing cannot
  break it. An allow-list, so a constraint added later stays off the draft
  by default. Lost, accepted ("we can be more clever later"): the number
  spinner's limits and `:invalid` styling while typing. The `form`-
  attribute idea above is the cleverer option if that feedback is wanted.

  Tests: 3 in `sets.rs` (constraints off, `maxlength` kept, Add still
  refuses, a list's items keep theirs), mutation-checked; the e2e `_yet`
  test became `a_draft_breaking_its_pattern_does_not_block_the_submit`,
  plus `add_refuses_a_draft_breaking_its_pattern`. e2e 53/53.

## Not in this plan

- Sets of structs or of enums with data (Scope).
- A multiset, where order doesn't matter but duplicates are allowed. If
  apcsp ever needs one, it is a list for now.
