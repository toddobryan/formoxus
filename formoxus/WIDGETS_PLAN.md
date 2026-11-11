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
  value in plain sight, instead of a 0–100 that nobody chose for an `i32`.
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
  and a range always has a value anyway. Today `FieldAttrs::with_owned_attrs`
  (`field_attrs.rs:169`) adds `required` to every required field except a checkbox,
  so a range would get it. The fix is to ask the attribute table whether `required`
  is valid on the widget's element, instead of passing `is_checkbox`. That also covers
  the checkbox, which is why the special case can go.

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
  `WireForm::from_multipart(…).await`, then `Submission::accept` exactly as now. One
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
   allow on it (`attrs.rs:545`).
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
    recognized by the build walk before it is treated as a struct.
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

- [ ] **1.** (Claude) Tests first, failing until boxes 2 and 3. Suite tests for the
  markup: `type="range"`, `min`, `max` and `step` as written, no `required` attribute
  on a required field, the `<output>` showing the value (decision B), and no `<output>`
  with `omit_value`. Tests for the
  starting value (decision A): `empty_form` with `min: 0, max: 10` starts at `5`; with
  `step: 3` at `6` (5 is a step mismatch, and 6 is nearer than 3); with `step: 4` at
  `4`; and `reset` returns there. Box 3's e2e test checks the same three cases against
  a real browser. A wire round trip. Goldens for `range` on
  a `String`, on an `Option<i32>`, and without `min:` or `max:`.

- [ ] **2.** (Todd) The runtime. `RangeInput`: a row in `WidgetName`, and its own body
  in `InputView` rather than a `text_like_inputs!` row, because of the `<output>`. The
  `value_when_empty` hook on `Widget` for now (decision A; box 6 moves it to
  `LeafWidget` with the rest of the leaf-only items), called where a `FieldNode` is built
  with no value. `Output` in `HtmlElement` and `Tag`, and the
  `omit_value` prop (decision B). And
  `with_owned_attrs` deciding whether to add `required` by asking the attribute table
  whether `required` is valid on the widget's element, not by `is_checkbox` (trap 2).

- [ ] **3.** (Claude) The `form!` side: `range` in the three exhaustive message
  matches in `formoxus-macros/src/form/widget.rs` (the compiler lists them), the
  check that `min:` and `max:` are both given, the `omit_value` flag (refused on any
  widget but `range`), and the goldens. An e2e test form
  `/t/range` with its test: an untouched submit sends the midpoint (trap 1), and a
  keyboard change sends the new value. A gallery entry. Docs and memory for the part.
  Then Todd merges `range`.

### Part 2: `checkbox_multiple` and `select_multiple` (branch `multi_pick`)

- [ ] **4.** (Claude) Tests first for decision F, failing until box 5: `ItemValue::raw`
  adds a `String` item and a unit-enum item; a duplicate is refused as an `ItemValue::new`
  one is; an entry that does not parse becomes an `Invalid` item; one outside `choices:`
  is refused on the item; and removing a raw-added item clears its errors and its store
  keys.

- [ ] **5.** (Todd) Decision F's runtime: `ItemValue::raw`, and `Items::add_item` and
  `Items::probe` building an item from an entry. `probe` matters because the duplicate
  check builds the would-be item before adding it.

- [ ] **6.** (Todd) Decision G′'s runtime. First the split: `Widget` keeps `NAME`,
  `PARTS` and `EXTENDS`; the rest moves to `LeafWidget: Widget`; the class helpers bound
  on `Widget` and `renders`/`optional_is_okay`/`AnyWidget` on `LeafWidget`. The compiler
  lists every impl to follow, and box 8's goldens follow it too. Then the `SetWidget`
  trait, `SetWidgetProps`, an `AnySetWidget` as `AnyWidget` is for `LeafWidget`, a `widget` field on `SetNode` and the
  render branch that replaces the body (decision H), and
  `FormSpec::with_set_widget(&ModelPath<T>, …)`. `build.rs:719`'s assert stays as the
  backstop for a hand-built spec, now naming `with_set_widget` in its message.

- [ ] **7.** (Todd) `CheckboxMultiple` and `SelectMultiple`. Each change becomes
  `AddItem { value: Some(ItemValue::raw(…)) }` and `RemoveItem`s, highest index first
  (trap 6). The select reads `FormEvent::values()`, never `value()` (trap 3).

- [ ] **8.** (Claude) The `form!` side: `widget:` on a set resolves to a `SetWidget`
  (the two built-ins by name; a dev's by path, with an `on_unimplemented` message for
  a leaf `Widget` there, which fixes the bug above); `choices:` on the set's line, and
  on the items' line an error that says to move them (decision D); decision E's checks; decision H's refusals of `adder:`, `item:` and
  `feedback:` beside `widget:`; both names out of `NOT_YET`; goldens.

- [ ] **9.** (Claude) Suite tests for the two widgets: the markup, ticked and selected
  state from the model, a toggle and a select change as edits, `min_items` and
  `max_items`, an `Option` set with nothing ticked (trap 8), a wire round trip, and
  `classes_plus:`. e2e test forms `/t/checkbox_multiple` and `/t/select_multiple` with
  their tests (`select_option_multiple` in playwright-rs). Gallery entries. Docs and
  memory for the part. Then Todd merges `multi_pick`.

### Part 3: `file` (branch `file`)

- [ ] **10.** (Claude) A spike, thrown away afterwards, before any of `file` is built:
  one server function in the examples app taking `MultipartFormData`, one file input,
  and an e2e test driving it with `set_input_files`. It answers trap 14 (can the wire
  values ride in the same request, and how), confirms the server can read a JSON part
  and a file part in either order, and measures what happens past the 2 MB limit (trap
  12). Findings go into this plan, and decision J is revisited if they say so, before
  box 11 starts.

- [ ] **11.** (Todd) The runtime: `Upload` (decision K), recognized by the build walk
  before the struct case (trap 10); its leaf, holding the file's name in the store; the
  file table on `Form`; `FileInput` (trap 11); `Form::to_multipart` and
  `WireForm::from_multipart`, per the spike; and the checks in decision M.

- [ ] **12.** (Claude) The `form!` side: `file` out of `NOT_YET`, the `accept` row in
  the attribute table (trap 9), `max_size:`, and goldens (`file` on a `String`, on a
  non-`Upload` struct, `max_size:` on a non-file field).

- [ ] **13.** (Claude) Suite tests (the server-side checks: missing, too large, wrong
  type; a form with a file and ordinary fields, from the multipart into the model), the
  real e2e test with an upload, and a gallery entry. Docs, including the body limit
  (trap 12) and what `accept` does and does not prove. Memory. An apcsp note on what
  its upload form would look like.

- [ ] **14.** (Todd) Merge `file`. apcsp builds its upload form.

## Not in this plan

- **Several files in one field** (`Vec<Upload>`), unless decision L says otherwise.
- **Streaming uploads** past what fits in memory: decision K keeps the bytes in the
  model. A dev who needs gigabytes needs (ii) of decision J, which they can build
  beside formoxus today.
- **A searchable multi-pick (a combobox).** That is a dev's own `SetWidget` once
  decision G lands, or a later built-in.
- **`datalist`** (`list:` is already an attribute), and **`<optgroup>`** grouping
  inside `select_multiple`.
- **#22 and #23**, which come next on the road to publish.
