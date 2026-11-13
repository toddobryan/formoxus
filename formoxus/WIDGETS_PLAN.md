# The missing widgets: `range`, `checkbox_multiple`, `select_multiple`, `file`

Written 2026-11-11 by Claude, after Todd asked for one plan covering all four, with
`file` at the end. Next on the road to publish (`.claude/memory/mvp_scope.md`, item 2):
this, then #22 and #23, then the `form!` grammar docs and the doc pass. `file` is the
one apcsp needs (students uploading assignments), so it should not wait on anything it
does not depend on; see decision N.

Who does what, as before: Todd writes the runtime; Claude writes the tests (first,
where a test can come first), the `form!` side and its goldens, the e2e forms and
tests, the gallery entries, the docs and the memory.

**Three parts, each landing on its own.** They share almost nothing. `range` is one
more `<input>`. The two multi-pick widgets are a new kind of thing, a widget on a
*set* rather than on one leaf. `file` adds a model type and a second way for a form
to reach the server. None of the three is a radical change, so each is a short
branch merged when green, not one long one (decision N).

## Why

1. **`form!` advertises widgets it cannot render.** `select_multiple`,
   `checkbox_multiple` and `file` are in `WidgetName::NOT_YET`
   (`formoxus-spec/src/widget_names.rs:84`), so a dev who writes one is told it is
   "not supported yet". `range` is not even listed, though `InputType::Range` exists
   and the attribute table already allows `min`, `max` and `step` on it.
2. **Picking several values from a list is the parity gap that is left.** Django and
   leptos_form both have it (`formoxus_feature_parity.md`). Sets exist now
   (`SETS_PLAN.md`), and a set of choices is exactly what a checkbox list edits, so
   this is mostly a new way to render something formoxus already models and already
   sends across the wire.
3. **apcsp needs `file`.** A form with an upload cannot be built with formoxus today
   at all.
4. **Everything that moves the public surface lands before the doc pass**
   (`mvp_scope.md`, principle 2). All three parts add public API.

## Decisions (Todd, 2026-11-11)

- **A, E, F, H, J, M, N: as recommended.**
- **B: (i), with a bare flag `omit_value` to leave the `<output>` out**, written on the
  field's line like `no_trim` and `set`, and named for the non-default as they are:
  `volume => { widget: range, min: 0, max: 11, omit_value }`. `form!` refuses it on any
  widget but `range`, as it refuses `no_trim` on a number, and emits it as
  `RangeInput { omit_value: true }`; a hand-built spec writes that. Not inside
  `widget: range { … }`: those braces are Rust struct-field syntax, where a bare
  `omit_value` is shorthand for `omit_value: omit_value`, a variable. A `false` default
  also lets `Default` be derived.
- **D: (i), and choices written on the items' line are an error for these widgets.**
  Todd: something that applies to the whole widget, written on the items, is a mistake.
  So for a set with a `widget:`, `cities[] => { choices: CITIES }` is a `form!` error
  that says to write the choices on `cities`. **Only for a set with a `widget:`**
  (confirmed by Todd). A set rendered the ordinary way has no set-level widget: its
  items' choices are what its DRAFT's `<select>` shows, so on `cities[]` they are on the
  widget they apply to, by the same principle, and stay there.
- **G: (i), reshaped as a family of traits** (Todd's suggestion; the reasoning is under
  decision G′ below). `SetWidget: Widget` as a supertrait was considered and does not
  work: every `SetWidget` would then BE a leaf widget, with a leaf `render` it could only
  stub, and would type-check as `widget:` on a `String`.
- **K: revisit at box 10.** In memory on the client is not the problem: a `FileData` is
  the browser's file handle, not its bytes, and stays on disk until it is sent, as long
  as formoxus never calls `read_bytes` into wasm. The SERVER is: as K(i) is written,
  each file is buffered whole, and 150 students uploading 20 MB each at once is 3 GB at
  the peak. The alternative to weigh then: stream each file part to disk or object
  storage as it arrives, counting its size as it goes (and stopping past `max_size:`),
  so the model holds a handle (`Upload { name, content_type, size, stored }`) rather than
  the bytes. axum's `Multipart` streams parts, so this is possible; where the bytes go
  is then a dev's choice formoxus has to leave room for. Box 10's spike measures both.
- **L: one file per field.**
- **O (found at step 3h, decided by Todd at home 2026-11-11): a set of a unit enum gets a
  default order, the variants' declaration order.** Until now only a set of scalars had
  one (SETS_PLAN 4a); a set of a unit enum panicked at build without a `sort_by`, widget
  or not. A multi-pick over an enum is what `checkbox_multiple` is mostly for, so every
  dev would have hit it. Declaration order is also the order `choices` lists the
  variants in, so a widget's ticked items line up with its boxes. A dev who wants
  another order (alphabetical, say) still writes `sort_by:`.
- **T (Todd, 2026-11-11, from question P's `min_items: 1, max_items: 1` trap): a list or
  set of at most one item is refused.** Exactly one (`max_items: 1` with `min_items` of
  1 or more) should be the item's own type `T`; none or one (`max_items: 1` alone)
  an `Option<T>`. A set's message adds that a `select` or `radio_group` picks it.
  `form!` refuses a literal `max_items: 1` at compile time (`at_most_one_checks`,
  goldens `form_{list,set}_of_{exactly,at_most}_one`); the build panics for a
  hand-built spec or a non-literal limit (`refuse_at_most_one` in `build.rs`). Two
  (`min = max = 2`) is a real "pick exactly two" and stays allowed.

### Decision G′: a family of widget traits

Today `Widget` holds everything any widget needs, and a widget on a set would make half
of it meaningless. Split it by who reads what:

- **`Widget`**, the base: `NAME`, `PARTS`, `EXTENDS`. This is what the class system
  (`Classes::part`, `has_part`, `field_row`: `classes.rs:70`–`502`) and `form!`'s
  attribute checks read, and nothing more. Every kind of widget has these.
- **`LeafWidget: Widget`**: `ACCEPTS`, `ACCEPTS_OPTIONAL`, `TAKES_CHOICES`, decision A's
  `value_when_empty`, and `render(&self, WidgetProps)`. What `field_kind::renders` and
  `optional_is_okay` read, and what `AnyWidget` boxes. Every widget today is one of these.
- **`SetWidget: Widget`**: `render(&self, SetWidgetProps)`, and an `AnySetWidget`.

**Why `SetWidget` has no `ACCEPTS`.** `ACCEPTS` is a `FieldTypeSet`: which of the four
scalar types (`Text`, `Int`, `Float`, `Bool`) one leaf can hold. `renders` sorts the
field's shape into one of those and asks whether the widget's set contains it. A set
widget's question is a different one, what kind of COLLECTION of what kind of ITEM (a
set, not a list; of choice leaves or unit enums, not structs), which a `FieldTypeSet`
cannot say. Decision E's checks are its `ACCEPTS`. If a later set widget needs to narrow
them, it gets a const of its own kind then.

**The cost:** every `impl Widget for` becomes `impl LeafWidget for` plus a three-const
`impl Widget for`, in 24 places here and apcsp's `MarkdownWidget`. Breaking, so before
publishing, which is now.

**The names: `Widget`, `LeafWidget`, `SetWidget`** (Todd, 2026-11-11). "Leaf" is already
public vocabulary (`nodes::RawValue::Leaf`, `field_node::LeafArgs`, the `widgets` module
doc's "one component per leaf"). `SingleItemWidget` was considered and set aside: "item"
already means one element of a list or set (`#3`, `ItemValue`, `min_items`), and a leaf
widget renders no items while a set widget renders many. `FieldWidget` was the old derive
path's name.

## Part 1: `range`

A `RangeInput` widget, `widget: range`, rendering `<input type="range">` for an
integer or float field. Most of it is a row in `text_like_inputs!`. Two things are
not, and they are decisions A and B.

### Decision A: what a range shows when its field is empty

A slider cannot show "no value". Given `value=""`, the browser shows the midpoint of
`min` and `max` (HTML's value sanitization for `type=range`: the default value is
`min + (max − min) / 2`, with `min` 0 and `max` 100 when absent). formoxus's store
would still hold `""`. So on an `empty_form` the user sees 50 and the form holds
nothing: submit without touching the slider and an `i32` field says "required", about
a field that visibly has a value, and the only fix is to wiggle the slider off 50 and
back.

- **(i) Recommended.** `form!` requires both `min:` and `max:` on a `range` (a
  compile error without them), and a range field that would start empty starts at
  the value the browser would show: `min + (max − min) / 2`, then, if that is not on
  a step, the nearest value that is (`min` plus a multiple of `step`), the larger of
  two equally near, and never past `max`. That is HTML's rule for a step mismatch, so
  formoxus must compute exactly what the browser does, not something close. It is put into the field when the node is built,
  so the store, the model the form would produce and the screen agree from the first
  render, and `reset` returns to it. The hook is general rather than special-cased:
  a new `Widget` method, say `fn value_when_empty(attrs: &FieldAttrs) ->
  Option<String>`, defaulting to `None`, which `RangeInput` overrides. A dev's own
  slider can then say the same thing. Requiring `min` and `max` keeps the starting
  value in plain sight, instead of a 0–100 that nobody chose for an `i32`. A float
  range must be given `step:` as well (Todd, 2026-11-11): HTML's default step is 1, so
  without one it offers only whole numbers.
- (ii) Keep `""` and let the slider show a value the form does not have until it is
  touched. Cheapest, and wrong in the way described above.
- (iii) Refuse `range` on any field the model does not give a value, so it works
  with `form_for` only. Breaks `empty_form`, which is how every create page starts.

### Decision B: whether the number is shown

A bare slider shows no number. Screen readers are fine, since a range exposes its
value natively, but a sighted user dragging toward 37 cannot see they are at 37.

- **(i) Recommended.** Render an `<output for="{id}">` after the slider, holding the
  current value from the store, as a new class part. That needs `Output` added to
  `HtmlElement` and `Tag` in `formoxus-spec`, which is where every element a widget
  part can be is listed.
- (ii) No number. A dev who wants one writes their own widget.

### Settled by HTML, not decisions

- **`ACCEPTS` is `Int` and `Float`.** Not `Text`: a slider cannot produce a word.
- **`ACCEPTS_OPTIONAL` is `false`.** A slider cannot be unset, so it cannot say
  `None`. `form!` then refuses `range` on an `Option<i32>`, as it refuses `checkbox`
  on an `Option<bool>`, and needs a sentence in `optional_refusal` saying so.
- **No `required` attribute.** HTML does not allow it on a range (the attribute
  table agrees: `Required`'s `on:` leaves out `Range`, `formoxus-spec/src/attrs.rs:545`),
  and a range always has a value anyway. `FieldAttrs::with_owned_attrs`
  (`field_attrs.rs:169`) adds `required` to every required field except a checkbox,
  and it has to stay there: `Required` in `base_attrs` is also how a widget learns its
  field is required (`RadioGroup` panics without it and moves it onto each radio;
  `Select` picks its placeholder by it). So `RangeInput` takes it out of its SPREAD,
  `current_attrs().without(Attr::Required)`, as `RadioGroup` takes it off its
  `<fieldset>` (`radio_group.rs:94`). Corrected 2026-11-11: the draft said to decide
  it in `with_owned_attrs` from the attribute table, which would have stripped it from
  every radio group, whose `EXTENDS` is the fieldset's.

## Part 2: `checkbox_multiple` and `select_multiple`

Both edit a SET of choices: one checkbox per choice, ticked when that choice is in
the set, or one `<select multiple>` with the chosen options selected. Neither is a
leaf widget. Each needs the set's items and the items' choices, and each changes the
set by adding and removing items, which is what the set's adder and Remove buttons
already do. So the model, the store layout and the wire format stay exactly as they
are for any set (`cities` = `#0,#2`, `cities.#0` = `paris`), and so do `min_items:`,
`max_items:` and the set's validation.

### Decision D: where `widget:` and `choices:` are written

Today a set's choices are written on its items' line, `cities[] => { widget: select,
choices: CITIES }`, because they are the items' choices: the draft's `<select>` shows
them.

- **(i) Recommended.** Both on the set's own line:
  `cities => { widget: checkbox_multiple, choices: CITIES }`. `form!` files the
  choices under the items' spec (`cities[]`), which is where every other part of a set
  already reads them, so nothing at run time changes. Writing them on `cities[]`
  instead stays legal; writing them in both places is a compile error. The choices are
  what this widget shows, and splitting one widget's settings across two lines reads
  as two unrelated settings.
- (ii) `widget:` on the set and `choices:` on `cities[]`, as today. One meaning per
  key and no forwarding, but every use of these widgets takes two lines.

### Decision E: what the two widgets accept

- **(i) Recommended.** A set (`BTreeSet`, `HashSet`, or a `Vec` marked `set`, and an
  `Option` of any of these) whose items are either a leaf given `choices:` or a unit
  enum, whose variants are the choices, label-cased the way a variant `<select>`
  shows them. `BTreeSet<Weekday>` as a row of checkboxes with no `choices:` at all is
  the canonical use. Not a plain `Vec`: order and duplicates mean nothing to a list
  of checkboxes, and a reorderable list has no way to show its order here. Not struct
  items, which have no single value to tick. All of this is refused at compile time,
  by `const fn`s beside `field_kind::renders`.
- (ii) Leaf items with `choices:` only, unit enums later.

### Decision F: how a widget adds an item it knows only by its string

A checkbox knows "paris". `Edit::AddItem { value: Some(…) }` needs an `ItemValue`
built from a typed `T` with the item's shape (`items.rs:180` refuses any other), and
`value: None` on a set means "commit the draft" (`set_node.rs:778`). Neither lets a
widget add "paris".

- **(i) Recommended.** `ItemValue::raw(entry: impl Into<String>)`: an item named by its
  ENTRY, exactly the string the wire carries at the item's own path (`cities.#2` =
  `paris` for a leaf item, `days.#2` = `Mon` for a unit-enum item, whose variant name
  is its structure entry). `add_item` builds a blank item and gives it that entry,
  the same way `distribute_values` builds an item from the wire. The shape check is
  skipped because there is no shape to compare; the parse is the check instead. An
  entry that does not parse becomes an `Invalid` item, and one outside `choices:` is
  already refused on the item (`set_item_errors.rs`). The widgets then use the
  existing `AddItem` and `RemoveItem`, so `Form::edit`'s store and error bookkeeping
  for those two is reused unchanged. That bookkeeping is where box 8 of PATHS_PLAN
  caught the lost `!`, so it is worth not writing a second copy.
- (ii) A new `Edit::SetItems { path, entries: Vec<String> }` that replaces the set's
  contents in one edit. Atomic, which suits `select_multiple`'s one-change-many-items
  events, but it needs its own error cleanup for removed items.

### Decision G: what kind of thing a multi-pick widget is

A `Widget` renders one leaf from `WidgetProps`: one path, one value, through a
`FieldHandle`. A multi-pick widget needs the set's path, its choices, which of them
are in the set and at what position (to remove one), the edit callback, and the
set's errors.

- **(i) Recommended.** A second trait, `SetWidget`, beside `Widget`: the same `NAME`,
  `PARTS` and `EXTENDS` consts (there is no `ACCEPTS`; decision E's checks replace
  it) and `fn render(&self, props: SetWidgetProps) -> Element`. `SetWidgetProps`
  holds the set's `id`, `path`, `label`, `help_text`, `choices: Vec<Choice>`, which
  choice values are chosen and the item index of each, `on_edit`, `errors`, `classes`
  and the attributes. A trait because the class system keys slots on
  `(W::NAME, Tag)` (`widgets_declare_parts_presets_own_classes.md`) and `form!`'s
  checks read consts, so a trait fits both with no special case. It also gives a dev
  a third way to customize a set, beside `adder:` and `item:`: a tag-chip picker,
  say.
- (ii) Built-in only: an enum inside `SetNode` picking between the default rendering
  and the two new ones. No new dev-facing surface, but `classes_plus:` and `form!`
  would each need a special case for two names that are not widgets.

### Decision H: what a set with a `widget:` still renders

- **(i) Recommended.** The widget replaces the set's whole body: no item list, no
  adder, no Edit or Remove buttons, and no status region. A ticked checkbox and a
  selected option announce their own state, so the set's "Added Paris" message would
  be said twice. So `adder:`, `item:` and `feedback:` beside a `widget:` are compile
  errors (each would do nothing). The fieldset, legend, hint and the set's own error
  list stay, because the widget renders them through the same helpers. `min_items: 1`
  is how a dev says "pick at least one", and the legend already shows the required
  marker for it.
- (ii) Keep the status region for a dev's own `SetWidget`, which might be something
  that does not announce itself.

### The bug this fixes on the way

Today, `cities => { widget: textarea }` on a `BTreeSet<String>` **compiles and then
panics at run time** ("cities is a set, which has no single widget to override",
`build.rs:719`). The same mistake on a `Vec` is a compile error, because
`field_kind::is_single_value` (`field_kind.rs:363`) refuses `Kind::List` but not
`Kind::Set`. With `choices:` added it does fail to compile, but through a trait-bound
error on `choice_field_for`, not a message. Both were seen in a probe on 2026-11-11.
After this plan a `widget:` on a set must be a `SetWidget`, and a leaf `Widget` there
is a `form!` error that says to write `cities[]` to give every item a widget, which is
what the run-time panic says today.

## Part 3: `file`

A `FileInput` widget, `widget: file`, for a field of a new model type. Unlike the
other three, a file cannot travel in the store (strings) or in `WireForm` (JSON), so
this part adds a second way for a form to reach the server. Decision J is that way,
and box 10 is a spike that checks it before anything is built on it.

### Decision J: how a file reaches the server

- **(i) Recommended: a multipart submit.** One request carrying the form's wire values
  as one part and each file as a part named by its `NodePath`. On the client,
  `Form::to_multipart()` instead of `to_wire()`. On the server,
  `WireForm::from_multipart(…).await`, then `Submission::accept` exactly as now. (Since box 11:
  `Submission::accept_multipart`, which reads the parts itself; see 11h.) One
  request and one validation: the file is checked with the rest of the form, by the
  same spec, on both sides. That is the property formoxus is ahead of Django and
  leptos_form on, and a file is the last place to give it up.
- (ii) Upload on pick: the widget calls a dev-supplied server function as soon as a
  file is chosen and stores the reference it returns. formoxus never sees the bytes
  and the wire is unchanged, but files uploaded for a form that is never submitted
  pile up, and the spec does not validate them.
- (iii) Base64 inside the JSON wire. No new transport, but a third larger and the whole
  file in one JSON string. Fine for kilobytes only.

Under (i), the client keeps the chosen file where the store cannot: the store holds the
file's NAME at the field's path, so presence, `required`, the caption and live
validation all work through the ordinary store, and the `FileData` itself sits in a
table on `Form` keyed by `NodePath`. `reset` clears both.

### Decision K: the model type

**Superseded at box 10 and before box 11 (2026-11-12):** files are streamed to a sink,
not held in memory, and `Upload<S>` is the enum of decision U. See box 10's (c) and
box 11's decisions. What follows is the original framing.

- **(i) Recommended.** `formoxus::Upload { name: String, content_type: Option<String>,
  bytes: Vec<u8> }`, held in memory. `Option<Upload>` for an optional upload. In memory
  because a model is a value, and a stream cannot be one; `max_size:` (decision M) is
  what keeps that safe. The build walk has to recognize `Upload` by its shape BEFORE it
  walks it as a struct (trap 10).
- (ii) A dev-defined type behind a trait formoxus provides. More flexible, but a dev
  then writes the multipart reading themselves, which is the part worth having in the
  library.

### Decision L: one file or several

- **(i) Recommended unless apcsp needs more.** One file per field (`Upload` and
  `Option<Upload>`) in this plan; `Vec<Upload>` with `multiple` later. Several files
  for one assignment can still be several fields.
- (ii) `Vec<Upload>` too, now.

**Question for Todd:** what does an apcsp assignment upload look like: one file, or
several?

### Decision M: what is checked

- **(i) Recommended.** `required` (presence, as for every field); `max_size:`, a new
  formoxus setting in bytes, checked on both sides (the client before sending, from
  `FileData::size`; the server on the bytes); and `accept:`, a new row in the
  attribute table (trap 9), validated on the server against the file's extension and
  its claimed content type. Both of those are the browser's claims, so `accept` is a
  usability check and not a security boundary, and the docs say so.
- (ii) `required` and `accept` only, with size left to the server's body limit (trap 12),
  which fails as a 413 and not as a message on the field.

## Decision N: branches

- **(i) Recommended.** One short branch per part (`range`, `multi_pick`, `file`), each
  merged to `main` by Todd when green, so `file` is never held behind review of the
  other two. Nothing here is the kind of change that wants one long branch with the
  compiler errors as the to-do list.
- (ii) One branch `widgets` for the whole plan.

## Traps, checked against the source

1. **A range given `value=""` shows the midpoint while the store holds `""`.** From
   HTML's value sanitization rules, not from formoxus's source; box 3's e2e test
   confirms it in a real browser before decision A's fix is trusted.
2. **`with_owned_attrs` exempts only a checkbox from `required`**
   (`field_attrs.rs:169`), so a required range would get an attribute HTML does not
   allow on it (`attrs.rs:545`). But `Required` there is also the signal every widget
   reads, so it is removed from the range's spread, not from `base_attrs`.
3. **On a `<select multiple>`, `FormEvent::value()` is the FIRST selected option
   only.** dioxus-web 0.7.10 reads `HtmlSelectElement::value()` (`events/form.rs:57`).
   `FormEvent::values()` lists every selected option, each as `(name, Text)`
   (`events/form.rs:95`). Use `values()`.
4. **A `widget:` on a set compiles and panics** (see "The bug this fixes on the way").
5. **`AddItem` cannot take a string** (decision F): `Some` needs the item's shape
   (`items.rs:180`), and `None` on a set commits the draft (`set_node.rs:778`).
6. **Several removals in one change shift positions.** `RemoveItem` takes an index,
   so after the first removal every later index is off by one. Remove the highest
   index first, then add. Matters for `select_multiple`, where one change event can
   deselect several options.
7. **A set's status region would announce what the control already announces**
   (`set_node.rs:937`), which is decision H.
8. **`Option<BTreeSet<_>>` with nothing ticked writes `None`**, because
   `Items::is_present` is false for an empty collection; a plain set writes an empty
   one. Correct, but it needs a test, since it is easy to get backwards.
9. **There is no `accept` row in the attribute table** (`formoxus-spec/src/attrs.rs`),
   and `Multiple`'s row already lists `File` (`attrs.rs:495`).
10. **A struct holding `Vec<u8>` would be walked as a struct with a list of `u8`
    items**: a `ListNode` with one number input per byte. `Upload` has to be
    recognized by the build walk before it is treated as a struct. (Since box 10,
    `Upload` is an enum that holds no bytes, and it would be walked as a
    `VariantNode` instead: the same trap, now at the enum case. See 11d.)
11. **A script cannot set a file input's value, except to `""`.** So SSR, hydration and
    `reset` must all render it empty, and the widget shows the chosen file's name
    beside it, from the store. Re-rendering the `<input>` as a new element would also
    clear the browser's own display of the selection while the table still holds the
    file.
12. **axum's `Multipart` extractor limits a request body to 2 MB by default**
    (dioxus-fullstack 0.7.10 `payloads/multipart.rs`, its docs). A dev raises it with
    `DefaultBodyLimit`; formoxus's docs have to say so, and `max_size:` larger than the
    body limit is a configuration mistake worth a sentence.
13. **The examples app has no server function yet** (`examples/src`). `file`'s e2e test
    needs the first one.
14. **The client side of `MultipartFormData` is built `From<FormEvent>`**
    (`payloads/multipart.rs:156`): from the browser's own FormData for a `<form>`
    element, not from parts formoxus assembles. formoxus does render a real `<form>`
    (`form.rs:409`), and its inputs are named by `NodePath`, so a submit event would
    carry each file under its path. But it would not carry the structure entries
    (variant names, item keys), which are not inputs. Whether the wire values can ride
    along (a hidden input holding the JSON, or parts built by hand) is what box 10
    finds out.

## Boxes

### Part 1: `range` (branch `range`)

- [x] **1.** (Claude) Tests first, failing until boxes 2 and 3. Suite tests for the
  markup: `type="range"`, `min`, `max` and `step` as written, no `required` attribute
  on a required field, the `<output>` showing the value (decision B), and no `<output>`
  with `omit_value`. Tests for the
  starting value (decision A): `empty_form` with `min: 0, max: 10` starts at `5`; with
  `step: 3` at `6` (5 is a step mismatch, and 6 is nearer than 3); with `step: 4` at
  `4`; and `reset` returns there. Box 3's e2e test checks the same three cases against
  a real browser. A wire round trip. Goldens for `range` on
  a `String`, on an `Option<i32>`, and without `min:` or `max:`.

  Done 2026-11-11, on branch `range`:
  - `tests/suite/range.rs`, 13 tests, with its `mod` line in `suite.rs` COMMENTED OUT:
    a `widget: range` the macro does not know yet would stop the whole suite compiling.
    Uncomment it once boxes 2 and 3 are in. The starting-value cases are 0–10 → 5, step
    3 → 6, step 4 → 4, 1–4 → 3 (a tie takes the larger), and a float 0–1 → 0.5 with `step: 0.1`
    and 0.4 with `step: 0.4`.
  - Goldens in `tests/ui-pending/` (the existing home for checks not built yet, no
    `.stderr` until the check exists): `range` on a `String`, on an `Option<i32>`,
    without `min:`, without `max:`, and `omit_value` on a `number`. Run with
    `cargo test -p formoxus --test compile_fail -- --ignored`; today all five fail on
    "unknown widget `range`" or "unknown key `omit_value`", the wrong reason. Box 3 moves
    them to `tests/ui/` and blesses them.
  - **Seam between boxes 2 and 3:** adding `RangeInput`'s row to `WidgetName` breaks
    `formoxus-macros` at once, at the three exhaustive matches in `form/widget.rs`
    (`optional_refusal`, `takes_choices`, `cannot_render`). Those are box 3's, so either
    leave them failing for Claude or add the arms; they are one line each.
  - **Decided (Todd), the float case:** `form!` requires `step:` on a float range. A
    range steps by 1 unless told otherwise, so a float range without `step:` would offer
    only whole numbers ("why use a float if you don't want to allow fractional values?").
    The suite's float test now gives a step, and a sixth golden,
    `form_range_float_without_step.rs`, pins the refusal.

- [x] **2.** (Todd) The runtime. `RangeInput`: a row in `WidgetName`, and its own body
  in `InputView` rather than a `text_like_inputs!` row, because of the `<output>`. The
  `value_when_empty` hook on `Widget` for now (decision A; box 6 moves it to
  `LeafWidget` with the rest of the leaf-only items), called where a `FieldNode` is built
  with no value. `Output` in `HtmlElement` and `Tag`, and the
  `omit_value` prop (decision B). And
  `required` taken out of the range's spread with `.without(Attr::Required)`, as
  `RadioGroup` does, leaving it in `base_attrs` as the signal (trap 2; done by Claude
  2026-11-11, at Todd's request).

  Done 2026-11-11 (Todd; Claude wrote the `required` spread and step 9 at his request):
  `value_when_empty(&self, attrs: &FieldAttrs) -> Option<String>` on `Widget`, forwarded
  through `DynWidget`/`AnyWidget`, asked in `LeafArgs::node` with the attributes the widget
  renders from; the parse factored out of `distribute_values` as `FieldNode::take_raw`, so
  a starting value is read exactly as a typed one. The whole suite passes with
  `range.rs` in, except `omit_value_leaves_the_output_out`, which cannot compile until
  box 3 teaches `form!` the flag. `Tag::Output` changed `form!`'s list of parts, so
  `a_part_that_is_no_element_lists_the_tags` now ends in `output`. Not yet committed.

- [x] **3.** (Claude) The `form!` side: `range` in the three exhaustive message
  matches in `formoxus-macros/src/form/widget.rs` (the compiler lists them), the
  check that `min:` and `max:` are both given (and `step:` on a float), the `omit_value` flag (refused on any
  widget but `range`), and the goldens. An e2e test form
  `/t/range` with its test: an untouched submit sends the midpoint (trap 1), and a
  keyboard change sends the new value. A gallery entry. Docs and memory for the part.
  Then Todd merges `range`.

  Done 2026-11-11:
  - `form!`: `omit_value` is a bare flag, emitted as the widget prop `omit_value: true`
    and refused on any widget but `range`; a `range` without `min:` or `max:` is a parse
    error on the widget's name; a float `range` without `step:` is a `const _` on
    `field_kind::is_float` (new), since only rustc knows the type. `range`'s optional
    refusal now says "use `number`", as the checkbox's says "use `select`".
  - The six goldens moved from `tests/ui-pending/` to `tests/ui/` and were blessed,
    every message read. `range` on a `String` shows five errors, not one: its `min:`
    and `max:` are refused too, and a range cannot be written without them.
  - One more suite test, `a_request_without_the_range_gets_its_midpoint` (14 in all),
    pinning that a request leaving the range out gets the midpoint, not "required".
  - e2e: `/t/range` and `e2e/tests/range.rs` (2), passing in a real browser. The form
    renders bare reference sliders with no value, and the test holds formoxus's
    starting value to what the browser draws on them (trap 1).
  - Gallery: "Sliders" (`examples/src/examples/sliders.rs`). README's widget paragraph,
    `css_class_vocabulary.md` (`fx-range`, `fx-range-value`), `e2e_harness.md`.

### Part 2: `checkbox_multiple` and `select_multiple` (branch `multi_pick`)

- [x] **4.** (Claude) Tests first for decision F, failing until box 5: `ItemValue::raw`
  adds a `String` item and a unit-enum item; a duplicate is refused as an `ItemValue::new`
  one is; an entry that does not parse becomes an `Invalid` item; one outside `choices:`
  is refused on the item; and removing a raw-added item clears its errors and its store
  keys.

  Done 2026-11-11, on branch `multi_pick` (from `main` after `range` merged):
  `tests/suite/raw_items.rs`, 9 tests, its `mod` line in `suite.rs` commented out until
  `ItemValue::raw` exists. Checked by running them once with the typed
  `ItemValue::new(entry.to_string())` standing in: the six on `String` sets pass (keys,
  store layout, errors), and the three only a raw entry can satisfy fail, as they should:
  a `u8` item from `"big"`, and a `Weekday` item from `"Friday"` and from `"Sunday"`.
  `Invalid` and unknown-variant items are asserted only to have SOME error at the
  returned path; the messages are the field's and the variant node's own.

- [x] **5.** (Todd) Decision F's runtime: `ItemValue::raw`, and `Items::add_item` and
  `Items::probe` building an item from an entry. `probe` matters because the duplicate
  check builds the would-be item before adding it.

  Done 2026-11-11 (Todd): `ItemValue` holds a private `ValueType`, `Typed` or `Entry`;
  `shape()` and `peek()` are `Option`s, `None` for an entry; `add_item` and `probe` check
  the shape of a typed value only; `new_named` builds an entry's item blank and hands it
  the entry through `distribute_values` at the item's own path, so a leaf parses it and a
  unit-enum item chooses (or remembers an unknown) variant. All nine `raw_items` tests
  pass; `just ci` 1295. Docs by Claude.

- [ ] **6.** (Todd) Decision G′'s runtime. First the split: `Widget` keeps `NAME`,
  `PARTS` and `EXTENDS`; the rest moves to `LeafWidget: Widget`; the class helpers bound
  on `Widget` and `renders`/`optional_is_okay`/`AnyWidget` on `LeafWidget`. The compiler
  lists every impl to follow, and box 8's goldens follow it too. Then the `SetWidget`
  trait, `SetWidgetProps`, an `AnySetWidget` as `AnyWidget` is for `LeafWidget`, a `widget` field on `SetNode` and the
  render branch that replaces the body (decision H), and
  `FormSpec::with_set_widget(&ModelPath<T>, …)`. `build.rs:719`'s assert stays as the
  backstop for a hand-built spec, now naming `with_set_widget` in its message.

  Progress 2026-11-11: the split (steps 1 and 2) is done and green, `just ci` 1295.
  Todd moved the leaf-only items to `LeafWidget` and re-exported it from `widgets`;
  Claude split the suite's seven impls and the five goldens' (their `.stderr` changed
  only in line numbers), fixed six intra-doc links, and wrote the traits' docs, the
  `widgets` module doc, the README's paragraph and `widget_trait_family.md`.

  Progress 2026-11-11, at home: step 3b (`WidgetCommon`) is done and green, `just ci`
  1295, nothing committed yet. Todd wrote the struct, `Deref`/`DerefMut`, the
  construction sites, `radio_group.rs`'s partial moves (now `take`s, since a read
  through `Deref` borrows ALL of `props`) and the `widgets` export; Claude the suite's
  `ShoutyWidget` (clones) and seven intra-doc links that crossed the split. `common`
  is `pub(crate)`, so outside the crate a moved common field is a clone, never
  `props.common.x`.

  Progress 2026-11-11, at home, later: 3a, 3c-3g done (Todd; `chosen` in
  `SetNode::chosen` by Claude), and 3h's four suite tests pass (`sets.rs`, "A set
  widget, through a hand-built spec"), `just ci` 1305. `SetWidget`/`AnySetWidget` are
  re-exported from `widgets`, and `build.rs`'s leaf-widget-on-a-set panic now names
  `with_set_widget`, pinned by a test. 3h found decision O; 3i builds it.

  Progress 2026-11-11, at home, last: 3i done (Claude, at Todd's request), `just ci`
  1313. `DeclaredOrder` in `set_node.rs`, ranking a node by `own_entry`, which
  `SetNode::chosen` now shares. Found on the way: `form!`'s compile-time order check
  (`field_kind::items_have_default_order`) had the same scalars-only rule, so a unit
  enum set without `sort_by` would not have COMPILED; it now takes a bare unit enum
  too (not through a newtype, matching the run time), and its message, in two goldens,
  names enums without data. Eight suite tests, on an enum declared in one order and
  `Ord`ered in the other so the two cannot be confused; the 3h tests lost their
  `sort_by`.

  **Step 3, written out (Todd asked).** Each piece here is small; the order is the
  order they depend on each other.

  - **3a. The trait.** `pub trait SetWidget: Widget { fn render(&self, props:
    SetWidgetProps) -> Element; }` in `widgets/widget.rs`, beside `LeafWidget`,
    re-exported from `widgets`. No consts of its own: what a set widget accepts
    (decision E) is checked by `const fn`s in `field_kind`, which box 8 writes, not by a
    `FieldTypeSet`. The same rule about hooks as `LeafWidget::render`: a plain method,
    so a widget that reads anything renders a `#[component]` and passes the props on.
  - **3b. `WidgetCommon` first, on its own (Todd, 2026-11-11).** The fields every
    widget's props share, in one struct both props types hold and `Deref` to:
    `pub struct WidgetCommon { id, path, label, help_text, errors, classes }` (the name is
    Todd's, 2026-11-11), with the helpers that read only those: `validity()`,
    `is_invalid()`, `part_class::<W>()`, and a `described_by()` for the
    `aria-describedby` value. `WidgetProps` becomes `{ common: WidgetCommon, base_attrs,
    extra_attrs, values, choices, on_field_event }` plus `impl Deref<Target =
    WidgetCommon>` and `DerefMut`, keeping `current_attrs()`, `control_class()` and
    `field_handle()`, which read its own fields. Through `Deref`, `props.id`,
    `props.path` and `props.validity()` keep compiling in every leaf widget, apcsp's
    `MarkdownWidget` included; what changes is building one (the literal gains
    `common: WidgetCommon { … }`: `FieldNode::render`, `VariantNode`, `WidgetProps::new`,
    the suite's `builtin_widgets.rs`) and MOVING a common field out, which becomes a
    clone or `props.common.x` (two sites, found 2026-11-11: `radio_group.rs:167`'s
    `if let Some(text) = props.label` and the suite's `form_macro.rs:909` `path: props.path`;
    `errors: props.errors` keeps compiling, as an `ErrorsStore` is `Copy`). Do this before `SetWidgetProps` exists and get it green,
    so the refactor and the new type never share a diff.

    **Why a shared struct, and why not embed `WidgetProps` itself.** Todd chose a shared struct
    over repeating six fields: field pass-through is what he wanted from composition,
    and `Deref` gives it. Embedding the whole `WidgetProps` was considered and refused:
    it would hand a set widget a `field_handle()` onto the set's own path, so one call
    could write a structure key into the store (`structure_on_the_wire.md` decision A
    forbids exactly that), plus a leaf's attributes and an `Option` of choices that a
    multi-pick must never see as `None`.
  - **3c. `SetWidgetProps`**, `{ common: WidgetCommon, choices, chosen, required,
    on_edit }` plus the same `Deref` to the `WidgetCommon`, so a set widget reads `props.id`
    and `props.validity()` exactly as a leaf widget does:
    - `common.path` is the SET's own path, which is where its `AddItem` and `RemoveItem`
      are addressed (the one `render` already passes the adder), and `common.id` the
      set's page id, `ctx.id(&self.name)`.
    - `choices: Vec<Choice>`: every value the widget offers (3g says where from).
    - `chosen: Vec<String>`: the ENTRY of each item now in the set, in item order, so
      position `i` is the index a `RemoveItem` takes. A widget finds whether a choice is
      chosen, and where, with `chosen.iter().position(|e| *e == choice.value)`. A
      snapshot, not a store read: a set's items are structure, which the state owns,
      and any edit to the set re-renders the form anyway.
    - `required: bool`, `min_items > 0`, for the legend's marker, as `render` decides
      it today.
    - `on_edit: Callback<Edit>`, as the adder's props have it.
    No attributes yet: `form!` takes none on a set, and box 8 decides whether a
    `select_multiple` may (`size`, `disabled`). And no `FieldHandle`, deliberately.
  - **3d. `AnySetWidget`**, built exactly as `AnyWidget` is: a private `DynSetWidget`
    trait with the blanket impl, a `Box<dyn DynSetWidget>` inside, `Clone`, `Debug` and
    `PartialEq` by hand, `From<W: SetWidget>`. It is what the spec and the node hold.
  - **3e. Where it is written: the spec.** `FieldSpec` gets `set_widget:
    Option<AnySetWidget>`, and `FormSpec::with_set_widget(path: &ModelPath<T>, widget:
    impl Into<AnySetWidget>)` sets it, refusing a path that is not a set the way
    `with_adder` refuses one that is not a list. `build.rs`'s `set_node` copies it onto
    the node, and its assert at `build.rs:~719` ("a set, which has no single widget")
    stays as the backstop for a LEAF widget there, its message now naming
    `with_set_widget`.
  - **3f. Where it renders: `SetNode`.** A field `widget: Option<AnySetWidget>`, and at
    the top of `SetNode::render`, `if let Some(widget) = &self.widget { return
    widget.render(props); }` before the fieldset is built (decision H: the widget
    renders the whole set, its fieldset, legend, hint and errors included, as
    `RadioGroup` renders its own). Nothing else in `SetNode` changes: an `AddItem` or a
    `RemoveItem` from the widget reaches `SetNode::edit` the way the adder's does.
  - **3g. Where `choices` and `chosen` come from: a `Node` method.** The set's DRAFT is
    one blank item, so it already knows what an item can be. Add `fn choices(&self,
    case: LabelCase) -> Option<Vec<Choice>> { None }` to `Node`. A `FieldNode` returns
    its field's `choices()` (`field.rs:55`). A `VariantNode` over a unit enum returns its
    variants, `value` the variant name and `display` the name label-cased, as its
    `<select>` shows it. `SetNode` asks `self.draft.choices(ctx.label_case)`; `None`
    means box 8's checks let through something they should not have, so panic, naming
    the path. Each item's entry for `chosen` is what it sends at its own path:
    `collect_values` it and take the value whose path is the item's.
  - **3h. Trying it before box 8.** `form!` cannot write `widget:` on a set until box 8,
    so check 3a–3g with a hand-built spec and a small test widget through
    `with_set_widget` (it can render `chosen` as text). Box 9's real tests come after
    box 7's real widgets.

  - **3i. Decision O's default order.** `SetOrder::default_for` (`set_node.rs`) gains a
    second case: when the item shape is an enum whose variants are all unit, a new
    `ItemHandler`, say `DeclaredOrder`, holding the variant names in declaration order.
    It compares by each name's position in that list. `sort_peeks` reads a peek's variant
    (`into_enum()`, then its variant index). `sort_nodes`, `dedup_nodes` and `position`
    cannot use `peek_value`, which is `None` for a `VariantNode` (that is why there was no
    default order); they read the item's variant name, the one value it sends at its own
    path, as `SetNode::chosen` does, and an unchosen item (`""`) sorts last, as an
    `Empty` leaf does in `DefaultOrder`. The panic stays for every other non-scalar item.
    Tests: build order, `AddItem` placing an item by declaration order, a duplicate
    refused, a `HashSet` built the same in two processes' iteration orders, a round
    trip; and the step 3h tests drop their `declared_order` `sort_by`.

- [x] **7.** (Todd) `CheckboxMultiple` and `SelectMultiple`. Each change becomes
  `AddItem { value: Some(ItemValue::raw(…)) }` and `RemoveItem`s, highest index first
  (trap 6). The select reads `FormEvent::values()`, never `value()` (trap 3).

  **Written out (Todd asked, 2026-11-11).** Questions P–S, below the steps, are all
  answered, and the steps already follow the answers. Box 6's hand-built-spec tests are the way to try
  each widget before box 8 lets `form!` name it: swap `PropsAsText` for the real one.

  - **7a. Two files, two unit structs.** `widgets/checkbox_multiple.rs` with `pub struct
    CheckboxMultiple;` and `widgets/select_multiple.rs` with `pub struct SelectMultiple;`,
    each `#[derive(Clone, Debug, Default, PartialEq)]` like `RadioGroup`, each `pub mod`
    in `widgets.rs` and re-exported from it beside `RadioGroup` and `Select`. Each
    implements `Widget` and `SetWidget`, and NOT `LeafWidget`: that is what keeps either
    from type-checking as `widget:` on a `String` (decision G′).
  - **7b. `NAME` as a string literal for now:** `const NAME: &'static str =
    "checkbox_multiple";` and `"select_multiple"`. Every other built-in writes
    `WidgetName::X.form_name()`, but both names are still in `WidgetName::NOT_YET`, and
    adding them as `WidgetName` rows is box 8's work, because every exhaustive `match` on
    `WidgetName` in `form!` then needs an arm (the reason `NOT_YET` gives for not being
    variants). Box 8 adds the rows, takes the names out of `NOT_YET`, and switches these
    two consts to `WidgetName::…form_name()`; the existing test that each type's `NAME`
    is its row's `form_name` then covers them.
  - **7c. `PARTS` and `EXTENDS`.** `CheckboxMultiple` is a fieldset of choices exactly as
    `RadioGroup` is, so it takes RadioGroup's parts, `[Fieldset, Legend, Div, Input,
    Label]`, and `EXTENDS = AttrSet::valid_on(HtmlElement::Fieldset)`. `SelectMultiple`
    is one labelled control, as `Select` is (question R), so `[Div, Label, Select]` and
    `EXTENDS = valid_on(HtmlElement::Select)`. No attributes reach either
    yet (step 3c: `form!` takes none on a set), so `EXTENDS` only matters once box 8
    decides on `size` and `disabled`.
  - **7d. Their classes, in both presets.** `classes.rs` keeps one table of every
    built-in's parts (`BUILTINS`, the `parts_of::<W>()` list ending with `RangeInput`), and
    `fx_part` (and the bare preset's equivalent) panics on a `(NAME, Tag)` pair it was
    never told about. So add `parts_of::<CheckboxMultiple>()` and
    `parts_of::<SelectMultiple>()` to `BUILTINS`, and give `fx_part` arms for the
    checkbox group's five parts, modelled on RadioGroup's: `fx-form-field
    fx-checkbox-multiple` with `fx-invalid` on the fieldset, `fx-legend fx-field-label`,
    `fx-choice`, `fx-control fx-checkbox`, `fx-choice-label`. The select's three fall
    through to the generic `Div`/`Label` arms and need only a `Select` arm if it should
    differ from `Select`'s. `parts_of` is bounded on `Widget`, not `LeafWidget`, since the
    split, so it takes a `SetWidget` as it is. Record the new class names in
    `css_class_vocabulary.md`.
  - **7e. `max_items` in `SetWidgetProps`** (question P). Add `pub max_items:
    Option<usize>` beside `required`, filled in `SetNode::render`'s widget branch from
    `self.items.max_items()`, as `required` is from `min_items()`. Fix `PropsAsText` in
    `sets.rs` to print it, and one test that a set with `max_items` hands it over.
  - **7f. `CheckboxMultiple::render`.** As every built-in does, `render` returns a
    `#[component]` (`CheckboxMultipleView { widget_props: props }`) and does the work
    there, so the errors read subscribes that component and not the whole form. The
    view renders, in order:
    - A `<fieldset>` with the `Fieldset` part's class and `aria-describedby` from
      `described_by(&props.id, props.help_text.is_some(), props.is_invalid())` (the free
      function in `widgets/help.rs`, as `SetNode` calls it; the `described_by()` method
      step 3b listed was not added), but **no `aria-invalid` and no `role`**: a bare fieldset is
      role `group`, where ARIA leaves `aria-invalid` undefined, which is why `SetNode`
      leaves it off its own fieldset (`set_node.rs`, the comment at the top of
      `render`). RadioGroup sets `role="radiogroup"` to be allowed it; there is no
      checkbox-group role to do the same.
    - A `<legend>` with the label and, when `props.required`, `required_marker`. This
      is the only place "pick at least one" shows: **no checkbox gets a `required`
      attribute**, because on a checkbox `required` means THAT box must be ticked
      (formoxus's own #6), and HTML has no "at least one of these" for a group.
      `min_items` is held by the set's validation, as it is for every set.
    - `field_help(&props.id, props.help_text.as_deref(), &props.classes)`.
    - One `<div>` per choice, in `props.choices` order, holding `input { type:
      "checkbox", id: "{props.id}-{index}", value: "{choice.value}", checked, onchange }`
      and `label { for: "{props.id}-{index}", "{choice.display}" }`, as RadioGroup's
      radios are built outside the `rsx!` (`radio_group.rs`, the `radios` vec).
      `checked` is `props.chosen.contains(&choice.value)`. **No `name`** (question Q).
      `disabled` is `!checked && props.max_items.is_some_and(|max| props.chosen.len() >= max)`
      (question P): at the limit, only the ticked boxes stay live, so the user can untick
      and pick another, as the Add button does.
    - `onchange`: `e.checked()` says which way it went. Ticked:
      `on_edit.call(Edit::AddItem { path: props.path.clone(), before: None, value:
      Some(ItemValue::raw(choice.value.clone())) })`. Unticked: `Edit::RemoveItem {
      path, index }`, where `index` is `props.chosen.iter().position(|e| *e ==
      choice.value)`, looked up while rendering and moved into the closure. A snapshot
      index is safe because every edit re-renders the form and so rebuilds `chosen`.
    - `FieldErrors { field_id: &props.id, paths: vec![props.path.clone()], errors:
      props.errors, classes }`, the set's own errors only (question S).
  - **7g. `SelectMultiple::render`**, the same way, through `SelectMultipleView`:
    `select { multiple: true, id, class, aria-describedby, aria-invalid, onchange }` with
    one `option { value, selected: props.chosen.contains(&choice.value), display }` per
    choice. Unlike the fieldset, a `<select multiple>` is role `listbox`, which DOES take
    `aria-invalid`, so it gets it, from `props.validity()`, as a leaf control does. No
    `name` (Q). An unselected option is `disabled` at `max_items`, as a box is in 7f
    (P). It is laid out by `field_row`, as `Select` is (R): a `<label for>`, the marker
    when `required`, the caption and the set's own errors in their usual places. `onchange` reads **`e.values()`** (trap 3: `value()` is only the
    first selected option), takes the `Text` of each `(name, FormValue)` pair as the
    selected entries `now`, and diffs them against the snapshot `props.chosen`:
    - every index `i` with `chosen[i]` not in `now`, **highest first** (trap 6:
      removing index 0 first shifts index 2 to 1), each sent as a `RemoveItem`;
    - then every entry in `now` not in `chosen`, each sent as an `AddItem` with
      `ItemValue::raw`. Adds after removes, so no add's index is ever shifted, and an
      add has no index anyway.
    One change event can deselect several options (shift-click, ctrl-click), which is
    the whole reason for the diff. Check what `values()` gives for a select with no
    `name` before relying on it; the pairs' names are not needed, but an empty name must
    not drop the pair.
    **Built 2026-11-11 (Claude, Todd asked).** Checked in `dioxus-web` 0.7.10: on a
    `<select>`, `values()` is one `(select.name(), Text)` pair per selected option, so
    no `name` gives pairs named `""`, none dropped. `required` IS emitted on the select
    (unlike a checkbox): on `<select multiple>` it means at least one selected, which is
    `min_items >= 1`. `field_row` now takes `&WidgetCommon`. The diff is a pure
    `edits_between` with inline tests; the select shares `fx-control fx-select`.
  - **7h. The live-check question, settled by reading, not assumed.** A leaf widget's
    `FieldHandle` reports blur and input to the form for live validation; a set widget
    has no `FieldHandle` (step 3c, deliberately). Each `AddItem`/`RemoveItem` already
    goes through `Form::edit`, which updates the set's errors. Confirm that ticking the
    last box off under `min_items: 1` shows the set's error right away, or that it waits
    for a submit, and write down which; that is the open question box 9 tests.
    **Settled 2026-11-11: it waits.** `Form::edit` drops errors on paths the edit
    removed and checks nothing, and a set widget has no `FieldHandle`. So a tick that
    meets `min_items` leaves a submit's "Add at least 1 item." showing, and unticking
    the last box shows nothing until a submit. Pinned by two `_yet` tests in `sets.rs`
    (`a_ticked_box_leaves_…`, `unticking_the_last_box_…`); box 9 decides.
    **Fixed the same day (Todd: "do it now", Claude wrote it):** `apply_edit` ends with
    `check_item_count`, which after any `AddItem`/`RemoveItem` runs `check_live` on the
    container's path when live validation is on (a tick is a write and a blur at once),
    and only drops a clean result when it is off. One place, so a dev's set widget and a
    list's own Add/Remove get it too. `is_live` is resolved in `use_form` and in
    `Form::edit`. The `_yet` tests became `a_ticked_box_clears_…`,
    `unticking_the_last_box_says_so_at_once`, and a select one; `item_counts.rs` has the
    list side. ci 1331.
  - **7i. Try each through a hand-built spec.** Box 6's `sets.rs` tests, with the real
    widget in place of `PropsAsText`, and a `render_to_html` check of the markup. Box 9
    writes the real suite and e2e tests once box 8 lets `form!` name them.
    **Done 2026-11-11** (ci 1327): markup, no `name`/`aria-invalid`/`role`/`required`
    on the boxes, locking at `max_items`, tick and untick through `Harness`, the
    select's markup and `required`, and one select change swapping several options
    (new `Harness::select`).

  **Answered 2026-11-11: P, Q, R and S all as recommended.** On Q: a native post
  already cannot round-trip a formoxus form (no `action`, `onsubmit` prevents default,
  list keys ride only in `collect_values`), so no `name` loses nothing; progressive
  enhancement, if ever wanted, is its own issue covering lists and variants too. Todd's note on S:
  the only errors these sets should ever show are `min_items` and `max_items`, plus
  one rare case: the choices change between when the client loads them and when the
  submit reaches the server, so an entry that was a valid choice no longer is.

  **Question P: what happens at `max_items`.** `max_items` is checked by validation,
  not refused by `AddItem` (`items.rs`: `over_max` is only for a forged wire), and the
  set's Add button stops at the limit itself (`at_most(item_count, max_items)` in
  `item_buttons.rs`). A checkbox group that lets the user tick past the limit only
  says so after validation. **Recommended:** give `SetWidgetProps` a `max_items:
  Option<usize>`, as `AddItemProps` already has, and have `CheckboxMultiple` disable
  every unticked box once `chosen.len()` reaches it, and `SelectMultiple` every
  unselected option. That is the adder's own behaviour, so a set says "no more" the
  same way whichever way it renders. The alternative is to leave both enabled and let
  the set's "at most N" error show.

  **Question Q: whether the checkboxes and the select carry a `name`.** Every leaf
  control carries `name: "{path}"`, and `VariantSelect` does too (#17, so a test finds
  it by name). Here the only path there is, the set's, is where the wire puts the item
  KEYS (`cities` = `#0,#2`). A native post of the form would send `cities=paris&cities=rome`
  there, which `Items::distribute_values` would read as keys and reject. Nothing in
  formoxus reads a native post today (`distribute_form_values` has no caller), so this is
  harmless now and wrong the day something does. **Recommended: no `name`.** e2e tests
  find a checkbox by role and label (`get_by_role("checkbox", name)`), which is what
  `e2e_harness.md` recommends anyway.

  **Question R: what labels `SelectMultiple`.** It is one control, so like `Select` it
  could be a `<div>` holding a `<label for>` and the `<select>`, built with `field_row`,
  which also gives it the caption, the required marker and the error list in their
  usual places. Or it could be a fieldset and legend like the checkbox group. **Recommended:
  `field_row`**, as `Select`: a `<label>` names one control better than a legend
  does, and a screen reader then reads the label on focus. Decision H's "the fieldset,
  legend, hint and the set's own error list stay" was written with the checkbox group
  in mind; for the select it would become "label, hint and the set's own error list".

  **Question S: where an ITEM's error shows.** Both widgets render the set's own errors.
  An item can still have one: an entry outside `choices` (only from a forged wire, since
  the widgets offer nothing else), or a dev's validator naming one item. The widgets do
  not know item keys (`SetWidgetProps` has entries, not keys), so they cannot point
  `FieldErrors` at `cities.#2`. **Recommended:** leave it for now and say so in box 9's
  docs: a forged entry is refused by validation either way, and a validator that blames
  one pick of a multi-pick can blame the set instead. If it is wanted later,
  `SetWidgetProps` gains each item's key beside its entry.

- [x] **8.** (Claude) The `form!` side: `widget:` on a set resolves to a `SetWidget`
  (the two built-ins by name; a dev's by path, with an `on_unimplemented` message for
  a leaf `Widget` there, which fixes the bug above); `choices:` on the set's line, and
  on the items' line an error that says to move them (decision D); decision E's checks; decision H's refusals of `adder:`, `item:` and
  `feedback:` beside `widget:`; both names out of `NOT_YET`; goldens.

  **Done 2026-11-11 (Claude), ci 1344, e2e 72.** How `form!` knows a set, since it
  sees no types: a built-in's name says its kind (`WidgetName::renders_a_set`), and so
  does `set` beside a dev's widget. A dev's widget on a field without `set` could be
  either, so the field's SHAPE decides, at compile time: `form!` declares
  `const __FX_IS_SET_n: bool = is_set(shape)` and calls
  `field_kind::WidgetSlot::<__FX_IS_SET_n>`, whose two inherent impls take a
  `LeafWidget` (`false`) or a `SetWidget` (`true`). Inherent impls on the two const
  instances, not one trait, because each call then type-checks only against the impl
  picked; two blanket `From` impls would overlap. Its `failure` const returns a
  `slot_failure` number and `form!`'s own `assert!`s report it, so each message keeps
  its old span with no note inside `field_kind`. `FormSpec::__then` (hidden) lets the
  chosen builder sit mid-chain. A set widget's `choices:` go through
  `WidgetSlot::set_choices`, a no-op on a non-set so rustc's "no `iter` on `String`"
  never joins the real message. Decision D's error and the refusal of a widget on
  `key[]` are one cross-entry check in `form.rs` (`check_items_beside_a_widget`):
  only a list or set has items, and a list takes no widget, so a widget on `key`
  beside `key[]` is a set's whatever its kind. Decision H refuses `adder:`, `item:`
  and `feedback:` beside ANY `widget:`, which is right on a leaf too.
  `on_unimplemented` on `LeafWidget` and `SetWidget`. A dev's widget of the wrong kind
  is reported twice (the `failure` const and the builder), both on the widget. Ten new
  goldens (`form_leaf_widget_on_a_set`, `form_set_widget_*`, `form_dev_*_widget_on_*`);
  six suite tests through `form!` at the end of `sets.rs`.

- [x] **9.** (Claude) Suite tests for the two widgets: the markup, ticked and selected
  state from the model, a toggle and a select change as edits, `min_items` and
  `max_items`, an `Option` set with nothing ticked (trap 8), a wire round trip, and
  `classes_plus:`. e2e test forms `/t/checkbox_multiple` and `/t/select_multiple` with
  their tests (`select_option_multiple` in playwright-rs). Gallery entries. Docs and
  memory for the part. Then Todd merges `multi_pick`.

  **Done 2026-11-11 (Claude), ci 1350, e2e 79.** Most of the suite list was already
  covered by 7i and box 8; the rest: trap 8 both ways (an optional set unticked to
  empty is `None`, a plain one is empty), a wire round trip and the server holding
  `min_items` against a forged empty wire, and `classes_plus:` on each widget's parts.
  e2e `tests/set_widgets.rs`, 7 tests, on `/t/checkbox-multiple` (unit enum, 1..=2:
  ticks, unticks, the lock at the limit, `min_items` shown at once on the untick, no
  `required` on any box) and `/t/select-multiple` (two options in one change, a swap
  in one change, the browser's `required` blocking an empty submit); a multi-select is
  role `listbox`. Gallery: "A set ticked from checkboxes" and "A set picked in a
  multi-select" in `examples/sets.rs`, and the old `ChosenDays` lost the `sort_by`
  decision O made unnecessary. README's "what isn't done", the `widgets` module doc,
  `CheckboxMultiple`'s docs, `widget_matrix`'s note; memory: css classes, e2e.

### Part 3: `file` (branch `file`)

- [x] **10.** (Claude) A spike, thrown away afterwards, before any of `file` is built:
  one server function in the examples app taking `MultipartFormData`, one file input,
  and an e2e test driving it with `set_input_files`. It answers trap 14 (can the wire
  values ride in the same request, and how), confirms the server can read a JSON part
  and a file part in either order, and measures what happens past the 2 MB limit (trap
  12). Findings go into this plan, and decision J is revisited if they say so, before
  box 11 starts.

  Done 2026-11-12. The spike is commit `0564e46` on branch `file-spike` (pushed, so it can be read from home), kept
  for reference and never to be merged: `examples/src/spike_upload.rs` (the page at
  `/t/spike-upload` and its server functions) and `e2e/tests/spike_upload.rs` (13
  scenarios, run serially, each printing the server's report). It sends one file two
  ways to server functions that report every part's name, file name, content type,
  size, chunk sizes, and the server process's resident memory (RSS) before and at its
  peak:
  - **(a) "event":** dioxus's own `MultipartFormData::from(FormEvent)`. On the web,
    this is the browser's `new FormData(form)` built from the submitted `<form>`
    element, so it carries every input that has a `name`, hidden inputs included.
  - **(b) "hand-built":** a payload type of the spike's own, `HandBuilt`, that
    implements dioxus's `IntoRequest` on the client and axum's `FromRequest` on the
    server. On the client it builds a `web_sys::FormData` one part at a time, in the
    order the code chooses, and sends it with `ClientRequest::send_js_value`. The
    file comes from a `FileData` kept from the input's `onchange`:
    `FileData::inner()` downcasts to the browser's `web_sys::File`, so the browser
    streams the file from disk itself and wasm never reads its bytes.

  What it found:
  - **Trap 14: yes, the wire values can ride in the same request, by either route.**
    In (a), a hidden input holding the JSON arrives as an ordinary text part. In (b),
    the JSON is appended as a text part. Either way the server reads it back intact.
    Route (a) also sends every other named input as a text part (the spike's visible
    text field came through as one). That is harmless, because the JSON already holds
    everything, but it is a duplicate of every value. The structure entries are only
    in the JSON, as trap 14 expected.
  - **Order: parts arrive in exactly the order they were sent, by both routes.** In
    (a) that order is the DOM's, so the hidden input has to come before the file
    inputs. In (b) it is the order of the `append` calls, so formoxus would decide it.
    The server read both orders without trouble (wire then file, and file then wire).
    Wire first is still the order to send in, because then the server holds the
    form's values, and could validate them, before it touches any file.
  - **Trap 12: past axum's 2 MB default, the request is not refused up front.** The
    handler runs, the parts before the limit are read normally, and the error
    surfaces in the middle of the file part, as `413 Payload Too Large` from
    `Field::chunk()`, after about 1.8 to 2.0 MB of the file have been read. So a
    `from_multipart` has to turn that error into a form error itself; nothing does it
    for it. The limit is raised for one server function alone with
    `#[middleware(DefaultBodyLimit::max(…))]` on it, and that worked: with a 64 MiB
    limit a 20 MB file arrived whole. That is the sentence formoxus's docs need.
  - **Decision K's memory question, measured.** On a 20 MB file, buffering the file
    whole with `Field::bytes()` raised the server's RSS by 21 MB, the file's size.
    Streaming it with `Field::chunk()` and writing each chunk to a temporary file
    raised it by 0.06 to 0.5 MB across four runs. Chunks were at most about 0.9 MB
    in every run where they were logged, 0.5 MB or less in most. (One early run
    whose chunk sizes were not logged stopped at 11.7 MB, which only a single chunk
    of about 10 MB could explain. It did not happen again, and the likeliest cause is
    `dx serve`'s development proxy, which sits in front of the server in these runs
    and would not in production. Not chased further.) Projected to Todd's example of
    150 students uploading 20 MB at once: about 3 GB at the peak if buffered,
    against tens of MB if streamed. That is an extrapolation from one upload at a
    time; concurrent uploads were not measured.
  - **Checking `max_size:` on the server: read the rest of the request, and only
    then answer.** A server function that stopped reading a 20 MB file at 1 MiB and
    answered at once gave the client no answer at all, in three runs out of four:
    `TypeError: Failed to fetch` twice, and once a 500 from `dx serve`'s proxy saying
    it could not write the body. The browser was still sending when the server
    replied and closed the connection. The fourth run's clean answer was luck. A
    version that, once over the limit, kept reading and throwing away the rest of the
    body before it answered gave the client the answer in all three of its runs. So
    the server's `max_size:` check has to drain, and the body limit bounds how much it
    can be made to drain. That also makes the client-side check (from
    `FileData::size`, before anything is sent) the one that gives the user a quick
    message. The server's check is the one that enforces the limit.

  What this means for the decisions, for Todd to answer before box 11 (letters):

  **(a) Decision J stands: a multipart submit.** It works, and it is still one request
  and one validation. The question the spike opens is **which client route.**
  Recommended: **(b), a payload type formoxus owns**, say `formoxus::Multipart`,
  which is `IntoRequest` on the client and `FromRequest` on the server. The reasons:
  1. It needs nothing from the submit event. Today's `onsubmit` in `Form::render`
     drops the `FormEvent`, and button functions never see it. Route (a) would have to
     thread that event through `using_fns!` to the dev's submit function.
  2. formoxus decides the order, so the wire part always comes first, whatever the
     dev's own markup contains.
  3. It sends what the form's file table holds, keyed by `NodePath`, which is how
     decision J already describes the client side. It does not send whatever happens
     to be in the DOM at the moment of the submit.
  4. It sends no duplicate text parts.

  The cost of (b): formoxus calls `ClientRequest::send_js_value`, which dioxus-fullstack
  compiles only under its own `web` feature. It also needs `web-sys` (`FormData`,
  `File`, `Blob`) on wasm, and axum's `Multipart` on the server. A library cannot see
  another crate's features through `cfg`, so formoxus gains features of its own, `web`
  and `server`. Today it has none and relies on Cargo's feature unification instead
  (see the comment on `dioxus-fullstack-core` in `formoxus/Cargo.toml`). A dev would
  write `web = ["dioxus/web", "formoxus/web"]`, the same pattern a Dioxus app already
  uses for its other dependencies. That pattern is the second question:

  **(b) formoxus gets `web` and `server` features?** Recommended: yes. The
  alternative is keying the client code on `target_arch = "wasm32"`, and on the server
  depending on axum directly. That works for a fullstack app today, but it breaks the
  first time dioxus-fullstack's `web` feature is off on a wasm build. It would also put
  axum into every dev's server build, whether that dev uploads anything or not.

  **(c) Decision K, revisited: where a streamed file goes.** The measurements favour
  streaming. Recommended: **formoxus streams each file part to a temporary file,
  counting its size as it goes**, and `Upload` holds `name`, `content_type`, `size`,
  and that temporary file, which is deleted when the `Upload` is dropped unless the dev
  moves it somewhere permanent (`upload.persist_to(path)`, or reads it). This is
  Django's `TemporaryUploadedFile`. The model stays an ordinary value that a server
  function receives, and the dev chooses where the file ends up, after validation
  rather than before it. The alternative is a sink trait the dev implements, which
  streams straight to object storage and skips the temporary file, but it puts the
  dev back to writing the part of multipart handling that is worth having in the
  library. That is the reason K(ii) was turned down in the first place. Still open
  under either answer: what `Upload` is on the CLIENT, where there is no file on the
  server's disk, only a `FileData` in the form's file table. Box 11 has to settle that
  before the model type can be written.

  **Decided (Todd, 2026-11-12):** (c) yes, and the destination should not be only a
  temporary file: a dev should be able to stream a file straight to a regular file
  of their choosing or to an S3 bucket. That suggests a destination trait (call it
  `UploadSink` for now) with formoxus's temporary file as the built-in default, and a
  regular file as a second built-in. S3 would be the dev's own impl, or an example,
  because formoxus should not depend on an AWS SDK. Two things the trait must allow
  for: a file's checks (`max_size:`, `accept:`) fail partway through the stream, so a
  sink needs an `abort` that throws away what it has written (for S3, aborting the
  multipart upload); and a sink writing to the final place stores the file before the
  rest of the form is known to be valid, so a later validation failure has to abort
  it too. (d) yes: Todd, "we have to, if we want to return an error correctly."

  **(a) decided (Todd, 2026-11-12): (b), a payload type formoxus owns.** The comparison
  that settled it: dioxus's `MultipartFormData` can only be built from a submit event
  whose target is the `<form>`, so only the submit button could upload, and the event
  would have to be threaded through `run_button` and `using_fns!`; the wire values
  would have to live in a hidden input, so what is sent is what the DOM held at the
  last render rather than `to_wire()` at the moment of the submit; and the file sent
  is whatever the `<input>` holds, which a re-created element silently empties
  (trap 11). With formoxus's own type the dev writes
  `form.absorb(f(form.to_multipart()).await?)`, the same shape as `to_wire()`.

  **And it must work on desktop and mobile Dioxus too (Todd, 2026-11-12),** because
  nothing formoxus has written so far is web-only, and the native path is small. On
  wasm the client builds a `web_sys::FormData` and sends it with `send_js_value`, as
  the spike did. On native (`not(target_arch = "wasm32")`) it builds a
  `reqwest::multipart::Form` from each `FileData::path()` and sends it with
  `ClientRequest::new_reqwest_request()`, which is what dioxus's own `send_multipart`
  does; neither is behind a dioxus-fullstack feature. Caveat, not yet checked:
  formoxus has never been RUN on desktop or mobile. The code has nothing web-only in
  it (no `web_sys`, `wasm_bindgen` or `target_arch`; its two `document::eval`s are
  supported in Dioxus's webview platforms), but that is an expectation, not a test.
  Box 13 adds a desktop smoke run of an upload form, and checks what `FileData::path()`
  is on mobile, where a picked file may not be an ordinary path.

  **(b), revisited once (a) and the native path were settled: formoxus needs no
  features of its own after all,** which replaces the recommendation above. Checked in
  dioxus-fullstack 0.7.10's manifest and `lib.rs`:
  1. The server half needs axum's `Multipart`. dioxus-fullstack depends on axum with
     its `multipart` feature UNCONDITIONALLY, on every target, and re-exports
     `axum::extract` unconditionally (`pub use axum::{body, extract, response,
     routing}`; only the whole-`axum` re-export is behind `server`). So
     `dioxus_fullstack::extract::Multipart` is there in every build, and the
     spike's web build already compiled against `dioxus::fullstack::extract`.
  2. The native client half needs `ClientRequest::new_reqwest_request()` and
     `reqwest`, both ungated.
  3. Only the wasm client half is gated: `send_js_value` is behind dioxus-fullstack's
     `web` feature, which is nothing but `dep:web-sys`. formoxus can switch that on
     itself in a `[target.'cfg(target_arch = "wasm32")'.dependencies]` entry, with its
     own `web-sys`. That is sound because every wasm Dioxus build is the web platform;
     desktop and mobile are native.

  So the client code is keyed on `target_arch = "wasm32"`, there is no feature for a dev
  to forget to forward, and nothing changes in a dev's `Cargo.toml`. The cost: formoxus
  gains `dioxus-fullstack` as a dependency (it has only `dioxus-fullstack-core` today),
  which brings axum and reqwest into a dev's build even if the app has no server. A
  formoxus form without a server already gets little from `WireForm`, so that dev is
  probably rare; an opt-in `upload` feature is the fallback if one shows up.
  **Decided (Todd, 2026-11-12, at home): as recommended, no features.**

  **(d) The server's `max_size:` check drains past the limit and then answers with a
  form error**, as measured above, rather than stopping. Recommended: yes. Formoxus's
  docs then say to set the body limit a little above the sum of the form's
  `max_size:`s, because the body limit is the most a request can make the server
  drain.

- [ ] **11.** (Todd) The runtime: `FileUpload` (decision K), recognized by the build walk
  before the enum case (trap 10); its leaf, holding the file's name in the store; the
  file table on `Form`; `FileInput` (trap 11); `Form::to_multipart` and
  `Submission::accept_multipart`, per the spike; and the checks in decision M.

  **Decided before box 11 (Todd, 2026-11-12, at home),** on top of box 10's (a)-(d):
  (The "sink" here was named `FileUploadHandler` by decision X below, and `Upload`
  became `FileUpload`.)
  1. **The sink is an argument on the server call, not something `form!` writes.** The
     spec is compiled into the wasm client too, and an S3 sink is server-only code, so a
     sink in the spec would need a `cfg` in every dev's form. One sink handles the whole
     request and is asked once per file, with the field's path, the file's name and its
     content type, so a dev can still send different fields to different places.
  2. **`FileUpload` is generic over where its file went: `FileUpload<S = TemporaryUploadedFile>`.** `S` is
     what the sink produces: formoxus's temporary file, a path, or a dev's own S3 key
     type. The model's type then says where its files are, and the compiler checks it.
  3. **"Keep the current file" is in this plan.** A form built with `form_for` from a
     model that already has its file (a student resubmitting an assignment) shows that
     file's name, and a submit without picking a new one keeps it, as Django's
     `ClearableFileInput` does. Decisions U and V below are what that costs.

  **Decided next, from questions found while writing these steps out (Todd,
  2026-11-12, at home).** The names follow Django where Django has the concept
  (Django's `FileInput` is a bare input; its `ClearableFileInput` adds "Currently:"
  and a Clear checkbox; its `FileField.clean(data, initial)` keeps `initial` when no
  new file came; its `FileUploadHandler` receives a file's chunks as the request is
  parsed; its `TemporaryUploadedFile` is what the temporary-file handler makes).

  **Decision U: the model type is an enum, `FileUpload<S = TemporaryUploadedFile>`.**
  Point 3 is what made it one: a kept file has only its name on the server, so a
  struct would have had every other field optional. Todd's names, which read as a
  file's life in order:
  1. `Chosen { name, content_type, size }`: on the client only, a file chosen and not
     yet sent. It is what `Form::validate()` builds.
  2. `Received { name, content_type, size, stored: S }`: on the server only, a file
     that arrived with THIS request, and the handler put it at `stored`.
  3. `Existing { name }`: the file the record already had when the form was built,
     kept because no new one was chosen. Django's `initial`. **Only the name**, so the
     server's location for it (a path, an S3 key) never goes to the browser in an
     edit form's model; the dev's handler finds the file through its own record.

  `Option<FileUpload>`'s `None` is no file, or one the user removed: Django's `False`
  from `clean`. A `name()` method reads the name from any variant. A server function
  `match`es: `Received` is saved, `Existing` leaves the record alone, and `Chosen`
  can't arrive there (the docs say so).

  **Decision V: the server checks an `Existing` against the record being updated.** An
  `Existing` from the client is only the client's claim, and on a create form it would
  pass a required file with no file at all. So `accept_multipart` takes `existing:
  Option<&T>`, the record being updated, which the server loads anyway (see
  `WireForm`'s note on `initial`); a create form passes `None`. This is Django's rule
  too: its server rebuilds the form with `instance=`, so `initial` is the server's own
  value. An `Existing` at a path where `existing` holds no upload counts as missing: a
  required error, or `None` for an `Option<FileUpload>`. The parameter is required, not
  a builder method, so a server function can't forget it
  ([[feedback-hard-to-misuse]]). **Trap: inside a list item, the item's key (`#3`) is
  not its index in `existing`** once the user has added, removed or moved items, so
  there is no reliable way to find the existing upload. In this plan, an upload inside
  a list or set item can't be kept, and an `Existing` there counts as missing.
  Decision L's one file per field makes this rare anyway.

  **Decision W: one handler per request means one `S` per model.** Every `FileUpload`
  field in a model names the same `S`, the handler's output type. A dev who sends two
  fields to two places writes one handler that routes by path, with an enum as its
  output if the two places need different handles. The alternative, a handler per
  field, would put server code back in the spec (point 1). Todd: "Let's not
  overcomplicate things."

  **Decision X: the handler trait and the one built-in.** The trait is
  `FileUploadHandler`, with Django's method names where Django has the method (11c). The only built-in, for now and
  for the MVP, is `TemporaryFileUploadHandler`, making `TemporaryUploadedFile`s. A
  regular file is a temporary file moved with `move_to` (11b), and S3 is a dev's own
  impl. A `MemoryFileUploadHandler` for small files (say under 200 KB) is a later
  addition, in "Not in this plan". `TemporaryFileUploadHandler::in_dir(path)` puts the
  temporary files in a directory of the dev's choosing (Django's
  `FILE_UPLOAD_TEMP_DIR`). On the same filesystem as the files' final home, `move_to`
  is then a `rename`: instant, atomic, and with no copy.

  **The user's file name: only its final segment is ever kept (Todd).** formoxus cleans
  the name as the part arrives, before any handler sees it, as Django's `UploadedFile`
  does with `os.path.basename`. It splits on both `/` and `\` and keeps the last piece,
  since old browsers on Windows sent whole paths. It also drops control characters,
  and refuses an empty name, `.` or `..` (a form-level error, since only a hand-made
  request sends one). The cleaned name is what `Received` holds and what the store
  shows. The docs for `move_to` and `new_file` still say never to build a path from
  it, because a final segment can still collide with another user's file.

  **Decision Y: `move_to(dest, IfExists) -> io::Result<PathBuf>`** (Claude's
  recommendation, `Version` Todd's addition, 2026-11-12). One method with a required
  parameter and no default, so every call site says which it means
  ([[feedback-one-function-not-two]]):
  1. `IfExists::Fail`: an error if `dest` exists, as Django's `file_move_safe` does by
     default.
  2. `IfExists::Replace`: replaces it atomically, so the old version is there until the
     moment the new one is. On the same filesystem `rename` does that in one step;
     across filesystems the file is copied to a temporary name beside `dest` and then
     renamed over it. This is the resubmission case.
  3. `IfExists::Version`: keeps both, by adding `-2`, `-3`, … before the extension
     (`essay.pdf`, `essay-2.pdf`). Django's `Storage.get_available_name` does the same
     job with a random suffix instead (`essay_a1b2c3d.pdf`).

  It returns the path the file ended at, which is only knowable afterwards under
  `Version`. **Trap: `std::fs::rename` overwrites without asking on Unix**, so `Fail`
  and `Version` can't be "check that it doesn't exist, then rename": two moves at once
  (two students, one name) would both pass the check and one file would be lost. Both
  need a step that fails atomically if the name is taken. On the same filesystem that
  is `hard_link(src, dest)`, which fails with `AlreadyExists`, followed by removing
  `src`. Across filesystems it is a copy into a file opened with
  `OpenOptions::create_new(true)`. `Version` tries `-2`, `-3`, … until one of those
  succeeds. A name with two dots versions at the last one (`a.tar.gz` becomes
  `a.tar-2.gz`), which `Path::file_stem` gives for free and is good enough.

  Todd, on the race: a dev who saves different users' files to one path deserves what
  they get (apcsp puts the username in each path), but be careful anyway. The atomic
  step costs no API, and it also covers ONE user's double-clicked submit: two requests
  from one student, the same path, at once, which a username in the path doesn't
  separate.

  **Decision Z: the code lives in `src/file.rs`** (Todd, 2026-11-12, at home; he had
  first suggested `widgets/file.rs`), as `formoxus::file::…`, with `FileInput`
  re-exported from `widgets` the way `widgets` re-exports `Validity` from
  `field_attrs`. Most of it is not a widget: `FileUpload` is a MODEL type, named in the
  dev's model crate beside its `#[derive(Facet)]`, and `FileUploadHandler`,
  `Multipart<T>` and `accept_multipart` are server and wire code.
  `UploadNode` and `Submission::accept_multipart` can live in the same file: an
  inherent `impl Submission<T>` block can be in any module of the crate, and a leaf
  node at the crate root has a precedent in `field_node.rs`. If the file grows past
  what reads well, it becomes `file.rs` plus `file/` submodules, as `form.rs` is.

  **The steps, in the order they depend on each other.**

  - [x] **11a. Dependencies.** `dioxus-fullstack` joins `formoxus/Cargo.toml` with no
    features, at exactly the version `dioxus` is (as `dioxus-fullstack-core` is pinned
    today, and for the same reason: two copies would mean two `ClientRequest`
    types). Under `[target.'cfg(target_arch = "wasm32")'.dependencies]`, the same crate
    again with its `web` feature, plus `web-sys` with the `FormData`, `File` and `Blob`
    features and `wasm-bindgen` (for `JsCast`). `serde_json` moves from a
    dev-dependency to a dependency, because the wire part is the `WireForm` as JSON.
    The comment above the entry says why there are no features of formoxus's own (box
    10 (b)), so nobody "fixes" it into a feature later.

    Done 2026-11-12, at home (Claude, at Todd's request). `dioxus-fullstack` has
    `default-features = false` in the workspace, which drops its default `ws`
    (websockets) so formoxus doesn't force it on a dev; a dev's own
    `dioxus/fullstack` brings it back if the app uses it. `serde_json` left the
    dev-dependencies, since a dependency already covers the tests. The lock gained
    three dependency edges and no new crates. `just ci` passes (1350), and so do
    `cargo check -p formoxus --target wasm32-unknown-unknown`, `just check-web` and
    `just check-server`.
  - [x] **11b. The model types, in the `file` module** (`formoxus::file::FileUpload`, decision Z; no prelude entries before publishing, per
    [[prelude-is-the-one-list]]). `FileUpload<S>` as decision U settles it, deriving
    `Facet`, `Clone`, `Debug` and `PartialEq`, because every model type needs those four
    (`use_form`, `form_for` and `Submission` all require them). `TemporaryUploadedFile`,
    the default `S`: a temporary file, deleted when the last clone is dropped. So it
    holds an `Arc` of a small guard whose `Drop` removes the file, and it compares by
    `Arc::ptr_eq` (not by path: `x == x` would lock one `Mutex` twice and deadlock). Its methods:
    1. `path()`, where the file is now.
    2. `move_to(dest, IfExists)` (decision Y), which moves the file
       somewhere permanent and disarms the guard. **It takes `&self`, not `self`:** the
       model has to be `Clone`, and `Submission::model()` hands out `&T`. Clones share
       the guard, so after a move no clone deletes the file when dropped, and `path()`
       gives the new location for all of them; the guard's path is behind a `Mutex`
       for that. **Trap:** `rename` fails across filesystems (`EXDEV`), and the
       temporary directory is often a `tmpfs`, so `move_to` falls back to copying,
       flushing to disk and then removing, as Django's `file_move_safe` does.
       `in_dir` (decision X) is how a dev avoids the fallback.

    The guard's field is `#[facet(opaque)]`, since facet has no shape for an `Arc` of
    a guard. No `tempfile` crate: these types exist on wasm too, where it does not
    build, though nothing there ever creates one. **A crash leaks the temporary
    file,** since `Drop` never runs; Django has the same gap. The docs suggest that a
    dev using `in_dir` outside the system's `/tmp` (which the OS cleans) set up a
    cron job to sweep old files from it.
  - [x] **11c. The handler trait, `FileUploadHandler` (decision X).** It has `type Stored`
    (the `S`, with the same four bounds as above, plus `Send + Sync + 'static`).
    (`Stored`, not `Output`, Todd 2026-11-12: it is the handle to where the file was
    put, and it becomes `FileUpload::Received`'s `stored`. The writer's associated
    type is `Stored` too.)
    `new_file(path, name, content_type)` starts a file and returns a writer for it;
    the writer's `write_chunk(chunk)` takes each chunk, `finish()` returns the
    `Stored`, and `abort()` throws away what has been written, for a file whose
    checks failed partway (`max_size:`) or a request that broke off. **The writer's
    names are verbs, not Django's** (Todd and Claude, 2026-11-12). Django's handler
    methods are callbacks for events (`receive_data_chunk`, `file_complete`,
    `upload_interrupted`), told to one stateful handler as things happen. The writer
    has no Django counterpart; it is an object formoxus tells what to do, so it
    takes plain commands, the usual Rust stream vocabulary. `abort`, not
    `interrupt_upload`: by the time it's called the upload has already stopped, and
    what is asked is to discard what was stored (S3's `AbortMultipartUpload`). The
    handler keeps Django's `new_file` and `delete`, which are already verbs and are
    what a dev coming from Django looks for. The handler itself has `delete(output)`, for a file that finished
    storing before a later check failed (Todd's (c)); that is Django's
    `Storage.delete`. For `TemporaryFileUploadHandler` it is a drop; for S3 a
    `DeleteObject`. Every method is async and its future `Send`, because a server
    function's future must be. **Trap: the client chooses the file's name.**
    `new_file` gets it already cut to its final segment (see "The user's file name"
    above), but its docs still say not to build a path from it unchecked.

    **Size (written before Todd started it, 2026-11-12):** about 120-150 lines, in
    four pieces:
    1. The `FileUploadHandler` trait (`type Stored`, `type Writer`, `new_file`,
       `delete`): about 30 lines, mostly docs.
    2. A second trait for the writer, `FileUploadWriter` (`write_chunk`, `finish`,
       `abort`). Django has one stateful handler
       object, but here one handler serves a whole request and each file needs its
       own state (the open file, the count), so a per-file writer is what
       `new_file` returns. About 25 lines.
    3. `TemporaryFileUploadHandler` with `new()` (the system temp dir) and
       `in_dir(dir: impl Into<PathBuf>)`, and its `new_file`, which creates the
       file with `create_new` under a name of formoxus's own (the process id and
       a counter, like `staging_path`), never the user's. About 40 lines.
    4. Its writer: holds the `File` and its path, writes each chunk,
       `finish` flushes and returns `TemporaryUploadedFile::new(path)` (which
       ends 11b's dead-code warnings), and `abort` removes the file.
       About 30 lines.

    Three choices inside it, with the recommendation Todd can just take:
    1. **Methods return `impl Future<Output = …> + Send`, not `async fn`.** An
       `async fn` in a public trait can't promise its future is `Send`, and a
       server function's future must be (rustc's `async_fn_in_trait` lint says
       exactly this). **The built-in's impls return `std::future::ready(result)`**
       (found when Todd built 11c). Two pedantic clippy lints meet here:
       `manual_async_fn` refuses `fn … -> impl Future { async move { … } }`, and
       `unused_async_trait_impl` refuses an `async fn` with no `.await` in it. The
       temporary-file handler never awaits anything, since its I/O is blocking
       `std::fs` (choice 3), so it does the work when called and returns a future
       that is already finished. `Ready<T>` is `Send` when `T` is. A dev's S3
       handler, which does await, writes `async fn` and meets neither lint.
    2. **Errors are `io::Error`.** It's std, and an S3 handler wraps its own with
       `io::Error::other(e)`. A generic error type would ride into
       `accept_multipart`'s signature for little gain.
    3. **A chunk is `&[u8]`, and the built-in writes with blocking `std::fs`.**
       `&[u8]` needs no `bytes` dependency (axum's `Bytes` derefs to it). Writing
       a ~0.5 MB chunk usually lands in the page cache and returns at once.
       `tokio::fs` would add a native-only dependency to save little; revisit only
       if a profile says so.
  - [ ] **11d. The build walk recognizes `FileUpload` (trap 10, corrected).** `FileUpload` is an
    enum now, so without this it would be walked as a `VariantNode` with a `<select>`
    of three variants, not as a struct. In `node_for_shape`, after the `Option` peel
    (so `Option<FileUpload>` works the ordinary way) and before the scalar, newtype and
    collection cases, a shape whose `decl_id` is `FileUpload`'s builds an upload leaf.
    `decl_id` is the same for every `S` (box 10 (c)), where `id` would differ. The same
    test is needed wherever else a walk descends: `field_of` (around `build.rs:160`),
    so that a path into a `FileUpload`'s fields is refused like a path into a newtype's,
    and `field_kind::kind`, the compile-time classifier, which is box 12's.
  - [ ] **11e. The upload leaf, `UploadNode`, a `Node` of its own and not a
    `FieldNode`.** A `FieldNode` parses a scalar from a string, and an upload is not
    one. What it holds: its name; the field's settings (`max_size`, `accept`, label,
    help text, the widget, `optional`, the validator, the same way a `FieldNode` gets
    them through `LeafArgs`); the file's name, which is its value in the store; and
    `meta: Option<UploadMeta>`, the file's `content_type` and `size`, plus, on the
    server, the stored output. Its methods:
    1. `collect_values`/`distribute_values` carry only the name, so presence,
       `required`, `to_wire` and `reset` work through the store as for any field.
    2. **A new `Node` method, `distribute_uploads(prefix, &UploadsByPath)`,** which
       sets `meta` from a table keyed by path. It has a default body that does
       nothing, and the five containers (`StructNode`, `OptionNode`, `ListNode`,
       `SetNode`, `VariantNode`) override it to pass the table to their children.
       Sizes and content types can't travel in the store, which holds one string per
       path, and they are what `max_size:` and `accept:` check.
    3. `validate`: missing (no name) against `required`; then, if there is `meta`,
       `max_size` against its size and `accept` against the name's extension and the
       content type, with HTML's rules for `accept` (`.pdf`, `image/*`, `text/plain`).
    4. `write_value_into` picks the variant. With `meta` and a stored output it is
       `Received`; with `meta` and none, `Chosen`; with a name and no `meta`, `Existing`.
       Writing `stored` is the hard part, because the node can't name `S`: the walk
       has only a shape. So the stored output is kept type-erased, as an
       `Arc<dyn Any + Send + Sync>` together with a function pointer that writes it
       into a `Partial`. `accept_multipart` makes the pointer, because it is generic
       over the handler and so knows `S`. **Trap:** a handler whose `Stored` isn't the
       model's `S` is caught only there, as `Partial::set`'s `WrongShape`. That is a
       dev bug, not a user's doing, so it panics with both type names, the way
       `reject_field` panics.
    5. Populating from a model (`form_for`): the name from whichever variant it is,
       and no `meta`, since a model loaded on the client has no file to hand. So an
       untouched upload in an edit form is `Existing`, which is point 3.
    6. `render` calls its widget like a leaf, with one widget only, `FileInput`
       (11f). A different widget is refused with the same message a `widget:` of the
       wrong kind gets today.
  - [ ] **11f. The file table and `FileInput` (trap 11).** `Form` gains `files:
    Signal<IndexMap<NodePath, FileData>>`, and `FieldHandle` a way to reach it: one
    `pick(Option<FileData>)` that writes the name to the store (`""` for none),
    puts the file in the table or takes it out, and reports the event. **Trap: a pick
    must be checked at once.** A text field waits for blur, but a pick is finished the
    moment it happens, like decision T's Add, and a file input's blur means nothing.
    So it reports a written event and a blur together, or a new event kind. The
    widget renders an `<input type="file">` and never gives it a value (a script can
    only set `""`). Beside it, it shows the chosen file's name from the store, so that
    a re-rendered input (which empties the browser's own display) doesn't make the
    file look lost. For an `Existing` file it reads "Current file: …", in a string that
    belongs in the queued strings move ([[emitted-classes-and-strings]]). For an
    `Option<FileUpload>`, a Remove button, which is `pick(None)`. On `reset`, the table is
    cleared with the store. **Trap: the input still shows the old file after
    `reset`**, because the browser keeps its own selection. A key on the `<input>`
    that `reset` changes (a reset counter on `Form`) makes Dioxus create it again,
    empty, which is the one time that re-creating it is what we want.
    `Form::validate` and the live check (`check_live`) both pass the table, as
    `UploadMeta`s with no stored output, to `distribute_uploads` before they
    validate. Forget it in `check_live` and `max_size:` is never checked live, while
    every test that only submits still passes.
  - [ ] **11g. `Form::to_multipart()`, the client half.** It returns a
    `formoxus::file::Multipart<T>` (named like `WireForm<T>`, which it parallels)
    holding `to_wire()` and the table's files, and `Multipart<T>` implements dioxus's
    `IntoRequest`. The first part is the wire JSON, under a reserved name (say
    `__formoxus_wire`, as a `pub const`), and then each file under its path's string.
    Wire first, so the server has the form's structure and settings before it touches
    a file (box 10). On wasm: a `web_sys::FormData`, each file from
    `FileData::inner()` downcast to a `web_sys::File`, sent by `send_js_value`,
    exactly as in the spike. On native: a `reqwest::multipart::Form` built with
    `.file(key, file_data.path())` and sent through `new_reqwest_request()`, as
    dioxus-fullstack's own `send_multipart` does (`client.rs:197`). **Trap: the table
    can hold stale files.** One picked inside an item since removed, or under a
    variant since unchosen, is still in the table. Send only files whose path is in
    the state's current paths, or drop entries in `apply_edit`. The first is simpler
    and can't be missed by a new kind of edit.

    **Decision AA: the wire part states each file's size** (Todd, 2026-11-12: yes, now). A multipart part
    carries no size, so the server learns one only by reading the whole file. But the
    first part is formoxus's own, so `to_multipart` can list each file's path with its
    `FileData::size` there, while the format is new. Two uses:
    (1) a stated size over `max_size:` is refused before `new_file` is called, so
    nothing is stored, though the part is still drained (box 10 (d)); (2) a later
    `MemoryFileUploadHandler` can choose memory or disk from it, with no format change
    then. The stated size is the client's claim, so the counted size stays the one
    that's enforced: a file that turns out larger than stated fails `max_size:` as
    usual (or, under a memory handler, spills to disk).
  - [ ] **11h. `Multipart<T>` on the server, and `Submission::accept_multipart`.** On the
    server, `Multipart<T>` implements axum's `FromRequest` by wrapping axum's
    `Multipart` and reading nothing yet. Then `Submission::accept_multipart(spec,
    multipart, &handler, existing).await` replaces box 11's earlier
    `WireForm::from_multipart`. The reason: a stream can only be read once, and the
    limits for a file part come from the spec, so the spec has to be in hand before
    the files are read. In order:
    1. The first part has to be the wire JSON. If it isn't, the request didn't come
       from `to_multipart`, and it is a form-level error with nothing stored.
    2. An empty form from the spec; `distribute_values` with the wire's values, which
       builds the structure, so every upload leaf's path now exists.
    3. Each further part: find the upload leaf at its path. A part at no leaf's path,
       or at a leaf whose name in the wire is empty, is drained and ignored, since
       only a hand-made request sends one. Cut the file name to its final segment ("The user's
       file name" above). Refuse a size stated in the wire part (decision AA) that
       is over `max_size:`, and check `accept`, both before calling `new_file`,
       from the file name and the content type, so a wrong type is never stored.
       Stream it in chunks into the writer (`write_chunk`), counting. Past
       `max_size:`, call `abort`, drain the rest of the part and the request (box 10 (d)), and record the
       error. A `413` from `chunk()` (the body limit, trap 12) aborts every open writer and
       becomes a form-level error naming the limit. **Trap, not yet measured:**
       whether the client still gets an answer after a 413. Box 13 tests it in a
       browser.
    4. `distribute_uploads` with each finished file's `UploadMeta`, with its output
       type-erased (11e.4), and decision V's check of each `Existing` against `existing`.
    5. `validate`, exactly as `accept` does. On `Err`, the handler's `delete` takes every output that
       finished storing, then return the errors. On `Ok`, the `Submission` holds a
       model whose files are `Received`.

    The dev's server function is then `async fn submit(form: Multipart<Essay>) ->
    Result<WireForm<Essay>, ServerFnError>`, under
    `#[middleware(DefaultBodyLimit::max(…))]` when its files exceed 2 MB, and the
    client calls `form.absorb(submit(form.to_multipart()).await?)`.
  - [ ] **11i. Green before box 12.** `just ci` passes with no `form!` changes, an upload
    field reachable only through a hand-built spec. Box 13 tests `accept_multipart`
    without a server or a browser. Checked 2026-11-12 in axum 0.8.9: the extractor
    (`extract/multipart.rs:68`) needs only a request whose content type has a
    boundary, wraps the body in `multer` and uses no tokio. So a test builds an
    `http::Request` with a multipart body in memory, extracts `Multipart<T>` from it,
    and runs it under `futures::executor::block_on` (a new dev-dependency; the suite
    has no executor today). The 2 MB default applies there too, so the 413 path is
    testable without a browser as well.

- [ ] **12.** (Claude) The `form!` side: `file` out of `NOT_YET`, the `accept` row in
  the attribute table (trap 9), `max_size:`, and goldens (`file` on a `String`, on a
  non-`FileUpload` struct, `max_size:` on a non-file field).

- [ ] **13.** (Claude) Suite tests (the server-side checks: missing, too large, wrong
  type; a form with a file and ordinary fields, from the multipart into the model), the
  real e2e test with an upload, and a gallery entry. Docs, including the body limit
  (trap 12) and what `accept` does and does not prove. Memory. An apcsp note on what
  its upload form would look like.

- [ ] **14.** (Todd) Merge `file`. apcsp builds its upload form.

## Not in this plan

- **Several files in one field** (`Vec<FileUpload>`), unless decision L says otherwise.
- **A `MemoryFileUploadHandler`** for small files (say under 200 KB; decision X).
  Todd's design: choose memory or disk from the file's STATED size, watch the stream,
  and spill to a temporary file (or refuse) once it passes the limit. A multipart part
  carries no size of its own (browsers send one `Content-Length` for the whole
  request, which is what Django decides by), but formoxus writes the client half, so
  `to_multipart` can state each file's size (`FileData::size`) in the wire part, which
  arrives before every file. That is decision AA in 11g, built now.
- **Keeping an upload inside a list or set item** (decision V's trap): an `Existing` there
  counts as missing.
- **A searchable multi-pick (a combobox).** That is a dev's own `SetWidget` once
  decision G lands, or a later built-in.
- **`datalist`** (`list:` is already an attribute), and **`<optgroup>`** grouping
  inside `select_multiple`.
- **#22 and #23**, which come next on the road to publish.
