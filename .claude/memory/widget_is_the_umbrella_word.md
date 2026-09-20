---
name: widget-is-the-umbrella-word
description: "2026-09-20: `widget` is the ONE word for anything that renders a field — reversing the widget→control rename of 2026-09-19 (commit 1cd4069). A Control/Widget split was considered and rejected. Records why, so the reversal doesn't look like churn and doesn't get re-reversed"
metadata:
  type: project
---

## The settled vocabulary

**One word: widget.** `WidgetType`, `WidgetProps`, `ScalarWidget`,
`src/widgets/`, `widget:` in `form!`, `default_widget()`, `custom_widget`,
`with_custom_widget()`. There is no second category. A `<input type=text>` and
a search-backed combobox are the same kind of thing at different sizes.

This REVERSES the widget→control rename of 2026-09-19 (commit 1cd4069). Both
renames are in one day's history; this is the one that stuck.

## Why "widget" and not "control"

The control rename rested on a claim that turned out to be false: that these
components render native HTML controls. **None of them renders a bare element.**
`Input` — the plainest — wraps its `<input>` in a `<label class="form-field">`
with a caption span, a required marker and a `FieldErrors` list. The sole
exception is `type="hidden"`, which renders bare *because there is nothing to
wrap*. "Control" names the native element; every one of these is an assembly
around one.

Two supporting facts, both checked rather than assumed:

- **Django — the library this design's Field/Widget split came from — calls the
  whole range `Widget`.** `TextInput`, `Textarea` and `Select` are all
  `django.forms.Widget` subclasses. Flutter and Qt use "widget" as the umbrella
  too. The control scheme was fighting every major precedent; this one matches
  them. (See [[formoxus-widget-survey]] for the original Django study.)
- **Built-in does not mean simple.** `WidgetType::RadioGroup` and
  `CheckboxMultiple` are enumerated variants, but there is no `<radiogroup>` —
  a radio group is *n* `<input type="radio">` sharing a `name`, plus *n*
  labels. So composites already live among the built-ins.

## The Control/Widget split that was rejected

An intermediate proposal: Control = native HTML (input, textarea, select),
Widget = composites (a combobox with search). Rejected because **the split is a
different axis from the one the code already has.** `WidgetType` splits on *who
supplies it* — eight enumerated variants vs. `Custom { name, render }` — while
Control/Widget splits on *what it's made of*. Those cross both ways: a consumer
can write a `Custom` that renders one plain `<input>`, and `RadioGroup` is a
built-in composite. As sibling categories they cannot partition the vocabulary.

Had the split been kept, the only workable form was **widget as a SUBTYPE of
control**, not a sibling — which preserves `WidgetType`/`widget:` and lets
`RadioGroup` be a built-in composite without contradiction. Worth remembering if
the question ever reopens.

## The word is reused, not inherited

Under the derive path a widget was a **trait** a type implemented —
`FieldWidget`, `DefaultWidget`, `ProvidedWidget`. It now names an **ordinary
Dioxus component** taking `(values, props)`; a custom one implements nothing.
Same word, different mechanism. Anything in history or in
[[widget-registry-idea]] using the trait sense predates
[[derive-path-removal]].

## The one real line, whatever it is called

There IS a behavioral difference worth documenting, it just isn't a naming one:
**a widget wrapping a single native element gets browser validation for free**
(`required`/`pattern`/`min` are enforced by the browser against the element
holding the value), **and a composite does not** — a combobox's visible
`<input>` is not where the value lives. This matters to
[[config-cascade]]'s `use_browser_validation` and will need saying in the docs
for whoever writes the first composite widget.
