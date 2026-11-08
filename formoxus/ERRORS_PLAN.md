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
  plain data; validate fills it,           Store<HashMap<String, Option<Vec<…>>>>
  collect_errors reads it                  seeded from the state, then the one
                                           home; widgets read it by path
```

**One home at rest.** On the client `FormState.field_errors` is a hand-off, not a
second copy: it is empty except during an operation. One private function,
`with_errors`, is what every operation goes through:

1. Take `state.write()`, and copy the store's `Some` entries into `field_errors`,
   empty ones too, since an empty one means checked and clean (#7). A `None` gets
   no entry, so the map says exactly what the store says.
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

1. **Clear an error by writing `None`, never by removing the key.** dioxus-stores
   0.7.10's `HashMap` `remove` and `retain` mark only the map shallowly dirty
   (`impls/hashmap.rs:174`, `:221`), so a widget reading that key through
   `get_unchecked` does NOT wake. `insert` does mark the key, which is why an absent
   key can be read and still wake on its first error.

   So a key, once written, stays, and the store's value is an `Option` (decided by
   Todd, 2026-11-05). No key or `None` means not checked, `Some` of an empty `Vec`
   means checked and clean (#7), and `Some` of messages means it has errors. Plain
   `Vec` would have made "cleared" look the same as "checked and clean".
2. **A `Store` cannot hold an `IndexMap`,** so the store's order is lost, and
   `collect_errors` promises render order. `Form::to_wire` orders the store's
   entries by `Node::paths`.
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

- [x] **2.** (Todd) Errors off the nodes. Plain data only, no store yet.

  Rewritten 2026-11-05 as a work order, after Claude built all of it in a throwaway
  copy to prove the order. Line numbers are c48402f's. The library first compiles
  after 2h and the tests after 2i, so a red build before then is expected.

  **The rules, for every node:**

  1. `validate(&self, prefix, out) -> bool` returns whether this node and everything
     under it is clean, and writes an entry for every path it checks, an empty `Vec`
     when clean (#7). A gated validator did not run, so it writes no entry.
  2. Loops run every child: `is_valid = child.validate(…) && is_valid`, in that order.
  3. A node adds its OWN name, so a container passes its own path down as the prefix.
     `OptionNode` adds nothing. `VariantNode` has two paths: its own errors sit at
     `my_path`, its children under `child_prefix(prefix)`, which takes the PREFIX.
  4. `paths` pushes a node's own path first, then its children: render order.
  5. A `FieldValue::Invalid` stays on its node: it is a value, not a message.
  6. `paths` walks an absent option's inner fields, and must: a form validator, a
     `push_field_error` and a server's reply can all name one, as an absent option's
     inputs are on screen. So 2g's rule does not cover blanking an `Option<struct>` or
     a leaf, which is typing, not an edit: box 5 handles that.

  - [x] **2a.** `FormState.field_errors: ErrorsByPath`, beside `errors`. Delete the
    `errors` fields of `FieldNode`, `StructNode`, `VariantNode` and `Items`.

  - [x] **2b.** `nodes.rs`: the trait in its final shape, in one edit, so the compiler
    lists everything left.

    - [x] `paths(&self, prefix, drafts: Drafts, out)`, with `Drafts::{Include, Exclude}`.
      It needs `#[derive(Clone, Copy, PartialEq, Eq, Debug)]`, or the first loop over
      children moves it.
    - [x] `edit(…, specs, errors: &mut ErrorsByPath)`.
    - [x] Delete `has_errors_within` and `has_errors_here`, with their docs. Nothing
      needs them once `validate` returns a `bool` and render reads the map, so
      `SetNode` was right not to have them: do not add them back.
    - [x] The render stopgap: `NodeContext.errors: Rc<ErrorsByPath>`, a `root`
      parameter, cloned in `nested` and `optional`, and `errors_at(&self, path) ->
      Vec<ValidationMessage>`. It is what lets the suite go green here instead of
      after box 4, where 29 tests would otherwise stay red (Todd agreed 2026-11-05):
      `WidgetProps.errors` stays a `Vec`, and 4a swaps this field for the store. `Rc` is
      allowed here, as `NodeContext` never reaches the server.

  - [x] **2c.** Finish what is nearly done.

    - [x] `field_node.rs`: `paths`' new parameter. Delete the commented-out block at
      `:275`. `render`: `let errors = ctx.errors_at(&ctx.path(&self.name))`, then
      `!errors.is_empty()` at `:315` and `errors` at `:333`, replacing the `todo!()`.
      Delete `has_errors_within`, `has_errors_here`. `edit` takes `_errors`.
      `validation_messages` (`:505`) validates into a map; take the node by reference,
      or clippy objects.
    - [x] `option_node.rs`: delete `let mut is_valid` (`:46`), `has_errors_within`,
      `has_errors_here` and its stale comment; drop the `qualify` import. Pass
      `drafts` and `errors` through.
    - [x] `items.rs`: `paths` takes `drafts`; `edit_within` takes and passes `errors`.
      Delete `errors()`, both commented-out blocks, `has_errors_within`, `add_errors`,
      `has_errors_here` and `clear_errors`.
    - [x] `list_node.rs`: `paths` and `edit` pass the new parameters. Delete
      `has_errors_within` and `has_errors_here`. `render`: `let own_errors =
      ctx.errors_at(&path)`, then `!own_errors.is_empty()` at `:68` and `own_errors`
      at `:97`.

  - [x] **2d.** `struct_node.rs`.

    - [x] `validate`, shaped like `ListNode`'s: children under `nested`, and only if
      clean, its validator, inserted at `nested`.
    - [x] `paths`: `nested`, then each child with `nested` as prefix.
    - [x] `render` (`:44`, `:62`): `own_errors` from `ctx.errors_at`.
    - [x] `edit` takes and passes `errors`.
    - [x] Delete `has_errors_within`, `has_errors_here`, `collect_errors`,
      `distribute_errors`, `clear_errors`, and the `ValidationMessage` import.

  - [x] **2e.** `variant_node.rs`.

    - [x] `validate`: `Unchosen` and `Unknown` write their one message at `my_path` and
      return false. `Named`: children under `child_prefix(prefix)`, then the validator
      at `my_path` only if they were clean.
    - [x] `paths`: `qualify(prefix, &self.name)`, then, if `child_prefix(prefix)` gives
      one, the children. Both `paths` and `validate` receive the PREFIX. This is
      what makes a changed or unchosen variant's old fields drop out.
    - [x] `set_variant`: delete both `self.errors.clear()` lines. `distribute_values`
      calls it too, and has no map to write to.
    - [x] `lookup_variant`: takes `&self`, records nothing, still returns the `Err`.
    - [x] `edit`'s `ChooseVariant` arm updates the map, since `set_variant` and
      `lookup_variant` no longer can. After a good choice, remove the enum's entry:
      `errors.shift_remove(&my_path)`. 2g's rule cannot do this, because the enum's path
      still exists, so "You must choose a variant" would stay on screen. After a stale
      name, record it: `errors.entry(my_path).or_default().push(…)`, with
      `unknown_variant_message(name)`, and still return the `Err`.
    - [x] `forward_to_child` takes and passes `errors`.
    - [x] `render`: `own_errors` from `ctx.errors_at` at `:256` and `:292`.
    - [x] Delete `has_errors_within`, `has_errors_here`, `collect_errors`,
      `distribute_errors`, `clear_errors`.

  - [x] **2f.** `set_node.rs`.

    **Decided (Todd, 2026-11-05): a draft's errors.** `paths` says whether to include
    drafts. `FormState::edit`'s rule includes them, so a refusal at a draft's path
    survives it. `collect_errors`, and so `has_errors` and `to_wire`, and unclaimed
    errors do not, so a refusal reaches none of them.

    - [x] `paths`: the parameter is the PREFIX, so `let my_path = qualify(prefix,
      &self.name)`. Then `my_path`, the items under it, and the draft under it (after
      the items, as on the page) only for `Drafts::Include`.
    - [x] `item_value` (`:206`): `node.validate("", &mut ErrorsByPath::default())`. It
      no longer needs the clone, as `validate` takes `&self`.
    - [x] A private `forget_draft_errors(&self, my_path, errors)`:
      `errors.retain(|path, _| !owns(&draft_path, path))`.
    - [x] `commit_draft` (`:575`) takes `errors`. Forget first, then if not present
      return `None`, then if `!self.draft.validate(my_path, errors)` return `None`.
      NOT a scratch map: its refusal shows on the draft's own widget. The forget
      replaces `:578`'s `clear_errors`, and is what clears a refusal on a draft since
      emptied: `validate` is never reached there. Keep `let candidate =
      self.draft.clone_box();` after those checks: `refuse_duplicate` takes `&mut self`,
      so it cannot be handed `&*self.draft`.
    - [x] `edit_item` (`:523`) takes `errors` and forgets first: it rebuilds the draft.
    - [x] `edit` passes `errors` to both, to `self.draft.edit` and to `edit_within`.
    - [x] `render_item` (`:458`): `item.paths(&path, Drafts::Exclude, &mut paths)`, then each
      one's `ctx.errors_at`. An item has no widgets of its own, so its `<li>` shows
      everything under it, as `collect_errors` did.
    - [x] `render`: `own_errors` from `ctx.errors_at` at `:841` and `:883`.

  - [x] **2g.** `form/state.rs`.

    - [x] A private `paths(&self, drafts: Drafts) -> Vec<String>`, over `self.nodes`.
    - [x] `form_for_impl` (`:457`): `field_errors: ErrorsByPath::default()`.
    - [x] `validate`: clear `errors` AND `field_errors`, run the loop (rule 2), and
      return `None` if it was not clean. `field_errors` and `nodes` are separate
      fields, so the borrows are fine.
    - [x] `collect_errors`: walk `self.paths(Drafts::Exclude)` and keep the non-empty entries.
      Walking rather than reading the map gives render order and leaves drafts out, in
      one place.
    - [x] `has_errors`: whether `collect_errors` has anything, form or fields. NOT a
      read of the map, or a refused draft would count.
    - [x] `distribute_errors(&mut self, errors) -> ErrorsByPath`: the same signature.
      A path in `self.paths(Drafts::Exclude)` extends its `field_errors` entry, and anything else
      goes in the map it returns. Its two callers then need no change.
    - [x] `edit` (`:306`): `let result = …edit("", edit, specs, &mut
      self.field_errors);`, then retain only what `self.paths(Drafts::Include)` holds, then
      `result`. Prune on `Err` too, which is why it is not `?`: a refused edit records
      its message before returning.

    **Two behaviours change here, both accepted (Todd, 2026-11-05):**

    1. A submit clears a draft's refusal, as `validate` empties the whole map, and Add
       judges the draft again. Today it stays, since `SetNode::validate` never touches
       the draft. 2j pins this.
    2. `has_errors` before any `validate` no longer sees a value that failed to parse.
       It reports only what `validate` has found, and no caller asks any sooner.

  - [x] **2h.** `form.rs`. The library compiles after this.

    - [x] `distribute_errors`: uncomment it, and replace the `clear_errors` loop with
      `state.field_errors.clear()`.
    - [x] `root_ctx` (`:437`): pass `self.state_signal.read().field_errors.clone()`.
      Render reads the state already, so this subscribes nothing new.

  - [x] **2i.** Green checkpoint. `just ci`, then `just e2e`.

    - [x] `tests/suite/forms.rs`: `:38` and `:66` lose `errors`; the three `FormState {
      … }` literals gain `field_errors: ErrorsByPath::default()`; `:197` and `:316`
      read `form.collect_errors().paths()` in place of `has_errors_within`.
    - [x] Stale comments: `ErrorsByPath`'s doc (`form.rs:38`), the `set_items` doc
      ("and errors"), `VariantNode::is_present`'s, and in tests `enums.rs:964`,
      `widgets.rs:315` and the header of `errors.rs`.
    - [x] `cargo fmt`. Claude's copy passed all 864 tests, all 58 e2e tests, clippy and
      `cargo doc` with only these changes. `msrv` was not run.

  - [x] **2j.** (Claude) Tests for what the suite cannot see. Each of these lines was
    broken on purpose in the copy, and **all 864 tests still passed**, so a green 2i
    proves less than it looks. Written once 2i is green, each mutation-checked:

    - [x] Choosing a variant clears "You must choose a variant" (2e).
    - [x] A stale variant name reports at the enum's path as well as in the `Err` (2e).
    - [x] Two bad fields in one variant both report (rule 2 in `VariantNode`).
    - [x] A refused draft stays out of `collect_errors`, `has_errors` and `to_wire`.
    - [x] A removed item's errors go, and a changed variant's fields' (2g's rule).
    - [x] Refused, then emptied, then Add: the refusal goes (`commit_draft`).
    - [x] Refused, then Edit on an item: the refusal goes (`edit_item`).
    - [x] A pushed error on an absent option's field is gone after the next `validate`.
    - [x] A refused draft's error is gone after a submit, and back on the next Add.
    - [x] Also the ones owed from review: a count error still reports its items' errors,
      and item paths pinned exactly (`tags.#0`, not `tags.#0.#0`).

- [x] **3.** (Todd) The store.

  - [x] **3a.** Create the store and fill it from the state. Nothing reads it yet, so
    no behaviour changes and every test still passes at the end of this step.
    `field_errors` keeps its contents here. Emptying it now would blank the render
    stopgap: 10 suite tests failed when that was tried in a copy, while this version
    passed all 715.

    - [x] Decide what the store's value is: `Option<Vec<…>>` (trap 1).
    - [x] `nodes.rs`, beside `ValuesStore`: `pub type ErrorsStore =
      Store<HashMap<String, Option<Vec<ValidationMessage>>>>;`, with a doc comment
      saying it is the client's home for field errors and what `None` means.
    - [x] `widgets.rs:99`: add `ErrorsStore` to the `pub use`, as `ValuesStore` is
      there. A custom widget needs the name in 4a.
    - [x] `Form`: a new field `errors_store: ErrorsStore`, beside `values_store`. A
      `Store` is `Copy`, so `Form` stays `Copy`.
    - [x] `use_form`: create it right after `values`, the same way. `peek`, not `read`,
      for the reason the comment there gives. The `collect` is needed because
      `field_errors` is an `IndexMap` and a store cannot hold one (trap 2).

      ```rust
      let errors = use_store(move || {
          state
              .peek()
              .field_errors
              .iter()
              .map(|(path, messages)| (path.clone(), Some(messages.clone())))
              .collect::<HashMap<_, _>>()
      });
      ```

    - [x] The `Form { … }` at the end of `use_form`: `errors_store: errors,`.
    - [x] `Form::errors(&self) -> ErrorsStore`, beside `Form::values`, returning the
      field.

  - [x] **3b.** Move the errors into the store. This was two steps, 3b and 3c, but it
    has to be one: nothing is right until all of it is in. Written in a copy, it passed
    all 1195 tests and clippy.

    **What changes.** Today `validate` and the rest write `field_errors`, and it stays
    full. After this, `field_errors` is empty between operations and the store holds
    the errors. For the length of one operation they are copied into `field_errors`,
    because every operation is written against the plain `FormState`, which is what lets
    the same code run on the server, where there is no store. So `FormState` does not
    change at all, except for one new reader in the last step.

    - [x] **`with_errors`, a private function in `form.rs`, beside `apply_edit`.** A
      function, not a method, because `apply_edit` has no `Form` either: `use_form`'s
      `on_edit` calls it before a `Form` exists. It copies the store's errors in, runs
      the operation, and writes back what changed:

      ```rust
      fn with_errors<T, R>(
          mut state: Signal<FormState<T>>,
          mut errors: ErrorsStore,
          operation: impl FnOnce(&mut FormState<T>) -> R,
      ) -> R
      where
          T: Clone + Debug + PartialEq + Facet<'static> + 'static,
      {
          let mut state = state.write();
          state.field_errors = errors
              .peek()
              .iter()
              .filter_map(|(path, messages)| Some((path.clone(), messages.clone()?)))
              .collect();
          let result = operation(&mut state);
          let after = std::mem::take(&mut state.field_errors);
          // A key is never removed (trap 1): what left the map becomes `None`.
          let gone: Vec<String> = errors
              .peek()
              .iter()
              .filter(|(path, messages)| messages.is_some() && !after.contains_key(*path))
              .map(|(path, _)| path.clone())
              .collect();
          for path in gone {
              errors.insert(path, None);
          }
          for (path, messages) in after {
              let messages = Some(messages);
              let changed = errors.peek().get(&path) != Some(&messages);
              if changed {
                  errors.insert(path, messages);
              }
          }
          result
      }
      ```

      The `gone` loop is what stops a stale error staying on screen: `validate` writes
      nothing for an absent option's fields, and `edit` drops paths, so an error can
      leave the map without anything writing to its key. The `changed` check is what
      keeps a field that did not change from re-rendering.

    - [x] **Each operation goes through it.** The body of each moves inside the closure,
      and the `let mut state = self.state_signal;` lines go.

      `validate`:

      ```rust
      with_errors(self.state_signal, self.errors_store, |state| {
          let values = state.ordered_values(&self.values_store.read());
          state.distribute_values(&values);
          state.validate()
      })
      ```

      `push_field_error`:

      ```rust
      with_errors(self.state_signal, self.errors_store, |state| {
          state.push_field_error(path.as_str(), message)
      })
      ```

      `distribute_errors`: its `let unclaimed = { … };` block becomes `let unclaimed =
      with_errors(self.state_signal, self.errors_store, |state| { … });`, with the same
      three lines inside. `absorb` calls it, so `absorb` needs no change.

      `reset`: only the `*state.write() = fresh;` line changes, to the `with_errors` line
      below. `fresh` is a copy of `initial`, which holds the errors the form started
      with, so the write-back puts those in the store and `None` over everything else.
      **The rest of `reset` stays:** `let leaves` before that line, and after it the
      `for` loop with its comment, which is what puts the VALUES back. Taking `leaves`
      first matters, because `fresh` moves into the closure.

      ```rust
      let leaves = fresh.leaf_values();
      with_errors(self.state_signal, self.errors_store, |state| *state = fresh);
      for (path, raw) in leaves {
          crate::widgets::write_value(&path, self.values_store, raw);
      }
      ```

      `push_error` stays as it is: a form-level error is not a field's (decision D).

    - [x] **`apply_edit` takes the store.** Add `errors: ErrorsStore` after `values`, and
      pass `errors` from `on_edit` and `self.errors_store` from `Form::edit`. Its one line
      that edits becomes this. The `?` is outside on purpose: a refused edit has recorded
      its message by then, and the write-back has to happen before it returns.

      ```rust
      let created = with_errors(state, errors, |state| state.edit(edit))?;
      ```

    - [x] **`use_form` builds its two signals the other way round.** The state a dev hands
      in can already hold errors, because a test or a page may validate before calling
      `use_form`. Those errors have to reach the store, and `reset` has to get them back,
      but the working state must start with an empty `field_errors`. Building `initial`
      first does that without writing to a signal during render. Then seed `errors` from
      `initial.peek()` in place of `state.peek()`.

      ```rust
      let initial = use_signal(state);
      let state = use_signal(move || {
          let mut working = initial.peek().clone();
          working.field_errors.clear();
          working
      });
      ```

    - [x] **`root_ctx` reads the store,** since `field_errors` is now empty when rendering
      happens. 4a replaces this stopgap.

      ```rust
      self.errors_store
          .read()
          .iter()
          .filter_map(|(path, messages)| Some((path.clone(), messages.clone()?)))
          .collect(),
      ```

    - [x] **`FormState::ordered_errors`, in `state.rs`, beside `ordered_values`.** It is
      `collect_errors` for a live form: it takes
      `&HashMap<String, Option<Vec<ValidationMessage>>>`, and keeps a path only when its
      value is `Some` of a non-empty `Vec`. The two share a private `errors_from`, which
      does the walk over `self.paths(Drafts::Exclude)`, so both keep render order (trap 2)
      and leave out a refused draft; each only says where to look a path up.
    - [x] **`Form::to_wire` calls it,** in place of `state.collect_errors()`, passing
      `&self.errors_store.read()`. `field_errors` is empty by then, so forgetting this
      would silently stop errors reaching the server.
      `a_live_forms_errors_reach_the_wire_and_has_errors` pins it.
    - [x] **`Form::has_errors(&self) -> bool`, beside `Form::errors`** (trap 6). Calls
      `ordered_errors` the same way, and returns whether its `form` or `fields` has
      anything.
    - [x] The comment in `on_edit` about an unknown variant name being "recorded against
      the `VariantNode`" now means the store: reword it.
    - [x] Checkpoint: `just ci`, then `just e2e`.

- [x] **4.** (Todd) Widgets read the store.

  - [x] **4a.** `NodeContext.errors: ErrorsStore`, replacing 2b's stopgap, and `WidgetProps.errors` becomes
    that store. A helper beside `get_current`, `pub fn get_errors(path, errors) ->
    Vec<ValidationMessage>`, for custom widgets. `None`, no key and an empty `Vec` all
    read as an empty `Vec` there, so a widget never sees the `Option`.

  - [x] **4b.** `FieldErrors` takes `paths: Vec<String>` and the store in place of
    `errors: Vec<ValidationMessage>`, and reads for itself with `get_errors`, one path
    after another. It is a component, so the read subscribes it, not the form. A list
    of paths, not one (Todd, 2026-11-07), because a set item has no single path:
    `render_item` shows every error under the item in one list, so it passes the
    `paths` it already walks, and every other caller passes its own path alone. That
    also ends 4a's stopgap in `field_row`, `radio_group` and `assignment_shape.rs`,
    which call `get_errors` to hand `FieldErrors` a `Vec`.

    `is_invalid` has read the store since 4a, and a read subscribes whoever is running
    when it happens. So what matters is where each call runs: a component wakes alone,
    while `Widget::render` and `Node::render` are the form's scope (trap 3).
    `control_class`, `part_class` and `field_row` (which calls `field_label`) all reach
    `is_invalid`.

    **Checked 2026-11-07: every built-in already calls them from a component, so this
    paragraph needs no code.** For each widget, `render` only hands over to the component:

    1. Text-like inputs and the checkbox: `render_input` to `InputView`, whose
       `text_like` and `checkbox` are plain functions, but run inside it.
    2. `Textarea` and `Select`: `TextareaView` and `SelectView`.
    3. `RadioGroup`: `RadioGroupView`, which calls `is_invalid` itself.
    4. `VariantSelect` is a component already.
    5. A hidden input shows no errors, so it never asks.

    A custom widget that calls these from its plain `render`, like `Notes` in
    `tests/suite/classes.rs`, subscribes the form. It still works, which trap 3 accepts.

    **The form is still woken by every error after this.** That is
    `ctx.errors_at` in `FieldNode::render` (`field_node.rs:295`), which 4c moves, and
    the containers' reads, which 4d keeps on purpose.

  - [x] **4c.** `aria-invalid` and `aria-describedby` move from
    `FieldAttrs::with_owned_attrs` into the components (traps 3 and 4). This is the step
    that stops an error waking the form. Today `FieldNode::render` reads the field's
    errors to work out those two attributes, and that read happens in the form's scope.

    Line numbers are 96100d2's. Built in a copy: all 716 suite tests and clippy passed,
    and an error written straight to one field's key went from re-rendering the form
    every time to not at all, with `aria-invalid`, `aria-describedby` and the error list
    still appearing and clearing, and the sibling untouched.

    - [x] **Split `with_owned_attrs` (`field_attrs.rs:175`) in two.** It keeps
      `required`, which comes from the `NodeContext` and never changes with errors, so
      it loses `field_id`, `invalid` and `help_text`:

      ```rust
      pub fn with_owned_attrs(is_checkbox: bool, required: bool, attrs: &FieldAttrs) -> FieldAttrs
      ```

      The two aria attributes go in a new method, which their doc bullets move to:

      ```rust
      pub fn with_error_attrs(&self, field_id: &str, help_text: Option<&str>, is_invalid: bool) -> FieldAttrs {
          let mut map: IndexMap<AttrKey, AttrValue> = IndexMap::default();
          let required = AttrKey::Std(Attr::Required);
          if let Some(value) = self.0.get(&required) {
              map.insert(required, value.clone());
          }
          if is_invalid {
              map.insert(
                  AttrKey::Std(Attr::AriaInvalid),
                  AttrValue::String("true".to_string()),
              );
          }
          if let Some(ids) = described_by(field_id, help_text.is_some(), is_invalid) {
              map.insert(AttrKey::Std(Attr::AriaDescribedby), AttrValue::String(ids));
          }
          map.extend(self.0.clone());
          FieldAttrs(map)
      }
      ```

      Two things in it are deliberate. `required` is copied first so the attributes keep
      their order, which the exact-markup tests pin. And `self`'s entries go in LAST, so a
      dev's own `aria_describedby` still replaces ours, as it does today.

    - [x] **`WidgetProps::current_attrs(&self) -> FieldAttrs`, beside `is_invalid`.** It
      is `self.attrs` with the field's error state (`self.base_attrs`, after the rename
      below), and it reads the store, so a component has to be what calls it. Its doc should
      say that, and that it is how a custom widget reaches a field's attributes:

      ```rust
      self.attrs
          .with_error_attrs(&self.id, self.help_text.as_deref(), self.is_invalid())
      ```

    - [x] **`FieldNode::render` (`field_node.rs:295`) stops reading errors.** Delete `let
      errors`, and pass only what is left. This is the line that matters most:

      ```rust
      let field_attrs =
          FieldAttrs::with_owned_attrs(self.is_checkbox(), ctx.required, &own.then(&self.attrs))
              .resolve_bounds(self.field_type());
      ```

    - [x] **The widgets spread `current_attrs` in place of `attrs`.** In `input.rs:211`
      and `:236`, `textarea.rs:49` and `select.rs:64`, `props.attrs.merge_with_attrs(…)`
      becomes `props.current_attrs().merge_with_attrs(…)`.

    - [x] **`RadioGroupView` (`radio_group.rs:82`):** `let field_attrs =
      props.current_attrs();`, in place of the reference to `props.attrs`. It has to stay
      above `let choices`, which moves a field out of `props` and so stops any method
      being called on it. The rest of the function needs no change, except for the `&`
      this drops.

    - [x] **`VariantNode::render` (`variant_node.rs:236`) applies both itself:**

      ```rust
      let attrs = FieldAttrs::with_owned_attrs(false, ctx.required, &field_attrs)
          .with_error_attrs(&id, self.help_text.as_deref(), !own_errors.is_empty());
      ```

      `VariantSelect` does NOT change to `current_attrs`, because the node passes it
      `help_text: None` (the hint renders under the legend), so `current_attrs` would
      lose the hint from `aria-describedby`. The enum's own errors wait for submit (B),
      so reading them in the form's scope is fine. That read stays, for 4d.

    - [x] **`WidgetProps::attrs` becomes `pub(crate) base_attrs`** (Todd, 2026-11-07). Left
      public, it is a trap: a custom widget spreading `props.attrs` compiles, and silently
      loses `aria-invalid` and `aria-describedby`, which is an accessibility defect nothing
      reports. No test noticed in the copy. So the two changes do different jobs:

      1. `pub(crate)` protects a dev. `props.attrs` outside the crate is a compile error,
         and `current_attrs` is the only way to the attributes.
      2. The rename protects the crate, where `pub(crate)` alone changes nothing. Every
         `props.attrs` stops compiling, so the compiler lists the sites, and it says what
         the value is: the attributes before the errors are applied.

      Sort each site the compiler lists into one of three. A spread onto an element
      becomes `current_attrs`, which is the step above: forget one there and that widget
      loses its aria attributes with no error. `VariantSelect`'s spread keeps
      `base_attrs`, as the node has already applied the error attributes. A `contains`
      check, `control_class` and `reject_if_not_required` keep `base_attrs`, as none of
      them looks at the aria attributes, and a read of the store would be wasted.

      `FieldNode::render` and `VariantNode::render` set the new name, and the docs on
      `help_text` and elsewhere that say "`attrs`" follow.

    - [x] **A public constructor for `WidgetProps`.** Code outside the crate can no longer
      write a struct literal, since one private field forbids it and `..` does not get
      around that. Only two tests do (see below), and neither `examples/` nor apcsp, but
      those tests deliberately build props by hand, to pin the widget boundary apart from
      the nodes, and they are integration tests, so they see only the public surface.

      Take the fields that have no sensible empty value, `attrs` among them, and leave
      `label`, `help_text`, `choices` and `extra_attrs` empty: they stay `pub`, so a caller
      sets them afterwards. Six arguments or fewer stays inside clippy's limit of seven.
      Its doc should say that a widget is handed its props, so this is for a test or a
      widget that renders another, not something a dev normally needs.

    - [x] (Claude) The inline tests in `field_attrs.rs`. Once the constructor is in,
      `builtin_widgets.rs:86` and `plain_props` in `widgets.rs:180` move to it, and
      `Spreading` in `author_attrs.rs:178` moves to `current_attrs`. Add a test that a
      custom widget spreading `current_attrs` gets `aria-invalid` and `aria-describedby`:
      the compiler now stops a dev reading `attrs`, but only a test proves the attributes
      arrive.

    **What this breaks, on purpose.** A custom widget reading `props.attrs` stops
    compiling, and has to call `current_attrs`. Box 9's docs say so, and apcsp's
    `crates/ui/src/markdown.rs` (`:70` and `:93`) follows at box 10.

    **What this does not change.** `with_errors` writes the state, so a submit, an edit
    and a pushed error all still re-render the form once, by design. It is a live check
    that must not, which is why box 5 writes only the store. So a test of this has to
    write to the store directly, as box 8b's will.

  - [x] **4d.** Containers (`StructNode`, `ListNode`, `SetNode`, `VariantNode`,
    legends and `fx-invalid`) read their own key in `render`. That is the form's
    scope, which is fine under B: their errors change only on submit.

    **Checked 2026-11-07: no change needed.** Each already reads its own key through
    `ctx.errors_at`, which has called `get_errors` on the store since 4a
    (`struct_node.rs:45`, `list_node.rs:68`, `set_node.rs:850`, `variant_node.rs:200`),
    and no `render` reads `field_errors` any more. The read subscribes the form to that
    key, which costs nothing, as only a submit or `validate` writes it, and those
    re-render the form anyway. Box 5 needs that subscription to stay (see there).

- [ ] **5.** (Todd) The live check. Validate one field as the user edits it, and
  write only the store, so the form does not re-render. Box 6 calls it from the
  controls' event handlers; this box only provides it.

  **How it works.** Clone the top-level node that owns `path`, `distribute_values`
  the store's current values into the clone, `validate` the clone into a scratch map,
  and write `path`'s result to the store. Running the same `validate` a submit runs is
  what guarantees the two agree (trap 5). The clone is needed because a leaf cannot be
  judged alone, and the state's own nodes are stale until the next
  `distribute_values` (trap 5 again).

  **It also clears siblings (Todd, 2026-11-05).** Blanking the last filled field of
  an `Option<struct>` makes it absent, and the form valid, while its siblings still
  show the "required" a submit gave them. It only ever clears, never adds, so B
  stands: a sibling the user has not reached is not scolded.

  **Which siblings: only those under the option this check found absent (Todd,
  2026-11-07, "Option 2").** A live check must leave alone what `validate` cannot
  re-derive, or it wipes it: a server's "that username is taken" (E), a pushed
  error, a form validator's at a path, and a container's own error, which waits for
  submit (B). But every error beneath an absent option stops applying, whoever wrote
  it, so those, and only those, go. Asked of the clone alone, since the state's nodes
  are stale and there is no "before" to compare with. A form validator's error under
  that option can go early, and comes back on submit if it still holds.

  Rejected: a second parameter on `paths`, with "under an absent option" as the
  difference of two walks. It is mechanical, but it clears under EVERY absent option
  in the top-level node, so typing in `contact.email` would wipe a form validator's
  "phone needed" on an absent `contact.phone`, which nothing brings back before
  submit.

  - [x] **5a. `LiveCheck`, in `form/state.rs`** (Todd, 2026-11-07), what
    `FormState::check_live` returns:

    ```rust
    pub(crate) struct LiveCheck {
        pub errors_at_path: Option<Vec<ValidationMessage>>,
        pub paths_to_clear: Vec<String>,
    }
    ```

    `errors_at_path` is `Some` when `path` was validated, an empty `Vec` meaning
    checked and clean (#7), and `None` when nothing validates it now: under an absent
    option, or in a draft. `paths_to_clear` are bare paths, not pairs with an
    `Option`, so the type itself says a sibling is only ever cleared. There is no
    `path` field: the caller passed it in, and a copy could only disagree with it.

  - [x] **5b. A new `Node` method that names the absent option around a path.**
    Something like `fn absent_owner(&self, prefix: &str, path: &str) ->
    Option<String>` (the name is yours): the path of the OUTERMOST option that owns
    `path` and is absent, or `None`. Outermost, because everything under it goes,
    an inner absent option's paths included.

    1. **A default of `None` in the trait**, which is right for `FieldNode`: a leaf
       contains nothing.
    2. **`OptionNode`** checks itself BEFORE asking its inner node, which is what
       makes the answer the outermost. Its own path is `qualify(prefix,
       &self.name())`, since `name()` delegates to the inner node and an option adds
       no segment (see its `edit`). If that path does not `owns` `path`, `None`. If
       it does and `!self.is_present()`, `Some` of that path. Otherwise ask the inner
       node with the same `prefix`.
    3. **The containers recurse only into the child that owns `path`,** with the
       prefix their own `paths` uses: `StructNode` its fields, `VariantNode` the
       chosen variant's nodes through `child_prefix` (an unchosen one has none),
       and `ListNode` and `SetNode` their items through `Items`, which needs a
       matching method beside `Items::paths`. Check `owns` on the container's own
       path first, so a walk never descends into a node that cannot hold `path`.
    4. **`SetNode` asks its items, not its draft.** A draft is never validated
       (`Drafts::Exclude`), so nothing beneath it needs clearing; a draft's path
       gets `errors_at_path: None` anyway (see below).

    The `owns` helper (`nodes.rs:141`) is the one to use throughout: it already
    refuses a mere string prefix (`address2` is not under `address`).

  - [x] **5c. `FormState::check_live(&self, path: &str, values: &ValuesByPath) ->
    LiveCheck`,** in `form/state.rs` beside `validate`.

    1. **Find the top-level node** whose name `owns` `path`. If none does, return a
       `LiveCheck` with `None` and nothing to clear: the caller passed a path the form
       does not have, and writing `None` there is harmless.
    2. **Clone it** (`Box<dyn Node>` is `Clone`, `nodes.rs:129`) and call
       `distribute_values("", &self.spec.fields, values)` on the clone. ALL of
       `values`, not only `path`'s: whether an option is present depends on its
       other leaves (trap 5).
    3. **`validate("", &mut scratch)` on the clone,** into a fresh `ErrorsByPath`.
       Its return value is not needed. `errors_at_path` is `scratch`'s entry for
       `path`, cloned or removed. A container's entry in `scratch` (a struct's
       validator) is ignored: B says those wait for submit.
    4. **`paths_to_clear`:** if the clone's `absent_owner("", path)` is `Some(owner)`,
       the clone's `paths("", Drafts::Exclude, …)` that `owner` `owns`, minus `path`
       itself, which `errors_at_path` already covers. Otherwise empty. `paths` walks
       an absent option's fields (settled 2026-11-05), which is what makes this find
       them.

    **Nothing here writes,** which is the point: `&self`, and the clone is dropped.
    So it runs on the server too, though nothing calls it there.

  - [x] **5d. `Form::check_live(&self, path: &str)`, `pub(crate)`,** in `form.rs`,
    which box 6's handle calls.

    1. **Read with `peek`**, never `read`: the state, and the value store for
       `state.ordered_values(&…)`. It runs in an event handler, where `read`
       subscribes nothing anyway, but `peek` says what is meant, and a later caller
       in a render would otherwise subscribe the form.
    2. **Write `errors_at_path` only if it differs** from the store's current value,
       compared as `with_errors` does (`form.rs:887`), so an unchanged field's
       widget does not wake on every keystroke.
    3. **Write `None` to each of `paths_to_clear` whose store value is `Some`,**
       empty or not: "checked and clean" stops applying under an absent option too.
       Skip the rest, which are already clear.
    4. **Never through `with_errors`, and never `state.write()`.** Either re-renders
       the whole form, which is what this box exists to avoid.

  **Trap: the containers' reads (4d) must stay subscribing reads, never `peek`.** A
  cleared sibling can be a container's own key, such as a struct validator's error
  under an `Option<struct>` that has just become absent. The form re-renders only
  because its `render` read that key, so the stale error goes. Read it with `peek`
  and the error stays on screen after the live check cleared it, with no test failing
  unless one covers exactly that case.

  **A draft.** A draft's leaves are off the walk, so `scratch` has no entry and
  `errors_at_path` is `None`: typing into a draft clears the refusal Add left there,
  and Add judges it again. `ordered_values` leaves drafts out of `values`, which is
  fine, since the clone never validates one.

  - [x] Checkpoint: `just ci`. Claude's tests are 8c, against 5c and 5d.
    Done 2026-11-08: `just ci` 1206. Eight inline tests of `FormState::check_live` in
    `form/state.rs` (the 8c cases that need no store), each mutation-checked: an
    option returning the edited path, an option asking its inner node first, and a
    struct asking only its first field all fail them. `Form::check_live` carries an
    `#[expect(dead_code)]` until box 6 calls it; its store writes are tested then.

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
    wakes its widget (trap 1); a removed item's and changed variant's errors go;
    a submit clears an absent option's stale error from the store (3b); `reset` restores; `to_wire` keeps render order; a seeded
    state's errors show; a submit's errors arrive where a live check's would.

  - [ ] **8c.** `check_live`: every kind of leaf check; a blank field in a present
    and in an absent `Option<struct>` (trap 5); blanking the last filled field clears
    its siblings' errors, and a sibling's error survives while the option stays
    present; an error under a DIFFERENT absent option in the same top-level node
    survives (Option 2's reason); the outermost absent option wins over an inner one;
    an unchanged result writes nothing; an item's path; a draft's; that
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

1. **`aria-invalid="false"` (#7).** Trap 1's `Option` makes it possible; nothing here
   uses it.
2. **Validating on load.** Silent, as it is today.
3. **Async or server-side live checks** ("that username is taken" as the user
   types). `check_on_server` is `FIELDS_PLAN.md` Step 8.
4. **`range`, `checkbox_multiple`, `select_multiple`, `file`.** Next, against the
   `WidgetProps` this leaves.
