# formoxus examples

Not published. This crate exists to be read, run, and — most importantly —
**compiled**: an example that CI never builds has already rotted, it just
doesn't know it yet.

It is a workspace member for that reason, but deliberately *not* a
`default-member`, so a bare `cargo test` in the edit loop skips it:

```bash
cargo test                      # fast loop — library and macros only
cargo test --workspace          # what CI should run — includes this crate
cargo run -p formoxus-examples --bin control_matrix
```

## `control_matrix`

A development tool rather than a teaching example. It prints which
`(value kind, control)` pairs actually render.

That set is not written down anywhere. `form!` accepts every control name in
its table against every field, and whether the pair then works is decided at
runtime by a match in `ScalarInput` whose fallthrough arm panics — so a reader
of the macro's table cannot tell which combinations are real. This prints the
truth, and keeps the root README's "What isn't done" section honest.

**How it detects failure is the interesting part, and the obvious approach is
wrong.** `catch_unwind` reports that every pair passes: Dioxus catches a panic
inside the component's own scope and renders that subtree as nothing, so the
unwind never reaches a caller. What a user actually experiences is the field
*silently vanishing* while its siblings render normally. So the matrix looks
for `name="<path>"` in the output instead.

Two earlier markers were false positives worth knowing about, since they say
something real about the widgets: a `hidden` input deliberately renders bare
with no label at all, and a checkbox puts its text in a plain `<label>` rather
than the `field-label` span every other control uses.

## Still to come

A fullstack example — a real `dx serve` app showing values crossing the wire as
`(path, value)` pairs, rebuilt server-side with `Submission`, with `FormErrors`
routed back onto the fields. That is the part of formoxus that is genuinely
hard to get right from the docs alone, and a single-file example cannot show it.
