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
  list's entry** (`questions => { add_row_button: SearchToAdd }`; renamed
  `adder:` on 2026-10-31, see box 5),
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
  - DONE 2026-10-30 (Claude), ebe4437. `MoveRowButtons` in
    `widgets/row_buttons.rs` (exported beside Add/Remove); icons in
    `widgets/icons.rs` (`pub(crate)`, an `icon(&[path data])` fn);
    `FixedKey::{MoveRowButtons, MoveRowUpButton, MoveRowDownButton}`.
    Remove is icon-only on EVERY list now, Add icon + "Add"; three
    snapshots re-blessed for it. Suite: five markup tests in `vecs.rs`'s
    "The move buttons". e2e: `/t/reorder` + `e2e/tests/reorder.rs` (2),
    mutation-checked (a ↑ that moves nothing fails it); full e2e 37/37.
    README's "cannot reorder yet" line dropped. `aria_hidden:` does not
    exist on dioxus's `svg`; it is written `"aria-hidden": "true"`.

- [x] **3a.** (Todd's code, Claude's tests; DONE 2026-10-31) Focus a
  new row: move focus into the first focusable element of a row that
  `AddRow` just made, via `onmounted`, on every list; not on rows present
  at first render, or the page would open focused on the last row. With
  Add below the rows the new row appears beside the button, so this is
  for screen readers (focus says where the row went) more than for
  sight. Lettered so boxes 4–8 keep their numbers.
  - Tests: e2e only (focus is a browser fact).
  - Decided (Todd, 2026-10-30): AFTER box 4, reusing its returned
    row path (`FormState::edit`) and its blank-vs-filled split. Focus
    ONLY a row that arrives blank (`AddRow` with no value): a filled
    row from a custom adder leaves focus where the adder put it, so a
    search-to-add user keeps searching. Shape: `on_edit` sets a one-shot
    signal to the new row's path; a `use_effect` in `use_form` focuses
    the row's first input with `document::eval`, scoped to the form, on
    `[name="p"], [name^="p."]` (a bare prefix would match `#20` for
    `#2`). Built without clearing the signal: keys never repeat.
  - Decided (Todd, 2026-10-31): `Form::edit` does NOT focus; only the
    buttons' `on_edit` does. Code that adds a row can focus what it likes.
  - Tests: `e2e/tests/new_row_focus.rs` (leaf row on `/t/reorder`, struct
    row on the new `/t/struct-rows`), mutation-checked. playwright-rs
    0.15.1's `to_be_focused` is broken for non-CSS locators, so they use
    `e2e::focus::expect_focused`.

- [ ] **3b.** (PARKED) Per-row "insert before" (`AddRow { before:
  Some(i) }`, which the edit already supports; `row_buttons.rs:28`
  parks the UI question). If ever: opt-in, only on a list with
  `allow_reordering`, since position barely matters elsewhere; an
  accessible name that says "before" ("Insert a row before row 3");
  and not until box 5 decides whether a custom adder can be told a
  position, since a search adder is one per list.

- [ ] **4.** (Todd) Add a row that arrives filled in, per B: a TYPED
  value on `AddRow`, erased so `Edit` stays non-generic and keeps its
  `Clone, Debug, PartialEq`. Today `Edit::AddRow { path, before }`
  (`nodes.rs:373`), `ListSet::add_row` (`list_set.rs:47`) always builds
  through `blank_row` (`:73`, `None` peek + `FormMode::Blank`), and
  `use_form`'s `on_edit` (`form.rs:712`) only logs an error. Design
  (Todd, 2026-10-30: "follow your design"):

  - [x] **4a.** `RowValue` in `nodes.rs`, beside `Edit`:
    `value: Rc<dyn Any>`, `shape: &'static Shape`, and
    `peek: for<'a> fn(&'a dyn Any) -> Peek<'a, 'static>`, the last made
    in `RowValue::new::<T: Facet<'static> + 'static>(value: T)` as a
    non-capturing closure (`|any| Peek::new(any.downcast_ref::<T>()…)`),
    so it is monomorphized per `T` and needs no `T` on the struct.
    `pub fn shape()`, `pub(crate) fn peek()`. `Clone` derives (`Rc`, a
    fn pointer and a `&'static` are all `Clone`). `Debug` and
    `PartialEq` by hand, both through `Peek`, which has them: `Debug`
    prints `RowValue(<the value's Debug>)`; `PartialEq` is
    `self.shape == other.shape && self.peek() == other.peek()`, so two
    `RowValue`s made separately from equal values ARE equal (better
    than `Rc::ptr_eq`, which a test building the expected edit would
    trip on). `Shape`'s `PartialEq` compares type ids.

  - [x] **4b.** `Edit::AddRow` gains `value: Option<RowValue>`, `None`
    being today's blank row. Every construction site breaks, which is
    the to-do list: `AddRowButton` (`row_buttons.rs:31`) sends `None`;
    the tests' sites are Claude's (`value: None`).

  - [x] **4c.** `blank_row` becomes `new_row(…, value: Option<&RowValue>)`
    (one function, not a twin): `Some` builds with `Some(value.peek())`
    and `FormMode::Populated`, as `form_for` would build that row;
    `None` is today's `None` + `Blank`. `set_rows` passes `None`.
    `add_row` takes the value and first refuses one whose `shape()` is
    not `self.shape` (the row type, so `Vec<Option<T>>` wants a
    `RowValue` of `Option<T>`): a `FormAccessError` naming both types,
    and no row added. A `Populated` subtree inside a `Blank` form is
    not the mixing `FormMode`'s doc warns about: the mode is fixed at
    the subtree's root, from whether THIS call has a value, never from
    a peek further down.

    Done (Todd, Claude the `optional` part, 2026-10-30). Found on the way:
    `add_row`/`set_rows` built rows with the LIST's `optional`, where
    `list_node` builds them with `false`; an `Option<Vec<bool>>`'s added
    row was a tri-state select beside a checkbox. `new_row` now always
    passes `false` and the parameter is gone from both. Tests in
    `optional_containers.rs` ("Rows added to an Option<Vec<T>>"). The
    same slip in `VariantSet::set_variant` is fixed too (Claude, at
    Todd's word): it builds a chosen variant's fields with `false`, and
    `VariantSet.optional`, read nowhere else, is deleted along with
    `enum_node`'s parameter. Test: "A variant chosen inside an
    Option<enum>". Blank rows were never dropped (an empty row already
    failed `validate`); an unwanted row is removed with its button.

  - [x] **4d.** `Node::edit` returns `Result<Option<String>,
    FormAccessError>`: the full path of a row it CREATED, so the caller
    can find it. `ListSet` returns `Some(qualify(my_path,
    &row_segment(key)))` from `AddRow` (filled or blank: 3a wants
    both); every other edit and node returns `Ok(None)`; containers
    already `return m.edit(…)`, so they only change signature.
    `FormState::edit` returns it unchanged; its `choose_variant`
    wrapper (`state.rs:282`) maps it away.

  - [x] **4e.** `use_form`'s `on_edit`: on `Ok(Some(row))` from an
    `AddRow` whose `value` is `Some`, copy the new row's leaves from
    `state.peek().leaf_values()`, those whose path `owns(row, …)`, into
    the store with `write_value` (overwriting, so nothing stale can
    shadow them). A blank row copies nothing: absent is `""` already.
    Without this the row would render filled (the tree has the values)
    but submit empty, since the store is what `to_wire` and `validate`
    read.

  - [x] **4e+.** (Todd's call, Claude, 2026-10-30) `Form::edit(&self,
    &Edit) -> Result<Option<String>, FormAccessError>`, a public way to
    apply an edit from code, because nothing outside formoxus could reach
    `on_edit` and so 4e was untestable. `on_edit` and `Form::edit` share
    `apply_edit` (`form.rs`): the button path logs a failure, `edit`
    returns it. 4t2/4t5 go through it: probes add a filled row with
    `Form::edit` in a `use_hook` and render `Submission::accept` of
    `to_wire()`, which reads the store; mutation-checked (dropping the
    store copy fails all four).

  - [x] **4f.** (Claude) Docs: `AddRow::value` (what `None` and `Some` build, the
    type check), `RowValue` (why erased, why the shape is kept: nothing
    checks the type at compile time once the adder is a plain fn, box
    5), and `Node::edit`'s new return.

  - [x] **4t1.** (Claude) `RowValue`: equal values from separate `new`s
    are equal, different values are not, the same `Debug` text from
    two types is not equal; `Debug` shows the value.
  - [x] **4t2.** (Claude) Filled rows through `Harness` (the real
    `on_edit`): a leaf row (`Vec<String>`), a struct row
    (`Vec<Location>`), an enum row (`Vec<Shape>`, its variant chosen by
    the value), and `Vec<Option<String>>` with `Some` and `None`; each
    renders its values AND `validate` returns them, which is what
    proves 4e.
  - [x] **4t3.** (Claude) `before: Some(i)` with a value inserts the
    filled row there; the rows around it keep their keys and values.
  - [x] **4t4.** (Claude) A value of the wrong type is refused, by
    message, and the list is unchanged.
  - [x] **4t5.** (Claude) Across the wire: a filled row reaches
    `Submission::accept` (`across_the_wire`) with its values.
  - [x] **4t6.** (Claude) `FormState::edit` returns the new row's path
    for a blank and a filled `AddRow` (including a nested list's, like
    `rows.#1.#2`), and `None` for `RemoveRow`, `MoveRow` and
    `ChooseVariant`.

- [x] **5.** A custom adder, per C, as `adder:` (renamed from
  `add_row_button:`; apcsp's adder is a search box, not a button).
  Decided (Todd, 2026-10-31): `AddRowButton` BECOMES the default adder,
  with the same props (one function, not two); the adder types, `Edit`
  and `RowValue` go in the `formoxus::widgets::*` glob, not the prelude;
  the props gain `id`, the list's own `{form}-{path}`, so an adder can
  build ids (a search box's `<label for>`) that cannot collide. Split as
  box 2: Todd the runtime (5a–5e), Claude the macro, exports and tests.

  Checked first: a component in a fn pointer renders through
  `DynamicNode::Component(VComponent::new(f, props, "RowAdder"))`, and
  Dioxus identifies a pointer-sized component by the pointer, so two
  adders are two components. But a bare fn pointer cannot sit in
  `FieldSpec`: `derive(PartialEq)` over one trips
  `unpredictable_function_pointer_comparisons` (warn by default, so an
  error under `-D warnings`), hence the newtype in 5b, like `NodeCtor`.

  - [x] **5a.** (Todd) `AddRowProps` in `widgets/row_buttons.rs`,
    `#[derive(Props, Clone, PartialEq)]`: `path`, `id`, `on_edit`,
    `values`, `classes`, `rows: usize`. `AddRowButton` takes it.

  - [x] **5b.** (Todd) `pub struct RowAdder(pub fn(AddRowProps) -> Element)`:
    `Debug` by hand, `PartialEq` by `std::ptr::fn_addr_eq`, `Default`
    is `AddRowButton`. The field must be `pub`: `form!` writes
    `::formoxus::widgets::RowAdder(SomePath)` at the dev's call site.

  - [x] **5c.** (Todd) `FieldSpec.adder: Option<RowAdder>` and
    `FormSpec::with_adder(path, RowAdder)`, which panics on a non-list
    path as `with_allow_reordering` does.

  - [x] **5d.** (Todd) `build.rs`'s `list_node` passes it into a new
    `ListSet.adder: RowAdder`, defaulted when the spec has none.

  - [x] **5e.** (Todd) `ListSet::render` renders `self.adder` through
    `VComponent` where it hard-codes `AddRowButton` today.

  - [x] **5f.** (Claude) `form!`: `adder: SomePath` on a list's own path
    or `x[]`, generating `.with_adder("path", RowAdder(SomePath))`,
    spanned on the component's path so wrong props are reported there;
    the list check is now `FieldBody::list_checks`, shared with
    `allow_reordering` (its goldens unchanged). Six parse tests in
    `field.rs` (part of 5t2); `adder` is in `legal_keys`, so a near miss
    is suggested. Bare `adder` with no component is syn's "expected `:`".

  - [x] **5g.** (Claude) Exports: `AddRowProps`, `RowAdder`, `Edit`,
    `RowValue` in `formoxus::widgets::*`.

  - [x] **5t1.** (Claude) Suite: with no adder every snapshot is
    unchanged; a custom adder renders instead of Add, with the right
    `path`, `id` and `rows`; a test adder sends a filled `AddRow`
    through its `on_edit` and the row arrives filled and reaches
    `to_wire`; `with_adder` on a non-list panics.

  - [x] **5t2.** (Claude) `form!`: parse tests, and a golden for
    `adder:` on a `String`.

  - [x] **5t3.** (Claude) e2e pair: `/t/adder`, a fixed set of choices;
    a click adds a filled row, and the submit shows its value.

  - [x] **5h.** (Claude, found by 5t1) `RemoveRow` clears the row's
    store entries. Before, they outlived it: harmless to `to_wire`, which
    reads only live rows, but an adder reading `values` whole to leave
    out choices already in the list kept leaving out a removed one.
    Cleared by the row's path (`apply_edit` finds its key first), not by
    its current leaves, so an unchosen variant's values go too.

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
  A filled row added off-screen is invisible (3a only focuses blank
  rows), so apcsp flashes "Question was added" once it has messaging
  (Todd, 2026-10-30). Make it an `aria-live="polite"` region, so a
  screen reader hears it too.

## Not in this plan

- Drag-and-drop reordering. Buttons first; they're keyboard-accessible
  for free.
- Min/max row counts: that's issue #14, and it composes with this (an
  adder hides itself at the max).
- An "insert here" button between rows: `AddRow.before` already allows it;
  the UI question in `row_buttons.rs`'s comment stays open.
