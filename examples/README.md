# formoxus examples

Not published. This crate exists to be read, run, and — most importantly —
**compiled**: an example that CI never builds has already rotted, it just
doesn't know it yet.

It is a workspace member for that reason, but deliberately *not* a
`default-member`, so a bare `cargo test` in the edit loop skips it:

```bash
cargo test                      # fast loop — library and macros only
cargo test --workspace          # what CI should run — includes this crate
```

## Still to come

A fullstack example — a real `dx serve` app showing values crossing the wire as
`(path, value)` pairs, rebuilt server-side with `Submission`, with `FormErrors`
routed back onto the fields. That is the part of formoxus that is genuinely
hard to get right from the docs alone, and a single-file example cannot show it.
