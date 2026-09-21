---
name: choice-fields-design
description: "Design for VALUE-choice fields (Select/RadioGroup/SelectMultiple/CheckboxMultiple). TIER 1 (STATIC) IS BUILT as of 2026-09-21: `widget: select { choices: EXPR }` -> FieldSpec.choices -> FormField -> ScalarWidget. Tiers 2 and 3 (render-time and reactive) are still open, and so is the Box<dyn Fn>-cannot-live-on-WidgetType problem they run into"
metadata:
  type: project
---

Answers the fork left open in [[widget-table-and-choice]]: what produces a
*value* choice, as distinct from the shape choice `VariantSet` already handles.
Read that file first for why this is not a missing-match-arm job.

## BUILT 2026-09-21: tier 1, static choices

Shipped, with 275 suite tests and 88 macro tests green:

```rust
const STATES: &[(&str, &str)] = &[("AL", "Alabama"), /* … */];

form! { Address { state => { widget: select { choices: STATES } } } }
```

The pieces, and why each sits where it does:

- **`widget: <name> { … }`** is a new brace block after a widget name, parsed
  into `WidgetArgs`. It is the syntax the HTML attribute keys (`rows`,
  `placeholder`, `class`) will join, so `choices` is the first argument, not a
  special case. Braces stay optional, so nothing written before it changed.
- **`FieldSpec.choices: Option<Vec<SelectChoice>>`**, NOT a payload on
  `WidgetType`. `WidgetType` is the dispatch discriminant and is compared for
  equality across the tree; two selects with different lists are the same *kind*
  of widget. This also keeps `ScalarWidget`'s arms from binding a payload
  dispatch never reads.
- **Gating is a hand-kept `CHOOSERS` list in the macro** (`select`,
  `select_multiple`, `checkbox_multiple`, `radio_group`). Same trade the
  `widgets!` table doc already records: the macro crate cannot see
  `ScalarWidget`'s arms, so being wrong here costs a good error message for a
  pair that would have panicked at render anyway.
- **`SelectChoice` grew `From` impls** for `(&str,&str)`, `&(&str,&str)`,
  `&str`, `String`, `(String,String)`. Not sugar: `SelectChoice` holds `String`s
  so a list of them can never be `const`, and nobody writes fifty states as a
  runtime `Vec`. The borrowed-tuple impl is the one that matters, since
  iterating a slice yields references.
- **`(Text|Int|Float, Select)` now renders** — it panicked before, which is the
  bug Todd hit. A `Bool` still derives `bool_choices()` when no list is given,
  and an explicit list overrides it.
- **A `select` with no choices panics**, naming the field and the fix. Dioxus
  contains the panic, so what a reader sees is the field missing; the test
  asserts absence and says why.

### Still open

- **Tiers 2 and 3** (render-time and reactive choices) are untouched, and the
  `Box<dyn Fn>` problem below is still the wall. `FieldSpec.choices` being a
  plain `Vec` is what a future `ChoiceSource` enum would replace.
- **Nothing validates that a submitted value is IN the list.** A `<select>` can
  only emit its own options, but a hand-crafted POST can send anything, so this
  is a real server-side gap rather than a nicety.
- `SelectMultiple`/`CheckboxMultiple` remain the structural problem below: a
  leaf holds ONE string.

## Todd's proposal (2026-09-19)

Choices apply to four widgets — `Select`, `SelectMultiple`,
`CheckboxMultiple`, `RadioGroup` — and arrive at three different times:

1. **Static** — known when the form is declared.
2. **Render-time** — fetched from the server, or built from a parameter passed
   at the render call site.
3. **Reactive** — changing during the form's life, in response to other fields.

Todd's argument: handle (3) and the other two come free, since nothing changes
choices after render that isn't already case (3). So choices are a *callback*,
with the static case being one that returns a constant.

```rust
pub struct Choice {
    pub display: String,   // what the user reads
    pub raw_value: String, // what the field actually carries
}
```

…and `raw_value` goes through the same string→value conversion every other
field uses.

## Decided, and consistent with what is already built

**The `Choice` shape is right, and the `raw_value` half is already a settled
decision elsewhere.** `SelectInput`'s doc records it: an option's value is the
raw string and *never* an index into `choices`. Indexing is the usual dodge for
a `T` that might not survive a round trip, but `T -> String -> T` is a
guaranteed identity for every builtin scalar here, so an index buys only a
silent `unwrap_or_default()` when it goes stale. Reusing the ordinary parse
path means a choice field needs no new conversion machinery at all.

**Caveat that must be enforced: `raw_value` may never be `""`.** Empty string
IS absence at both boundaries — it is exactly what the `--none--` option emits
and what `apply_leaves` reads as "not supplied". A `Choice` with an empty raw
value is therefore indistinguishable from an unanswered field. Reject it at
construction rather than discovering it as a silently-unset form.

## The collision: a closure cannot live on `WidgetType`

`WidgetType` is `Clone + Debug + PartialEq`, and its `Custom` variant already
uses a **`fn` pointer rather than `Box<dyn Fn>` for exactly this reason** — a
boxed closure supplies none of the three (see
[[facet-newtypes-and-custom-widgets]]). Those derives are load-bearing:
`FieldSpec` and `FormSpec` carry them, and props memoisation needs `PartialEq`.

`Provider<C>` dodges the problem differently — `Rc<dyn Fn() -> Pin<Box<dyn
Future<Output = C>>>>` with a hand-written `Clone` — but it is neither `Debug`
nor `PartialEq`, so it cannot sit inside `WidgetType` either.

**So: whatever carries choices must supply Clone + Debug + PartialEq, or it
must not live on `WidgetType`.** This is the open decision.

### Option A — `fn` pointer, plus `Provider` for the fetched case

```rust
fn(&ValuesByPath) -> Vec<Choice>   // + a `name: &'static str` for Debug
```

A plain fn pointer, exactly like `Custom`'s `render`: `Copy`, clones trivially,
compares by address, and `Debug`s via the name the macro fills in. It cannot
capture — but it *receives* the values store (`ValuesByPath =
Store<HashMap<String, String>>`), so it can read sibling fields and
re-evaluate. That covers (1) and (3) with the derives intact.

It does **not** cover (2): a render-time callback cannot `await`, so a server
fetch has to come from somewhere else. That somewhere already exists —
`Provider` is async and is explicitly "supplied at the render call site rather
than baked into the form's declaration". It is the mechanism this case was
built for; apcsp's `SourcePicker` had `type Choices = Vec<SourcePath>` on the
old derive path for precisely this.

**Where Todd's "(3) subsumes the others" does and does not hold:** it holds for
static, and it holds for fetched *if someone else* does the fetching into a
signal the callback reads. A non-capturing `fn` cannot reach that signal, which
is the whole reason Provider carries case (2) under this option.

### Option B — one capturing closure, in a side-channel

Keep a single `Box<dyn Fn>`/`Rc<dyn Fn>` covering all three, but store it
**keyed by path off to the side** instead of as a field on `WidgetType` — the
move `FormField` already makes with its `wrapper: Option<&'static Shape>`. The
derives survive because the closure never enters the type that needs them.
Simpler for a consumer (one concept, not two); costs a lookup and a second
place where per-field data lives.

**Not decided. Todd's call.**

## Single and multiple are NOT one feature

The four widgets split along a line the proposal does not draw:

- **`Select` and `RadioGroup` pick one value.** These drop straight into the
  existing leaf model — one leaf, one string, parsed as it always was.
- **`SelectMultiple` and `CheckboxMultiple` pick many**, and a leaf holds
  exactly one string. Today a multi-valued field is a `Vec<T>`, which becomes a
  `ListSet` with indexed row paths (`answers.#3.text`) and add/remove buttons —
  a different *shape*, not a different widget. So multi-choice is a structural
  question about how several values live under one path, and it interacts with
  leaf-paths-are-the-wire-format.

**Ship the two single-choice widgets first**; treat multi as its own design
pass.

## A simplification this buys

If choices attach to the **widget** rather than to the type, then a `String`
field with `select` + choices simply works, and so does an `i64` with a radio
group of numeric raw values — the raw string parses as whatever the field
already is. `ValueKind::Choice` and `ValueKind::MultiChoice` then never need
constructing at all, and the right move is to **delete** those two unconstructed
variants rather than fill them in.

It also keeps shape-choice (`VariantSet`/`VariantSelect`, "which variant is
this value?") and value-choice ("which of these candidates?") as separate
mechanisms answering separate questions, which is the conclusion
[[widget-table-and-choice]] was heading toward.

## Watch out for

- `form!` will need `choices:` syntax, and the macro expands in the consuming
  crate — so it can name a `fn` item or capture a local, whichever option wins.
- Reactivity has to happen **inside the leaf's own component**. `FormMember::
  render` is a plain function with no scope; reading the store there subscribes
  whoever called it, and one keystroke re-renders the whole form. This is the
  same constraint that forced one-component-per-leaf ([[facet-form-spike]]).
- `widget_matrix` in the examples crate is the check: implementing
  `Select`/`RadioGroup` for real should move cells from PANIC to ok, and the
  45/84 baseline is what to compare against.
