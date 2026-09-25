# Radio group: build steps

Written 2026-09-25 to be picked up in pieces. Each step ends with a check that
says it's done, and each is a safe place to stop. Commit after each one, and
`git log --oneline -5` shows where you left off.

## Decisions already made

- **`radio_group` is rejected on any `Option<T>`, at compile time.** A picked
  radio can't be un-picked, so an optional field would need a synthetic "none"
  radio. Ours would say `--none--`, a poor stand-in for the explicit "None of the
  above" / "I do not know" that GOV.UK and NN/g both call for when "no answer" is
  valid. If it is valid, the author makes it a real choice value, or uses
  `select`.
- **formoxus never pre-selects a radio.** NN/g says always offer a default;
  GOV.UK says never pre-select, because users miss the question or submit the
  wrong answer. A library that silently checked the first choice would make a
  required radio field unable to fail its "required" check. When the author knows
  a sensible default, the planned `initial` key (`CONSTRAINTS_PLAN.md`) is the
  place for it. In edit mode, the stored value is checked anyway.
- **The choices rule is `select`'s:** a `bool` renders without `choices`, and
  anything else needs them. `form!` already enforces that for `radio_group`, via
  the `"Select" | "RadioGroup"` arm of `rule()` in
  `formoxus-macros/src/form/widget.rs`.

## Steps

**0. Branch.** `git checkout -b radio-group` from `main`.

**1. Reject `radio_group` on an `Option` at compile time.**
- In `formoxus/src/field_kind.rs`, add
  `pub const fn is_optional(shape: &Shape) -> bool { matches!(shape.def, Def::Option(_)) }`.
  Check `shape.def` directly, not `kind()`, because `kind()` looks through
  `Option` on purpose.
- In `WidgetRef::checks` in `formoxus-macros/src/form/widget.rs`, when the name
  is `radio_group`, add one more `const _` assert spanned onto the widget name:
  `!::formoxus::field_kind::is_optional(#shape)`, with a message like
  "`radio_group` cannot render an optional field — a picked radio can't be
  un-picked; use `select`". Leave its `rule()` arm shared with `select`.
- Add a unit test for `is_optional` (`Option<bool>` and `Option<String>` yes;
  `bool` and `String` no), and a golden
  `formoxus/tests/ui/form_radio_group_on_an_option.rs` with
  `widget: radio_group { choices: STATES }` on an `Option<String>`. Generate it
  with `TRYBUILD=overwrite cargo test -p formoxus --test compile_fail`, then read
  the `.stderr`.
- Done when: `cargo test -p formoxus --lib field_kind` passes, and the golden
  shows the message with the caret on `radio_group`.

**2. Create the component file.**
- Create `formoxus/src/widgets/radio_group.rs` with a stub
  `#[component] pub fn RadioGroup(values: ValuesByPath, choices: Vec<SelectChoice>, props: FieldProps) -> Element`,
  the same signature as `Select` in `formoxus/src/widgets/select.rs`.
- In `formoxus/src/widgets.rs`, add `pub mod radio_group;`, and
  `pub use radio_group::RadioGroup;` beside the other re-exports.
- Done when: `cargo check -p formoxus` passes.

**3. Wire it into dispatch.** In `formoxus/src/widgets/scalar.rs`, add two arms
modelled on `select`'s:
- `(Text | Int | Float, RadioGroup)`: panic if `choices` is `None`.
- `(Bool, RadioGroup)`: fall back to `choices.unwrap_or_else(bool_choices)`.
- In both, also panic if `!props.required`, naming the field and saying to use
  `select`. `form!` can no longer produce either case, but a hand-built
  `FormSpec` still can.
- Done when: it compiles, and
  `cargo run -p formoxus-examples --bin widget_matrix` shows `radio_group`
  rendering on `flag`. The other columns still say PANIC because the matrix
  passes no choices; `select` does the same.

**4. The markup.** Fill in `RadioGroup`:
- `fieldset { class: "form-field radio-group", … }`, with a `legend` holding the
  label and the ` *` required marker.
- One
  `label { input { r#type: "radio", name: "{path}", value: "{choice.value}", checked: choice.value == current, required: true, aria_invalid: invalid, onchange: … } "{choice.display}" }`
  per choice, with `current = get_current(&path, values)`.
- `onchange: move |e: FormEvent| write_value(&path, values, e.value())`. For a
  radio, dioxus-web 0.7.10 gives you the input's `value` attribute; only
  checkboxes are special-cased (`dioxus-web/src/events/form.rs`).
- `FieldErrors { errors }` at the end.
- **No pre-selection and no "none" radio.** In create mode nothing is checked,
  and `required` makes the browser block submit until one is picked (on a radio,
  `required` applies to the whole group). In edit mode the stored value comes
  back as `current` and is checked.
- Done when: it compiles.

**5. Tests.** In `formoxus/tests/suite/choices.rs`, use the `select` tests'
pattern (`#[component] fn App`, `render(App)`, and the `Address`/`Flagged`
models with `STATES`):
- one `type="radio"` per choice, with `name="state"` and the value and display
  separate;
- nothing `checked` in create mode;
- in edit mode (`form_for` with a model whose `state` is `"AK"`), exactly that
  radio is `checked`;
- a `bool` with no choices renders True and False;
- every radio has `required`, and there's no `--none--`.
- Done when: `cargo test -p formoxus --test suite choices` passes.

**6. Check it for real (optional).** Add a `radio_group` field to
`examples/src/examples/select.rs` and `just serve`. Click through, submit, and
check that the value round-trips and that submitting with nothing picked is
blocked. Add a `.radio-group` rule to `examples/assets/main.css` if the fieldset
needs it.

**7. Tidy.**
- Add `RadioGroup` to the component list in `formoxus/src/widgets.rs`'s module
  docs.
- In `.claude/memory/widget_table_and_choice.md`, replace the "one deliberate
  divergence" sentence: `radio_group` now renders, and `Option` fields are
  rejected.
- Delete this file.
- Done when: `just ci` passes. Then merge to `main` and push.
