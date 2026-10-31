# Ordered lists: moving rows, and adding a row with a value

Written 2026-10-29 by Claude, from apcsp-dioxus, for Todd to build. apcsp
wants its "new assignment" page to be a formoxus form, and formoxus lists
can't yet do two things that page needs: **move a row** and **add a row
that arrives already filled in** (search-to-add). Both are general list
features, not assignment-specific.

Who does what, as usual here: Todd writes the library code; Claude writes
the tests, the e2e forms and the gallery example.

## The form that needs it (apcsp-dioxus)

apcsp's model is `crates/models/src/content/assignment.rs`. The form type
would be roughly:

```rust
struct AssignmentForm {
    name: String,
    settings: SettingsForm,              // late_policy enum, Option<u8>, Option<u16>, f64
    question_groups: Vec<QuestionGroupForm>,
}

enum QuestionGroupForm {
    All { questions: Vec<String>, shuffle_questions: bool },
    Random { how_many: u8, set: Vec<String> },
    Streak { how_many: u8, set: Vec<String> },
}
```

Each `String` in `questions`/`set` is a question's record id. The page
has 155 questions per course, so a row isn't typed in: a search box adds a
question, and the row shows the question (name, source, a clamped preview)
rather than an input. Today apcsp does this outside formoxus with
`ui::OrderedPicker` + `ui::SearchToAdd` (`crates/ui/src/forms/ordered_picker.rs`),
whose `move_up`/`move_down`/`remove` are plain fns on a slice, unit-tested
(13 tests; a bad index is a no-op).

## Already works (checked 2026-10-29)

- `Vec<enum>` rows: `tests/suite/vecs.rs` `shapes: Vec<Shape>`,
  `enum_rows_are_pinned_by_the_value`.
- A list inside a list: `rows: Vec<Vec<String>>` in the same file. A list
  inside an enum row inside a list (the `questions` above) is the same
  walk, but nothing tests that exact nesting yet; box 7 adds one.
- Mixed unit + struct variants with a variant picker: apcsp's question
  form (`select`, `data`).
- Number fields: `u8`, `u16`, `f64` and their `Option`s
  (`fields/number_fields.rs`).
- Cross-field errors at a path: the `validator:` closure with
  `ValidationError::at(path!(…))`. apcsp wants "this question is already in
  group 1" on the duplicate's row.
- A custom widget sees the whole form: `WidgetProps.values` is the form's
  one `ValuesStore` (`nodes.rs:258`), so a search box can hide questions
  picked in ANY group by reading the other lists' paths.
- Runtime data for a custom widget: it renders as a component, so it can
  `use_context` for the course's questions, which the page provides.

## What's missing

Today `Edit` (`nodes.rs:369`) has `ChooseVariant`, `AddRow { path,
before }` and `RemoveRow { path, index }`. `ListSet::render`
(`nodes/list_set.rs:186–198`) draws each row plus a `RemoveRowButton`, then
one `AddRowButton`; `add_row` (`list_set.rs:43`) always builds a BLANK row.
Rows are keyed, so nothing about reordering fights the design: the doc
comment at `list_set.rs:4` already promises "inserted mid-list, removed,
or reordered without renaming".

### Decisions (Todd, 2026-10-29)

- **A. Moving is opt-in per list:** the bare flag `allow_reordering` on the list's
  entry in `form!` (`answer_choices => { allow_reordering }`).
  Off by default, so no existing form changes. The list's OWN path, not
  `answer_choices[]`: in `form!`, `x` is the list (where its `label:`
  goes) and `x[]` is every row. `rows[] => { allow_reordering }` is
  still right for the inner lists of a `Vec<Vec<_>>`.
- **B. A new row's value is a TYPED value, built like construction builds
  a row.** `Edit::AddRow` gains an `Option` of the row's value: `None` is
  today's blank row, `Some` builds the row from it. Typed rather than raw
  `(sub-path, String)` pairs so the adder never has to know how the row
  lays out its fields.

  Where the value lands matters, because the tree and the store are
  separate: `Node::edit` gets only `specs` and never sees the live
  `ValuesStore`, which widgets write through `widgets/values.rs`. So:
  1. `ListSet::add_row` builds the row with `Some(peek)` and
     `FormMode::Populated` (what `form_for` does) instead of `blank_row`'s
     `None` + `FormMode::Blank`.
  2. `FormState::edit` (`form/state.rs:242`) returns the new row's path.
  3. `use_form`'s `on_edit` (`form.rs:712`), which holds both `state` and
     `values`, copies that row's leaf values into the store.

  The plan's first draft had `add_row` write the store itself and said a
  return value couldn't work because "an `Edit` goes up a callback,
  nothing comes back." Both wrong: the callback is `use_form`'s, and it
  holds everything.
- **C. The adder is a plain component fn, set as `add_row_button:` on the
  list's entry** (`questions => { add_row_button: SearchToAdd }`),
  defaulting to today's `AddRowButton`. Not a `Widget` impl: it isn't a
  leaf, so `Widget`'s consts mostly don't apply. Props: `path`, `on_edit`,
  `values`, `classes`, the row count.

  **Icons, as in apcsp:** the default ↑/↓/✕/Add buttons draw icons, not
  words. Each still needs an accessible name (`aria-label`), since an icon
  button with no text has none: Remove already has "Remove row N".
- **D. Default layout: ↑/↓ on the row's left, ✕ on its right**, so Remove
  is never next to a move (apcsp's `.ordered-picker` in its `main.scss`).
  How a dev overrides that waits until someone needs to.
- **E. A label is needed only where there is an input.** A display-only
  row (hidden input plus rendered text) needs no `<label>`; the LIST is
  what the user is answering, and its `<legend>` labels it. This is an
  exception to `FIELDS_PLAN.md` V4, not a reversal: a tuple field the user
  types into still needs one.

- **F. A reorderable list is an `<ol>` of `<li>` rows** (when
  `allow_reordering` is set; every other list keeps its `div` rows), so a
  screen reader says "item 2 of 5" exactly where position means something.
- **G. Box 3 is Claude's**, copied from apcsp's `OrderedPicker`, plus the
  new class keys. The macro work (box 2) comes first.
- **H. Add stays at the BOTTOM, appending** (Todd, 2026-10-30, reversed
  the same day). First decided as "Add at the top of every list", so a
  long list couldn't push it out of reach; reversed because a top button
  that appends drops the new row off-screen, and per-row "insert before"
  buttons (considered next) cost a button per row, still don't reach
  the end, and don't fit a one-per-list custom adder (box 5). Reach is
  solved in CSS instead: `position: sticky; bottom: 0` on
  `.fx-button-add-row` (box 3, `examples/assets/main.css`). Per-row
  insert is box 3b, parked.

### Boxes

- [x] **1.** (Todd) `Edit::MoveRow { path, from, to }` (or `index` +
  `Up`/`Down`; Todd's call), applied in `ListSet` next to `RemoveRow`
  (`list_set.rs:323`) by moving the boxed row in `self.rows`. A bad index
  is a `FormAccessError`, like `add_row`'s and `remove_row`'s, not a no-op
  like apcsp's `move_up`: the buttons are disabled at the ends, so a bad
  index is a bug, and `on_edit` logging it beats hiding it. The row keeps
  its key, so its values stay with it, and nothing changes on the wire:
  `collect_values` already writes row keys in `rows` order, and
  `set_rows` rebuilds in that order.
  - Tests (Claude): move first↔last, a middle row both ways, a bad index,
    values and errors following the row, the row-key order on the wire, a
    move inside a nested list (`rows[1][…]`).

- [x] **2.** Carry the `allow_reordering` flag from a list's entry in `form!`
  to `ListSet`. Steps a–g, each small: **Todd takes 2a–2d (runtime),
  Claude 2e–2g (macro) and all of box 2's tests** (decided 2026-10-30).
  Today a list's entry reaches its node like this: `FieldBody::parse_entry`
  (`formoxus-macros/src/form/field.rs:476`) parses the key,
  `FieldSpec::builder_calls` (`formoxus-macros/src/form.rs:559`) emits a
  `FormSpec` builder call, the call fills the runtime `FieldSpec`
  (`formoxus/src/form/spec.rs:22`), and `list_node`
  (`formoxus/src/build.rs:607`) reads it with `walk.spec_for(prefix)`;
  that is all `label:` does for a list. Compile-time checks are `const _`
  asserts from `FieldBody::type_checks` (`field.rs:287`), asking
  `formoxus::field_kind`'s `const fn`s about the field's shape.

  - [x] **2a.** (Todd) `field_kind::is_list(shape)`, a `const fn`:
    a new `Kind::List`, returned by `kind` for `Def::List` right after it
    peels an `Option`, so an `Option<Vec<_>>` can be reordered too.
    `is_single_value` (the one `matches!` the compiler wouldn't flag),
    `applies`, `renders` and `field_type` treat it like `Other`.

  - [x] **2b.** (Todd) Runtime: `allow_reordering: bool` on `FieldSpec`,
    and a `FormSpec::with_allow_reordering(path)` builder beside
    `with_label`, asserting `is_list` per 2-ii below. `spec_key_error` is
    now `spec_key_shape`, returning the shape, and `FormSpec::shape_at`
    is the one place a bad path panics.

  - [x] **2c.** (Todd) `list_node` copies it onto a new
    `ListSet.allow_reordering` (`build.rs:652` is the only place a
    `ListSet` is built).

  - [x] **2d.** (Todd) `ListSet::render` draws `<ol>`/`<li>` instead of
    the row `div`s when it is set (F). Moved here from box 3 so this box
    shows up in the rendered HTML, and so is testable from the suite,
    before any button exists. Adds `FixedKey::ListRows` (`fx-list-rows`)
    for the `<ol>`, and renames `ListRow`'s class `fx-form-row` →
    `fx-list-row` to match. `AddRowButton` stays below the rows (H).

  - [x] **2e.** (Claude, 2026-10-30, uncommitted) Macro parse:
    `FieldBody.allow_reordering: Option<Ident>` (the key, for the
    assert's span), an arm in `parse_entry` with "given twice" and the
    flag's "write it alone" error (now one `flag_with_value` fn shared
    with the table's flags), `is_empty` counting it, `legal_keys` listing
    it. A near miss within 2 edits gets "did you mean" (`allow_reodering`);
    `allow_reorder` is 3 away and gets the general unknown-key message.

  - [x] **2f.** (Claude, same) Macro codegen: `builder_calls` emits
    `.with_allow_reordering(#key)`; `type_checks` emits
    `const _: () = assert!(field_kind::is_list(#shape), "`allow_reordering`
    applies only to a list field")`, spanned on the key. Both name items
    2a/2b create, so a `form!` using the flag won't compile until 2b lands.

  - [x] **2g.** (Claude, same) Docs. The only list of field-body keys is
    the TODO above `form!` in `formoxus-macros/src/lib.rs`; added there.
    The README lists no keys.

  - [x] **2t1.** (Claude) Parse tests: six, in `field.rs`'s
    "allow_reordering" section, plus an arm in `every_legal_key_parses`.

  - [x] **2t2.** (Claude) Goldens `form_allow_reordering_on_a_string`
    and `form_allow_reordering_on_a_lists_rows` (`answers[]`, decision
    A's mistake); five `specs.rs` tests for the hand-built spec (every
    kind of list accepted; a scalar, a list's rows, a struct row and a
    bad path refused, the bad path first).

  - [x] **2t3.** (Claude) Six render tests in `vecs.rs`'s "Reorderable
    lists" section, each through `form!`: a top-level list is an `<ol>`
    of `<li>`s with Add after the `</ol>` and each Remove inside its
    `<li>`; an unflagged list keeps `div`s; `rows[]` reaches each inner
    list and not the outer one; `groups[].$Group::All.questions` (the
    apcsp shape); `answers?` on an `Option<Vec<_>>`. The two snapshots
    re-blessed for the rename, diff read: one line each.

  Decided (Todd, 2026-10-30):
  - **2-i. A bare flag only:** `allow_reordering`, never
    `allow_reordering: true`, which gets the "is a flag: write it alone"
    error every other flag (`required_true`, `autofocus`, …) gives. First
    decided as "both", from Claude's wrong claim that `required` is
    written that way; reversed once `parse_value` showed no flag takes a
    value.
  - **2-ii. Assert** when a hand-built `FormSpec` sets it on a non-list,
    which `form!` already refuses at compile time. Where (Claude's
    proposal, OK'd by Todd): in `with_allow_reordering` itself,
    not in each non-list builder. `FormSpec::field` already walks the
    shape through `spec_key_error` (`build.rs:95`) and panics on a bad
    path; that walk ends `At::Shape(shape)`, the shape the key names, so
    `spec_key_error` becomes a `Result<&'static Shape, String>` returning
    it, and `with_allow_reordering` asserts 2a's `is_list` on it. One
    check, shared with the macro's `const _` assert, firing when the
    spec is built rather than when the form is.
  - **2-iii. A plain field:** `allow_reordering: bool` on `FieldSpec`;
    `add_row_button` becomes a second field in box 5.

- [x] **3.** (Claude, from apcsp's `OrderedPicker`) Move buttons: a
  `MoveRowButtons` widget (↑/↓, `type="button"`, disabled at the ends,
  an accessible name like "Move row 2 up") rendered by `ListSet` on the
  row's left when `allow_reordering` is set, with ✕ on the row's right
  (D), with the rows already `<ol>`/`<li>` from 2d. ↑ sends
  `MoveRow { from: i, to: i - 1 }`, ↓ `to: i + 1`; apcsp's `move_up`/`move_down` and their tests
  don't come over, since `ListSet::move_row` does that job. Icons per C,
  inlined as SVG from Lucide (ISC; credit it in the file) rather than
  the `dioxus-free-icons` crate, which costs 13.8 s and 65 MB per target
  for four one-`path` icons. Add and Remove switch to icons too. New class keys in
  `formoxus-spec/src/fixed_keys.rs` and both preset tables in
  `classes.rs` (`fx-move-buttons`, `fx-button-move-up`,
  `fx-button-move-down`), under the `fx-` vocabulary
  (`css_class_vocabulary` memory). apcsp's grid CSS goes into
  `examples/assets/main.css`, with `position: sticky; bottom: 0` on
  `.fx-button-add-row` so Add stays in reach on a long list (H).
  - Tests (Claude): render snapshot; disabled ends; e2e: move a row and
    submit, the server sees the new order.
  - Then drop "A user cannot reorder a list's rows yet" from "What isn't
    done" in the root `README.md` (`formoxus/README.md` is a symlink to it;
    edit through the root, never `sed -i` the link, which replaces it).
  - Decided (Todd, 2026-10-30): Add is icon PLUS the word "Add" (once per
    list, so the text costs nothing; ↑/↓/✕ repeat per row, so they are
    icon-only with an `aria-label`). The `<svg>`s get no class: CSS
    reaches them as `.fx-button svg`.
  - DONE 2026-10-30 (Claude), uncommitted. `MoveRowButtons` in
    `widgets/row_buttons.rs` (exported beside Add/Remove); icons in
    `widgets/icons.rs` (`pub(crate)`, an `icon(&[path data])` fn);
    `FixedKey::{MoveRowButtons, MoveRowUpButton, MoveRowDownButton}`.
    Remove is icon-only on EVERY list now, Add icon + "Add"; three
    snapshots re-blessed for it. Suite: five markup tests in `vecs.rs`'s
    "The move buttons". e2e: `/t/reorder` + `e2e/tests/reorder.rs` (2),
    mutation-checked (a ↑ that moves nothing fails it); full e2e 37/37.
    README's "cannot reorder yet" line dropped. `aria_hidden:` does not
    exist on dioxus's `svg`; it is written `"aria-hidden": "true"`.

- [ ] **3a.** (unassigned, lower priority since H was reversed) Focus a
  new row: move focus into the first focusable element of a row that
  `AddRow` just made, via `onmounted`, on every list; not on rows present
  at first render, or the page would open focused on the last row. With
  Add below the rows the new row appears beside the button, so this is
  for screen readers (focus says where the row went) more than for
  sight. Lettered so boxes 4–8 keep their numbers.
  - Tests: e2e only (focus is a browser fact).

- [ ] **3b.** (PARKED) Per-row "insert before" (`AddRow { before:
  Some(i) }`, which the edit already supports; `row_buttons.rs:28`
  parks the UI question). If ever: opt-in, only on a list with
  `allow_reordering`, since position barely matters elsewhere; an
  accessible name that says "before" ("Insert a row before row 3");
  and not until box 5 decides whether a custom adder can be told a
  position, since a search adder is one per list.

- [ ] **4.** (Todd) Add-with-value per B, steps 1–3. `Edit` derives
  `Clone, Debug, PartialEq` and isn't generic, so the typed value needs
  erasing; the open part is how. One shape for it: a `RowValue` holding
  an `Rc<dyn Any>`, the `&'static Shape`, and a monomorphized
  `fn(&dyn Any) -> Peek<'_, 'static>` made in `RowValue::new::<T:
  Facet<'static>>`. `Rc` keeps it `Clone`; `Debug` and `PartialEq` are
  hand-written (`Rc::ptr_eq`?). The stored shape lets `add_row` refuse a
  value whose type isn't the list's row type, which nothing checks at
  compile time once `add_row_button:` is a plain fn. `AddRowButton` sends
  `None`.
  - Tests (Claude): a leaf row and a struct row added with values; the
    store and `collect_values` see them; inserting with `before` and a
    value; a value of the wrong type refused.

- [ ] **5.** (Todd) `add_row_button:` per C, replacing `AddRowButton`
  when set.
  - Tests (Claude): a custom adder in the suite (a fixed list of options;
    picking one adds a filled row); `form!` rejects `add_row_button:` and
    `allow_reordering:` on a non-list path at compile time.

- [ ] **6.** (Todd, small, maybe nothing) Confirm a custom widget on a
  `String` leaf works as a display-only row: hidden input with the id,
  plus whatever the widget draws, and no `<label>` (E).
  - Test (Claude): the row renders its display, has no `<label>`, and
    submits its value.

- [ ] **7.** (Claude) The apcsp shape end to end in the suite:
  `Vec<enum { All { questions: Vec<String>, … }, … }>` with moves, the
  adder, and a validator error on one nested row ("already in group 1").
  Then an e2e form and a gallery example.

- [ ] **8.** (Todd, in apcsp-dioxus) Pick it up there: the assignment form,
  `SearchToAdd` as the adder, the question row as the row widget, and
  `ordered_picker.rs`'s `OrderedPicker` retired if nothing else uses it.
  apcsp's `ANSWERING_PLAN.md` 2e–2j get rewritten around the form then.

## Not in this plan

- Drag-and-drop reordering. Buttons first; they're keyboard-accessible
  for free.
- Min/max row counts: that's issue #14, and it composes with this (an
  adder hides itself at the max).
- An "insert here" button between rows: `AddRow.before` already allows it;
  the UI question in `row_buttons.rs`'s comment stays open.
