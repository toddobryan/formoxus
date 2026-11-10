# Live paths: `NodePath` (#21), and bad item keys on an optional list (#20)

Written 2026-11-09 by Claude, after Todd asked for #20 and #21 in one plan. Next
on the road to publish (`.claude/memory/mvp_scope.md`): this, then the four
missing widgets, then #22 and #23, which take this plan's type.

Who does what: Todd writes the runtime; Claude writes the tests, the e2e and
example follow-ups, the docs, the memory, and the apcsp recount. No macro work:
`form!` emits spec paths and `path!` emits `ModelPath`s, and neither changes.

**Two parts, landing separately.** #20 is a one-line bug fix that breaks nothing,
so it goes on `main` first (box 1–2). #21 is a breaking change to every type that
carries a live path, so it is one radical change on a branch `paths` (boxes 3–8),
with the compiler errors as the to-do list.

## Why

1. **#20: a forged or stale request can lose an optional list without a word.**
   Garbage item keys, or more keys than `max_items:`, on an `Option<Vec<_>>` or
   `Option<BTreeSet<_>>` field are accepted as `None`. The same field without the
   `Option` is refused. `structure_on_the_wire.md` decision C says bad input is
   remembered and reported; here it is dropped.
2. **#21: a live path is a bare string, so nothing stops it being the wrong
   string.** `answers.#5.text` and `footprint.$Circle.size` are keys of four
   maps, an element's `name`, every `Node` method's `prefix`, and what crosses the
   wire. Nearly every bug found building lists, sets and `ErrorsStore` was a path
   built wrong: a container passing its own path where a prefix belonged, or a
   segment added twice. None of them failed to compile. And the rules for
   building and reading these strings live in nine free functions that any string
   can be handed to, in either argument order.

## #20: what is wrong, exactly

`Items::distribute_values` (`nodes/items.rs:83`) records a bad entry in
`rejected_keys` (the keys did not parse) or `over_max` (more keys than `max_items`,
refused before any item is built), and builds no items. `Items::is_present`
(`:154`) then answers false, because the collection is empty.
`OptionNode::validate` (`option_node.rs:47`) runs its inner node's `validate` only
when `is_present()`, and `Items::validate` is the only place either field becomes
an error. So the error is never made, and `write_value_into` writes `None`.

`VariantNode` hit the same thing and solved it the same way the fix here does:
`VariantChoice::Unknown` counts as present (`variant_node.rs:470`), "so
`OptionNode` validates it rather than skipping it, which would let a bad name pass
as `None`".

**The fix:** `Items::is_present` is also true while `rejected_keys` or `over_max`
is set. `ListNode` and `SetNode` both ask `Items`, so one change covers both.

## #21: the design

**One owned newtype, `NodePath`, for live paths only** (names and placement are
decisions A and B below). Model paths stay `ModelPath<T>`, and spec paths
(`answers[].text`, what `FormSpec`'s builders take) become `ModelPath<T>` too
(decision H, 2026-11-10), so a path is one of exactly two kinds: a place in the
model's TYPE, written by a dev and checked at compile time, or one live node,
built at run time. `SpecsByPath` keeps its `String` keys.

It wraps the same `String` it replaces, so the wire spelling, the `name`
attribute and every map's iteration order stay exactly as they are. What changes is
who can make one and what you can do with one.

**What it carries over from today's free functions:**

| Today | On `NodePath` | Where today |
|---|---|---|
| `""` as the root prefix | a `root()` constructor | everywhere a top-level call passes `""` |
| `qualify(prefix, name)`, 68 uses | `prefix.join(name)` | `nodes.rs:225` |
| `item_segment(key)`, 12 uses | `prefix.join_item(key)` | `nodes/items.rs:522` |
| `variant_segment(v)`, 7 uses | `prefix.join_variant(v)` | `nodes.rs:200` |
| `owns(nested, path)`, 39 uses | `path.starts_with(&nested)` (decision J) | `nodes.rs:158` |
| `ensure_owned(c, p)`, 8 uses | a method returning the same `Result` | `nodes.rs:170` |
| `path_without_variants(p)` (pub) | `path.without_variants()` | `nodes.rs:217` |
| `spec_key(p)` | `path.spec_key()`, still returning a `String` | `build.rs:71` |
| `resolve_indices(p, entry)` | still a function of a `ModelPath`'s string, but returning `Option<NodePath>` | `path.rs:212` |
| `keyed_or_as_written(p, keys)` | goes, or changes with decision F | `form/state.rs:562` |

**What changes type** (one breaking change, so all of it at once):

1. The map aliases: `ValuesByPath`, `ValuesStore`, `ErrorsByPath`, `ErrorsStore`
   (`nodes.rs:400–416`, `form.rs:46`) are keyed by `NodePath`.
2. The `Node` trait (public, `nodes.rs:46`): `prefix: &str` becomes
   `prefix: &NodePath` in `collect_values`, `distribute_values`, `validate`,
   `paths`, `edit` and `absent_owner`; `paths`' out-param becomes
   `Vec<NodePath>`; `absent_owner` returns `Option<NodePath>`.
3. `RawValue`'s three `path` fields (`nodes.rs:249`), and `into_pair`.
4. `Edit`'s path fields and `Edit::new_choose_variant` (`nodes.rs:598`, `:662`).
5. Public struct fields: `NodeContext.prefix`, `WidgetProps.path`,
   `AddItemProps.path` (`widgets/item_buttons.rs:34`), `SetItemProps.path`
   (`widgets/set_item.rs:28`).
6. `changed_paths: Signal<HashSet<String>>` (`form.rs:257`, `:925`, `:1008`),
   `FormState::item_keys`' keys, `FormErrors::paths()`, `FormAccessError`'s
   messages only in that they format one.
7. `FormState::push_field_error(&str, …)`: decision F.

**What does not change:** the HTML `id`s (`WidgetProps.id`, `NodeContext::id`),
which are derived from a path but are not one; a `RawValue::Structure`'s `data`
(`"#0,#5"`), which is a value, not a path; spec paths; `ModelPath<T>`.

## Decisions (Todd, 2026-11-09): all seven as recommended

Each letter says what was recommended and why; Todd took every one.

A. **The name: `NodePath`. DECIDED.** every live path names a node, and
   `Node`, `NodeContext` and the `nodes` module are already the vocabulary.
   `Path` puts back the collision with `std::path::Path` that renaming `ModelPath`
   on 2026-11-08 cleared. `FieldPath` is wrong for containers, whose paths are
   live paths too (`answers`, `footprint`).

B. **Where it lives: `formoxus::path::NodePath`, beside `ModelPath`. DECIDED.** Not in the prelude or `formoxus::widgets`, per 2026-11-09's "no
   new re-exports until publish"; a custom widget reads `props.path` without
   naming the type, and formats it with `{}`.

C. **Owned only, not an owned/borrowed pair like `PathBuf`/`Path`. DECIDED.**
   Methods take `&NodePath`. A borrowed `&NodePathStr` would
   need an unsized newtype over `str`, which means `unsafe` or the `ref-cast`
   crate, to save allocations that `qualify` already makes on every call today.
   It can be added later without breaking callers that take `&NodePath`.

D. **How much the segments know: `join`, `join_item`, `join_variant`. DECIDED.**
   Todd does not love the name `join`, but `std::path::Path::join` makes it
   the name a Rust reader expects, so it stays. `join(name: &str)` for a node's
   own name, plus `join_item(key: usize)` and `join_variant(name: &str)`, so the
   `#` and `$` sigils are written in one place. A full segment enum
   (`Field | Item | Variant`) inside the type can come later without its callers
   changing, which is #21's point 4. Not now: nothing needs to read a segment's
   kind except `spec_key`, `without_variants` and `default_label`, which can check
   the sigil as they do today.

E. **How a string becomes a `NodePath` from outside the crate: `NodePath::new`,
   no `From`. DECIDED.** A server
   reading a raw form post, and the suite (350 dotted string literals across 23
   files), have to make them. One explicit constructor,
   `NodePath::new(impl Into<String>)`, documented as "for a path off the wire, or
   in a test", and **no `From<&str>` or `From<String>`**, so a bare string never
   converts silently at a call site. `Deserialize` makes them for `WireForm` and
   `FormErrors`. A lookup needs no constructor: `Borrow<str>` lets
   `errors.fields.get("answers")` keep working, which most of the suite's
   literals are, and apcsp's `server.rs` too.

F. **`FormState::push_field_error` takes `&ModelPath<T>`. DECIDED** ("we should
   have done that a while ago"). It takes either kind of path today:
   `form/state.rs:381` takes a `&str` that may be written like a model path
   (`answers[2]`, `tags[=rust]`) or like a live one (`answers.#5`); `key_for` and
   `keyed_or_as_written` turn the first kind into the second at runtime, and pass
   the second through unchanged. It will take `&ModelPath<T>`, as
   `Form::push_field_error` already does (`form.rs:541`; `FormState` is generic
   over `T`, so it can), and the live-string form goes. `Form::push_field_error`'s
   doc says strings are for "a specific item (`venues.#0.city`)" that "a `path!`
   cannot spell", but `path!(M.venues[i])` has spelled one since 2026-11-03, so
   that reason is gone (the doc is stale either way). The alternative, not
   taken, was a second method taking a `&NodePath`, for a caller that already
   holds a live one; nothing outside the crate does today.

G. **#20 on `main` first, before the branch. DECIDED.** It is one line,
   breaks nothing, and the branch would otherwise carry a bug fix through a long
   refactor.

## Decisions (Todd, 2026-11-10): three more, from box 4's review

Todd asked whether spec paths and live paths could be one type. They cannot
without bringing back #21's bug class: a live path names ONE node and holds an
item key (`answers.#5`), while a spec path names a place in the type and holds a
pattern (`answers[].text`). Spec paths and model paths, though, are already the
same thing: `path!(Quiz.answers[].text)` yields `"answers[].text"`, through the
same parser that turns `form!`'s paths into keys.

H. **Spec paths become `ModelPath<T>`. DECIDED.** Every `FormSpec` builder
   that takes a path (`with_widget`, `with_label`, `with_min_items`,
   `treat_as_set`, `with_attrs` and the rest, `form/spec.rs:156–311`) takes
   `&ModelPath<T>` in place of `&str`. `FormSpec` is already generic over `T`,
   so a path for another model no longer compiles, and a typo is caught by rustc
   instead of by `shape_at`'s panic. `form!` emits `&ModelPath::__new("…")` keys
   (the string it emits today, wrapped). `SpecsByPath` keeps its `String` keys,
   since nothing outside `FormSpec` builds one. The cost is the hand-written
   builder calls in the suite and the examples, about 86, which become
   `&path!(M.x)`.

I. **A builder given an instance path panics. DECIDED.** `path!` can spell
   `answers[2]` and `tags[= rust]`, which name ONE item; a spec names every item
   at once (`answers[]`). The builder panics with a message naming the path and
   saying to write `[]`, the same kind of panic `shape_at` gives a path that
   names nothing. Not taken: a separate pattern-only type from `path!`, which
   would rule it out at compile time but split a macro devs already know in two,
   when `form!`, the main way to build a spec, never emits an index.

J. **`owns` becomes `starts_with`, called on the inner path, true on the root.
   DECIDED.** It matches `std::path::Path::starts_with` (checked 2026-11-10):
   the receiver is the longer path, `path.starts_with(&nested)`; only whole
   segments match, so `address2` does not start with `address`; and an empty
   base is a prefix of every path, so `starts_with(&NodePath::root())` is true
   everywhere. That settles trap 3 below. `is_within`, the name this plan used
   before, goes. The one thing to watch is that `p.as_str().starts_with(…)` is
   `str`'s method and compares characters, the same split `Path` and `str` have.

## Traps, checked against the source

1. **`dioxus-stores` takes a lookup key by value, and it must be `'static`.**
   `get`, `get_unchecked` (`dioxus-stores-0.7.10`, `impls/hashmap.rs:261`,
   `:285`) take `key: Q` where `K: Borrow<Q>`, so `Borrow<str>` does not help a
   store lookup: `&str` is not `'static` in general. Store reads clone the
   `NodePath` into the call, which costs what cloning the `String` costs today.
   `contains_key` and `remove` take `&Q`, so those can take `&str`.
2. **`Borrow<str>` requires a hash equal to `str`'s.** Deriving `Hash` on a
   struct with ONE `String` field gives that, since `String` hashes as its `str`.
   Add a second field and the derive breaks `Borrow` silently: lookups by `&str`
   miss. Implement `Hash` by hand over the string, or test it (box 3).
3. **`owns("", path)` is false today for every path except `""`.**
   `"x".strip_prefix("")` is `"x"`, which does not start with `.`. Nothing seems to
   rely on it, since every container computes its own path with `qualify` before
   calling `owns`, but `starts_with(&root)` should be true for every path
   (decision J, as `std::path::Path` does), and the change should be deliberate. Grep for an `owns` whose first argument can be a
   prefix, not a node's own path.
4. **`OptionNode` adds no segment.** It hands its `prefix` to the inner node
   unchanged (`option_node.rs:77`), so the inner node computes the same path. With
   `join` a method, it is tempting to add one there; that is the bug the comment
   warns about.

   Todd asked (2026-11-09): don't these paths mark an `Option` with `?`? No. The
   `?` exists only in source, in `form!` and `path!`, where it tells the macro to
   go through the `Option` when it builds the witness. Every string form of a path
   drops it: a `ModelPath`'s string, a spec key, and a live path alike.
   `path!(Contact.address?.city)` is `"address.city"`, pinned by
   `path_takes_a_question_mark_and_keys_without_it` (`tests/suite/form_macro.rs`),
   and an optional struct's leaves are named `address.street`, never
   `address.some.street` (`tests/suite/optional_containers.rs:99`). That is why
   the `Option` must stay invisible here: a `?` segment in a live path would
   change every `name` attribute and the wire.
5. **`join` must refuse a name containing a `.`.** `Node::name()` is still a
   `String` (`"title"`, `"#3"`), and a name with a dot would forge two segments.
   A `debug_assert!` is enough; names come from Rust field names and
   `item_segment`.
6. **A `NodePath` must never contain `[`.** Model paths do (`answers[2]`), and
   `resolve_indices` is the only bridge from one to the other. Today
   `keyed_or_as_written` returns a model path's string unchanged when it does not
   resolve, so the error message names it. Under F that string stays a
   `ModelPath`'s, and the message is formatted from it, not from a `NodePath` made
   out of it.
7. **The wire stays byte-for-byte the same.** `#[serde(transparent)]` on the
   type. `WireForm` and `FormErrors` round-trip tests (`tests/suite/wire.rs`)
   are what prove it, so run them before changing anything else in them.
8. **`rsx!` attributes.** `name: "{props.path}"` works through `Display`. A
   `name: props.path.clone()` would need `IntoAttributeValue`; grep the widgets
   and the gallery for that form and change them to the formatted one rather than
   implementing the trait.

## Boxes

- [x] **1.** (Claude) #20's tests, failing first, on `main`. In
  `tests/suite/item_counts.rs`, beside `the_server_refuses_too_many_items_unbuilt`:
  the issue's four rows for an `Option<Vec<String>>` and an
  `Option<BTreeSet<String>>` with `max_items: 2`, through `Submission::accept`,
  each refused with the message the non-optional field gets. Plus three edges:

  1. Nothing sent for the optional list is still `None`, with no error.
  2. An empty key list (`""`) is still `None`, with no error: `parse_item_keys`
     reads it as no items, which is neither rejected nor over the max.
  3. An `Option<Struct>` holding a list with garbage keys is now refused, not
     `None`. `StructNode::is_present` (`struct_node.rs:121`) is "any child is
     present", so the fix reaches it too, which is what decision C wants; pinned
     so that it is on purpose.

  Done 2026-11-09, in `tests/suite/submissions.rs` rather than `item_counts.rs`:
  it already holds the non-optional field's malformed-keys test and the `wire`
  helper, and half the cases are not about counts. A new section at the end, "Optional
  lists and sets: a bad entry is not `None` (#20)", with seven tests: the four rows,
  the struct case (all five FAIL today, each accepted), and the two `None` edges
  (both pass today, and must still pass). Checked against the issue's fix applied
  in place and then reverted: with it, the whole suite is 751/751.

- [x] **2.** (Todd) #20's fix, on `main`: `Items::is_present` (`nodes/items.rs:154`)
  is also true while `rejected_keys` or `over_max` is set, with a comment saying
  why, like `VariantNode`'s. `write_value_into` is never reached for such a node,
  since `validate` fails first. Then box 1 goes green, `just ci`, commit, and #20
  closes.

  Done 2026-11-09: Todd's fix, `just ci` green, committed by Claude, #20 closed.

- [x] **3.** (Todd) Branch `paths`. The type and its methods, per A–E, in
  `path.rs` beside `ModelPath`, with nothing using it yet. (Claude) its inline
  tests (`#[cfg(test)] mod tests`, which is the right home for a pure type),
  ported from whatever tests `qualify`, `owns`, `path_without_variants` and
  `spec_key` have now, plus: `join` on the root gives no leading dot; `starts_with`
  refuses a mere string prefix (`address2` in `address`) and, per trap 3, is true
  of the root; a `&str` lookup in a `HashMap<NodePath, _>` finds what a
  `NodePath` key inserted (trap 2); and serde round-trips as a plain string.

  Done 2026-11-10: 21 tests in `path.rs`, plus `replace_prefix` (added for
  `SetNode::edit_item`) and `without_variants`, ported from the suite's
  `path_without_variants` test. Both `starts_with` bugs found in review (the root,
  and a mere string prefix) were put back by hand and each fails tests.

- [x] **4.** (Todd) The radical change: every item under "What changes type"
  above, in one go, deleting the free functions as their last callers go. The
  compiler's errors are the list. `cargo check -p formoxus` green is the end of
  this box; the suite will not compile yet.

  Done 2026-11-10: `cargo check`, `clippy -D warnings` and the 198 inline tests
  green. `owns` became `starts_with` (decision J), `ensure_owned` became
  `ensure_starts_with`, `qualify` and `variant_segment` went (`NodeContext`
  gained `nested_variant`), `item_segment` moved to `path.rs`, and
  `path_without_variants` became `NodePath::without_variants`. `Node::name`
  stays a `String`: it is one segment, not a path.

- [ ] **5.** (Todd) Decision F, and the model-to-live bridge: `resolve_indices`
  returns `Option<NodePath>`, `key_for` and `keyed_or_as_written` change or go,
  and `Form::push_field_error`'s stale doc goes with them.

  Mostly done inside box 4, 2026-11-10: `ModelPath::resolve` (taking an `entry`
  over `&NodePath`), `key_for` and `keyed_or_as_written` over `&ModelPath<T>`,
  decision F, and `to_node_path` gone. Left: `Form::push_field_error`'s doc.

- [ ] **6.** (Todd) Decisions H and I: every `FormSpec` builder that takes a path
  takes `&ModelPath<T>`, stores `path.as_str()` as the `SpecsByPath` key, and
  panics on an instance path (`[2]`, `[= v]`) with a message that says to write
  `[]`. `shape_at` stays, since a `ModelPath` checks the model's fields but
  `spec_key_shape` is what checks a key against the node tree. `cargo check -p
  formoxus` green again is the end of this box.

- [ ] **7.** (Claude) `form!` follows: every builder call it emits wraps its key
  in `&::formoxus::path::ModelPath::__new("…")` (full module path, per the prelude
  rule). Regenerate the trybuild goldens with `TRYBUILD=overwrite` and read the
  diff: a span that moves is expected, a message that changes is not.

- [ ] **8.** (Claude) The suite, the examples and the gallery follow: map lookups
  by `&str` stay, inserts take `NodePath::new`, and any custom widget or adder in
  the suite that builds a path by `format!` uses the methods instead (that is the
  point, so it is not a mechanical rename). The hand-written builder calls (about
  86) become `&path!(M.x)`, and two new tests pin decision I's panic, one for `[i]`
  and one for `[= v]`. Then `just ci` and `just e2e`. The e2e selectors read
  `name` attributes, which do not change, so a failure there is a real one.

- [ ] **9.** (Claude) Docs and memory: `README.md` and `widgets.rs` wherever a
  live path is called a string; the trait docs on `Node`; the memory files that
  name the free functions (`structure_on_the_wire.md`, `indexed_paths.md`,
  `optional_flag_is_payload_only.md` at least); and a comment on #22 saying its
  view takes `NodePath`. Also the apcsp recount, read only, as for ERRORS_PLAN box
  10: what in apcsp names `ValuesByPath`, `ErrorsByPath`, `RawValue`, `Node`, or a
  `props.path` as a `String`.

- [ ] **10.** (Todd) Merge `paths` to `main`; #21 closes; apcsp follows.

## Not in this plan

1. ~~**Spec paths as a type.**~~ Taken into the plan 2026-11-10 as decision H:
   they are `ModelPath<T>`, not a third kind.
2. **#22 (a read-only store view) and #23 (`BoundField`).** After the missing
   widgets. They take `NodePath`, which is why this goes first.
3. **A structured key** (segments stored parsed, strings only at the wire). The
   newtype is the step that makes it possible later without breaking callers;
   decision D is where it would start.
