# formoxus examples

Not published. This crate exists to be read, run, and — most importantly —
**compiled**: an example that CI never builds has already rotted, it just
doesn't know it yet.

It is a workspace member for that reason, but deliberately *not* a
`default-member`, so a bare `cargo test` in the edit loop skips it:

```bash
cargo test                      # fast loop — library and macros only
cargo test --workspace          # what CI should run — includes this crate
cargo run -p formoxus-examples --bin widget_matrix
```

## `widget_matrix`

A development tool rather than a teaching example. It prints which
`(value kind, widget)` pairs actually render.

`form!` refuses a pair `BuiltinWidget::renders` rules out, at compile time.
Nothing checks it at runtime, so a hand-built `FormSpec` can still reach one.
The matrix builds each pair that way and prints what really happens beside
what `form!` would say: `ok*` marks a pair that renders but `form!` refuses,
and the widgets `form!` names but nothing renders yet are listed after the
table. That keeps the root README's "What isn't done" section honest.

**How it detects failure is the interesting part, and the obvious approach is
wrong.** `catch_unwind` reports that every pair passes: Dioxus catches a panic
inside the component's own scope and renders that subtree as nothing, so the
unwind never reaches a caller. What a user actually experiences is the field
*silently vanishing* while its siblings render normally. So the matrix looks
for `name="<path>"` in the output instead.

Two earlier markers were false positives worth knowing about, since they said
something real about the widgets: a `hidden` input deliberately renders bare
with no label at all, and a checkbox used to put its text in a plain `<label>`
rather than the `fx-field-label` span every other widget uses. The checkbox half
was a genuine inconsistency and was fixed on 2026-09-29. Since 2026-10-11
(issue #9) no widget wraps its control in a label: every field is a
`div.fx-form-field` holding a `label.fx-field-label` with `for`, beside the
control.

## Still to come

A fullstack example — a real `dx serve` app showing values crossing the wire as
`(path, value)` pairs, rebuilt server-side with `Submission`, with `FormErrors`
routed back onto the fields. That is the part of formoxus that is genuinely
hard to get right from the docs alone, and a single-file example cannot show it.
