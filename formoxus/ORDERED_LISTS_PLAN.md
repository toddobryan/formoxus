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

- **A. Moving is opt-in per list:** `allow_reordering: true` on the list's
  entry in `form!` (`answer_choices => { allow_reordering: true }`).
  Off by default, so no existing form changes. The list's OWN path, not
  `answer_choices[]`: in `form!`, `x` is the list (where its `label:`
  goes) and `x[]` is every row. `rows[] => { allow_reordering: true }` is
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

- [ ] **2.** Carry `allow_reordering: true` from a list's entry in `form!`
  to `ListSet`. Split between Todd and Claude; steps a–g, each small.
  Today a list's entry reaches its node like this: `FieldBody::parse_entry`
  (`formoxus-macros/src/form/field.rs:476`) parses the key,
  `FieldSpec::builder_calls` (`formoxus-macros/src/form.rs:559`) emits a
  `FormSpec` builder call, the call fills the runtime `FieldSpec`
  (`formoxus/src/form/spec.rs:22`), and `list_node`
  (`formoxus/src/build.rs:607`) reads it with `walk.spec_for(prefix)`;
  that is all `label:` does for a list. Compile-time checks are `const _`
  asserts from `FieldBody::type_checks` (`field.rs:287`), asking
  `formoxus::field_kind`'s `const fn`s about the field's shape.

  - **2a.** `field_kind::is_list(shape)`, a `const fn`, `Option` peeled
    the way `kind` peels it (`field_kind.rs:234`), so an `Option<Vec<_>>`
    can be reordered too: `matches!(…def, Def::List(_))`.
  - **2b.** Runtime: `allow_reordering: bool` on `FieldSpec`, and a
    `FormSpec::with_allow_reordering(path)` builder beside `with_label`.
  - **2c.** `list_node` copies it onto a new `ListSet.allow_reordering`
    (`build.rs:652` is the only place a `ListSet` is built).
  - **2d.** `ListSet::render` draws `<ol>`/`<li>` instead of the row
    `div`s when it is set (F). Moved here from box 3 so this box shows up
    in the rendered HTML, and so is testable from the suite, before any
    button exists.
  - **2e.** Macro parse: a `FieldBody.allow_reordering`, an arm in
    `parse_entry` beside `widget`/`label`/`choices` (with "given twice"),
    `is_empty` counting it, and `legal_keys` (`field.rs:982`) listing it,
    so `allow_reorder:` gets "did you mean".
  - **2f.** Macro codegen: `builder_calls` emits
    `.with_allow_reordering(#key)`; `type_checks` emits
    `const _: () = assert!(is_list(#shape), "`allow_reordering` applies
    only to a list field")`, spanned on the key.
  - **2g.** Docs: wherever `form!`'s keys are listed (the `form!` doc in
    `formoxus-macros/src/lib.rs`, README).
  - Tests (Claude): parse tests beside `every_legal_key_parses`; a
    trybuild golden for `allow_reordering` on a `String`; suite tests
    that `<ol>` renders for a top-level list, `rows[]` of a
    `Vec<Vec<_>>`, a list inside an enum row, and an `Option<Vec<_>>`,
    and that a list without it still renders `div`s.

  Open, for Todd:
  - **2-i.** Value: `allow_reordering: true` only, or also the bare flag
    `allow_reordering`, as `required` is written? Claude leans both, with
    the value a `bool` literal: `false` is then legal and does nothing,
    which is harmless.
  - **2-ii.** A hand-built `FormSpec` can still set it on a non-list,
    where `form!` can't. Ignore it there, or assert, the way `list_node`
    and `struct_node` assert on a `widget`? Asserting means touching each
    non-list builder; ignoring is one line of doc.
  - **2-iii.** One `bool` on `FieldSpec` now, and `add_row_button` as a
    second field later, or a `list: Option<ListSpec>` grouping both?
    `FieldSpec` already mixes leaf settings (`widget`, `node`, `attrs`)
    with `label`, so a plain field fits what is there.

- [ ] **3.** (Claude, from apcsp's `OrderedPicker`) Move buttons: a
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
  `examples/assets/main.css`.
  - Tests (Claude): render snapshot; disabled ends; e2e: move a row and
    submit, the server sees the new order.

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
