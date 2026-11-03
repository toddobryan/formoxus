---
name: feedback-macro-first
description: Formoxus optimizes for form!; a dev hand-building a FormSpec must call the builders correctly, and gets a clear panic, not an order-free design
metadata:
  type: feedback
---

When a `FormSpec` builder's validity depends on another builder having run
first (e.g. `with_sort_by` on a `Vec` needs `treat_as_set` before it), Todd
chooses: `form!` emits the calls in the right order, and the hand-built path
just panics with a message saying what to call first. Don't redesign for
order-independence (e.g. deferring the check to build time).

**Why:** (Todd, 2026-11-03, SETS_PLAN.md box 5 decision B) "we're optimizing
for use of the macro. If people try to do things manually, they have to do
them correctly." Same spirit as `SetOrder::default_for` panicking on a
hand-built spec that broke the contract.

**How to apply:** when a runtime builder has a precondition the macro
guarantees, propose a precise panic message naming the fix, not a structural
change. Still make the message good: it is the hand-builder's only guide.
Related: [[lists-and-sets]], [[attribute-rules-design]].
