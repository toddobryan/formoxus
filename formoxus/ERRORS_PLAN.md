# `ErrorsStore` and live validation

Written 2026-11-04 by Claude, from a conversation with Todd. Replaces
`FIELDS_PLAN.md` Step 7, and changes the design in
`.claude/memory/error_model_design.md` ("ErrorsStore"): that had errors staying
on the nodes with the store as a projection of them. Here they leave the nodes.

Who does what: Todd writes the runtime; Claude writes the macro side, the tests,
the e2e forms, the docs and the gallery example.

**On the branch `errors`, as one breaking change:** `WidgetProps.errors` goes, so
every custom widget changes.

## Why

Two things, wanted together before publish.

1. **Errors should change one field at a time.** `FieldNode::render` runs in
   `Form::render`'s scope, which reads `state_signal`, so one pushed error
   re-renders every field. Harmless once per submit; with live validation it
   would happen on every keystroke of a field showing an error.
2. **A user should learn about a mistake before submitting.** Today validation
   runs only on submit.

## Decisions (Todd, 2026-11-04)

A. **When a field is checked: reward early, punish late.** The first check is on
   blur, so nobody is scolded mid-word. A field showing an error re-checks on
   every input, so the error goes the moment it is fixed.

B. **What is checked live: a field's own checks only.** Parsing, `required`,
   constraints, and its `validator:`. `min_items`, a struct's or enum's
   validator, and the form's wait for submit: they can flag what the user has
   not finished yet.

C. **Errors leave the nodes.** They live in one flat map, not in a store and a
   tree that can drift apart. A store cannot be their only home, since
   `Submission::accept` runs `validate` on the server with no Dioxus runtime, so
   this copies how values work: a plain map, and a `Store` on the client.

   Kept as a projection, it fails three ways, which is why it went: `has_errors`
   says clean while a field shows an error, `to_wire` omits live errors, and a
   structural edit either wipes them or leaves them behind.

D. **Form-level errors stay on `FormState`.** They have no path and render once
   at form level, where a whole-form re-render costs nothing.

## The design

```text
              server                              client
FormState.field_errors: ErrorsByPath     ErrorsStore (in use_form)
  plain data; validate fills it,           Store<HashMap<String, Vec<…>>>
  collect_errors reads it                  seeded from the state, then the one
                                           home; widgets read it by path
```

**One home at rest.** On the client `FormState.field_errors` is a hand-off, not a
second copy: it is empty except during an operation. `Form` has one private
method that every operation goes through:

1. Take `state.write()`, and copy the store's entries into `field_errors`, empty
   ones too, since an empty one means checked and clean (#7).
2. Run the operation on the plain `FormState`: `validate`, `edit`,
   `push_field_error`, `distribute_errors`.
3. Drain `field_errors` back into the store, writing only the paths that
   changed, so only their widgets wake.

So every operation has ONE implementation, plain and runtime-free, the same on
server and client, and the store has one writer. A live check is the exception:
it never writes the state (see box 5).

**`ErrorsStore` is created by `use_form`**, beside the value store, so it lives in
the component calling `use_form`, but a dev never calls `use_store` for it. It is
`Copy` and rides on `NodeContext` and `WidgetProps`, as `values` does.

## Traps, checked against the source

1. **Clear an error by writing an empty `Vec`, never by removing the key.**
   dioxus-stores 0.7.10's `HashMap` `remove` and `retain` mark only the map
   shallowly dirty (`impls/hashmap.rs:167`, `:217`), so a widget reading that key
   through `get_unchecked` does NOT wake. `insert` does mark the key, which is
   why an absent key can be read and still wake on its first error. This also
   gives #7 what it needs: no entry is never checked, an empty one is checked and
   clean.
2. **A `Store` cannot hold an `IndexMap`,** so the store's order is lost, and
   `collect_errors` promises render order. `Form::to_wire` orders the store's
   entries by the walker in 2c.
3. **`Widget::render` is a plain method,** in the form's scope. The read has to
   happen in a component to be per-field: the built-ins have one (`TextareaView`,
   `SelectView`, `RadioGroupView`, the input's), and so does `FieldErrors`. A
   custom widget reading in a plain `render` still works, but subscribes the form.
4. **Reading `aria-invalid` in `FieldNode::render` defeats all of this.**
   `FieldAttrs::with_owned_attrs` bakes `invalid` into `aria-invalid` and
   `aria-describedby` there, and `field_row` passes `is_invalid()` to the label.
   All of them have to move into the component.
5. **A leaf cannot be judged alone.** `OptionNode` decides whether a blank inner
   field is required from whether the option is present, which for an
   `Option<struct>` depends on its sibling leaves, and the nodes' values are stale
   until the next `distribute_values`. So a live check clones the top-level node
   that owns the path, not the leaf (box 5).
6. **`FormState::has_errors` reads false on the client** between operations. A dev
   wants `Form::has_errors`, new here. Neither `examples/` nor `e2e/` calls the
   state's today.

## Decided once the plan was written (Todd, 2026-11-04)

E. **A blur checks only a field typed into since its last check.** A refinement
   of A, for two cases: tabbing through an empty required field should not scold,
   and a server's "that username is taken" should not vanish when the user clicks
   in and out without changing anything. A `<select>`, radio or checkbox has no
   typing, so its `onchange` counts as the blur.

F. **A knob, `live_validation:`,** in `form!` and `Formoxus`, cascading like
   `use_browser_validation`, default on. GOV.UK argues against inline validation,
   so a dev will want to turn it off; without the knob, they cannot.

G. **The handlers go on each control, not on a wrapper.** Each control already
   has the handler that writes its value, so the check follows the write in the
   same closure, and nothing depends on event order or bubbling. It is also small:
   `oninput` or `onchange` on five controls (text-like, textarea, select, checkbox,
   radio), and a new `onblur` on only the first two, since E has the others check
   on `onchange`. A hidden input has neither, and `VariantSelect` only makes an
   edit.

   Considered and dropped: one pair on `field_row`'s `<div>`, which would have
   reached a custom widget for free. But a radio group does not use `field_row`,
   and a custom widget that does not call the helper loses only the live check:
   it still validates on submit.

## Boxes

- [x] **1.** Branch `errors`, and this plan.

- [ ] **2.** (Todd) Errors off the nodes. Plain data only, no store yet.

  - [x] **2a.** `FormState.field_errors: ErrorsByPath`, beside `errors`. Delete the
    `errors` fields of `FieldNode`, `StructNode`, `VariantNode` and `Items`.

  - [ ] **2b.** `Node::validate(&self, prefix, out: &mut ErrorsByPath) -> bool`, the
    prefix threading as `collect_errors`' does today. `clear_errors`,
    `collect_errors` and `distribute_errors` go from `Node`;
    `validate` writes an entry for every path it checks, an empty `Vec` when
    clean, so the map already tells checked from never checked (#7).
    `FormState::collect_errors` becomes a read of the map that leaves the empty ones
    out, so `FormErrors.fields` is still empty for a form that passed, and
    `has_errors` asks whether any entry is non-empty. A `FieldValue::Invalid`
    stays on its node: it is a value, not a message.

  - [ ] **2c.** A walker, `Node::paths(&self, prefix, out)`: every path that can
    hold an error, containers included, in render order. It replaces what
    `distribute_errors`' unclaimed map did, for `push_field_error`, for a form
    validator's error at no path (its "At path: …" message) and for
    `Form::distribute_errors`' `Err`. `to_wire` orders by it (trap 2).

  - [ ] **2d.** The gates. A struct's, enum's or list's validator runs only over
    clean children (`struct_node.rs:74`, `list_node.rs:164`, `set_node.rs:710`,
    `variant_node.rs:318`), and `has_errors_within`/`has_errors_here` read a node's
    own field. Decided: `validate` returns whether it was clean, and takes `&self`,
    since it only ever wrote errors. Its loops must run every child, so
    `is_valid = child.validate(…) && is_valid`, in that order. A probe
    (`set_node.rs:206`, `:576`) validates into a scratch map.

  - [ ] **2e.** `Node::edit` gains `errors: &mut ErrorsByPath`, for what an edit
    records: `VariantNode`'s unknown-variant message (`variant_node.rs:156`), and
    a set draft refused on Add. Then `FormState::edit` drops every error whose path
    the tree no longer has, using 2c: one rule for a removed item, a variant
    changed, and an option emptied, in place of a `retain` per edit.

    **OPEN, for Todd at `SetNode::paths` and 2e: a draft's errors.** Claude to remind
    him there. A refusal at a draft's path is dropped by this rule if `paths` skips
    drafts, and reaches `to_wire`, `collect_errors` and `has_errors` if it does not.
    Either way a stale refusal outlives a successful Add, since the item keeps the
    draft's key, and so does one on an emptied draft or an Edit rebuild. Claude's
    proposal: one parameter on `paths` saying whether to include drafts (this rule
    includes them; `to_wire`, unclaimed errors and `collect_errors` do not), and
    `commit_draft` and the Edit rebuild first remove what the draft's path owns.

- [ ] **3.** (Todd) The store.

  - [ ] **3a.** `pub type ErrorsStore = Store<HashMap<String,
    Vec<ValidationMessage>>>` in `nodes.rs`, beside `ValuesStore`. `use_form`
    creates it, seeded from the state and leaving `field_errors` empty, and `Form`
    holds it. `Form::errors()`, `Form::has_errors()`.

  - [ ] **3b.** The private method above, with `Form::validate`, `edit`,
    `push_field_error`, `distribute_errors` and `absorb` going through it, and
    `apply_edit` taking the store. Trap 1 applies to the drain.

  - [ ] **3c.** `Form::reset` restores the initial snapshot's errors, writing an
    empty `Vec` over everything else. `Form::to_wire` reads the store (trap 2).

- [ ] **4.** (Todd) Widgets read the store.

  - [ ] **4a.** `NodeContext.errors: ErrorsStore`, and `WidgetProps.errors` becomes
    that store. A helper beside `get_current`, `pub fn get_errors(path, errors) ->
    Vec<ValidationMessage>`, for custom widgets.

  - [ ] **4b.** `FieldErrors` takes `path` and the store and reads for itself.
    `is_invalid` reads the store, and `control_class`, `part_class` and
    `field_label`'s `invalid` follow.

  - [ ] **4c.** `aria-invalid` and `aria-describedby` move from
    `FieldAttrs::with_owned_attrs` into the components (traps 3 and 4).

  - [ ] **4d.** Containers (`StructNode`, `ListNode`, `SetNode`, `VariantNode`,
    legends and `fx-invalid`) read their own key in `render`. That is the form's
    scope, which is fine under B: their errors change only on submit.

- [ ] **5.** (Todd) The live check. `FormState::check_live(&self, path, values:
  &ValuesByPath) -> Vec<ValidationMessage>`: clone the top-level node that owns
  `path`, `distribute_values` into the clone, `validate` it into a scratch map, and
  return `path`'s entry. Nothing written, so `Form` reads with `peek` and writes
  one store entry: no form re-render. Running the same `validate` is what
  guarantees that a live check and a submit agree (trap 5).

  A draft's leaves are off the walk, so `check_live` finds nothing and writes an
  empty `Vec`: typing into a draft clears the refusal Add left there, and Add
  judges it again.

- [ ] **6.** (Todd) The triggers, per A and E, on each control per G. `WidgetProps`
  gains a `Copy` handle, public so that a custom widget can use it as the built-ins
  do, with two calls beside `write_value`: one after a value is written (re-check
  if the field shows an error, else mark it typed into; a chooser checks at once),
  one on blur (check if it was typed into). `Form` keeps which paths were typed into
  since their last check (E) in a plain `Signal`, which no render reads.

- [ ] **7.** (Claude) `live_validation:` (F): `form!`'s key, its near-miss and
  given-twice tests, `FormSpec::with_live_validation`, `Formoxus`, and
  `NodeContext.live`.

- [ ] **8.** (Claude) Tests. Written against 2–6 as each lands.

  - [ ] **8a.** The existing suite: `has_errors` and `collect_errors` stay on
    `FormState`, so most of its 86 uses should pass as they are. Those reading
    through `Form` move to `Form::has_errors`. `builtin_widgets.rs:90` and
    `assignment_shape.rs:83` build or read `WidgetProps.errors` by hand.

  - [ ] **8b.** New `tests/suite/errors_store.rs`: one field's error does not
    re-render a sibling (a render counter in a custom widget); an error cleared
    wakes its widget (trap 1); a removed item's, changed variant's and emptied
    option's errors go; `reset` restores; `to_wire` keeps render order; a seeded
    state's errors show; a submit's errors arrive where a live check's would.

  - [ ] **8c.** `check_live`: every kind of leaf check; a blank field in a present
    and in an absent `Option<struct>` (trap 5); an item's path; a draft's; that
    `min_items` and a form validator never answer it (B).

  - [ ] **8d.** e2e, in pairs with `/t/live-*` forms: nothing shown while typing a
    first time; shown on blur; gone on the keystroke that fixes it; tab through an
    empty required field says nothing (E); a server's error survives a click in and
    out (E); a select's `onchange`; focus and caret not lost when an error appears,
    which only a browser shows; the knob off; a custom widget calling the two
    helpers, and one calling neither still validating on submit. Mutation-check
    each.

- [ ] **9.** (Claude) Docs and memory: `README.md` ("No live/as-you-type
  validation" goes), a custom widget's errors in `widgets.rs`'s docs, a gallery
  example, `FIELDS_PLAN.md` Step 7 pointing here, `error_model_design.md` rewritten
  to match, and #7 updated with what this leaves it.

- [ ] **10.** (Claude) `just ci` and `just e2e` green, then Todd merges to `main`.
  apcsp follows after, since it builds formoxus by path: anything of its own
  reading `WidgetProps.errors` changes then.

## Not in this plan

1. **`aria-invalid="false"` (#7).** Trap 1 makes it possible; nothing here uses it.
   **OPEN at 3a:** an empty `Vec` is also how an error is CLEARED (trap 1), so
   `reset`, 2e's rule and box 5's draft all leave "checked and clean" on a field
   nobody checked. `Option<Vec<…>>` as the store's value would tell them apart.
2. **Validating on load.** Silent, as it is today.
3. **Async or server-side live checks** ("that username is taken" as the user
   types). `check_on_server` is `FIELDS_PLAN.md` Step 8.
4. **`range`, `checkbox_multiple`, `select_multiple`, `file`.** Next, against the
   `WidgetProps` this leaves.
