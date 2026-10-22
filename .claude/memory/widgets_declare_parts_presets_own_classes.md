---
name: widgets-declare-parts-presets-own-classes
description: "DECIDED 2026-10-22 (Todd): a widget declares WHERE classes go (`PARTS: &[Tag]`), a `Classes` preset says WHAT they are; no `default_classes()` on `Widget`; every slot lookup/builder is generic over `W`, never a name literal"
metadata:
  type: project
---

Decided while planning checkpoint 3 of deleting `BuiltinWidget`
(`formoxus/FIELDS_PLAN.md`):

1. **A widget owns only where classes can go**: `const PARTS: &'static [Tag]`.
   `Widget::default_classes()` is removed. Nothing read it at run time; it
   was a second copy of `fx()`'s strings plus a test keeping them in step.
2. **A preset owns what they are.** `Classes::fx()` is one CSS vocabulary
   among possible others (`bare()`, a future Bootstrap), a table keyed by
   `(W::NAME, Tag)`. A preset cannot know a widget it has never seen, so a
   dev's widget is unclassed under `fx()` until the app adds
   `with_part::<MyWidget>(…)`, or it borrows a built-in's slots. A widget
   crate can ship a `fn(Classes) -> Classes` per preset it supports.
3. **`Tag` is its own closed enum** (`Div | Fieldset | Label | Legend | Input
   | Textarea | Select`), not a `&str` and not `HtmlElement`: a part IS an
   element (decided 2026-10-10), but `HtmlElement` carries the input type,
   which matters for attributes and not for classes. Todd: "should we
   really allow devs to put in any string when only a very small set are
   actually allowed?"
4. **No string names at call sites.** `part_class::<Select>(Tag::Select, …)`,
   `with_part::<W>`, `ClassEdit::part::<W>`, `field_row::<W>`: the key is
   built from `W::NAME` inside `Classes`. `Classes::with_parts` (a filter over
   every widget) was deleted.

**How to apply:** don't reintroduce per-widget default classes or
string-keyed slots. Related: [[css-class-vocabulary]], [[attribute-rules-design]].
