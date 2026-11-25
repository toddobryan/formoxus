---
name: facetous-probe
description: "PROVEN 2026-11-25 (facet 0.46.5): a hand-written `facetous::__attr!` types each constraint BY THE FIELD (derive passes `$field:tt : $ty:ty`); misuse = compile errors; const fn reads type- and field-level keys"
metadata:
  type: project
---

`facetous` = Todd's name (2026-11-25) for the neutral model-constraint crate
(CUSTOM_FIELDS_PLAN box 10; lives in the formoxus workspace for now, may move
out). Probe in that day's scratchpad (`facetous-probe/`, not kept).

1. **The facet derive calls `ns::__attr!(@const? @ns { ns } key { field : Type | = value })`**
   for a field-level `#[facet(ns::key = value)]` (container level: `key { }` /
   `key { | args }`). So the namespace crate may HAND-WRITE `#[macro_export]
   macro_rules! __attr` and see the field's type. `define_attr_grammar!` only
   uses `$ty` for `predicate`/`validator`; its `make_t` `put`s an untyped literal
   (i32 by default), its value kinds are fixed `i64`/`usize`. Don't use it.
2. Store with `::facet::Attr::new(Some("facetous"), "min", &const { let v: <$ty as Numeric>::Bound = $v; v })`.
   `Numeric` impl'd for ints/floats and `Option<N>` (Bound peels Option). Read:
   `field.attributes` (`ns == Some("facetous")`), `Peek::unchecked_new(attr.data.ptr(), attr.data.shape())`,
   compare with `partial_cmp`. Fn-holding payloads need an opaque hand `Facet`
   impl; read with `OxRef::get_as::<T>(T::SHAPE)` (there is no `get`).
3. Compile errors, all at the value/key: 256 on u8 (overflowing_literals, deny
   by default), 0.5 on u32 (E0308), `min` on String / `max_length` on u32 /
   `min_items` on String (`on_unimplemented` on `Numeric`/`Text`/`Items`),
   `check` fn of the wrong arg type (E0308), unknown key (catch-all arm's
   `compile_error!`).
4. `check = f` stored as `unsafe fn(PtrConst) -> Result<(), String>` via an inner
   wrapper fn; can't use a generic struct's params (same as facet's validator).
5. Alias works: `use facetous as rules;` → `rules::min`; stored ns stays "facetous".
6. `const fn` reads `shape.attributes` (type-level, e.g. `facetous::text`) and
   `st.fields[i].attributes` by field name: so `form!`'s `const _` can see a
   model field's constraints (decision S: duplicate = compile error).
7. NOT compile-time: regex syntax of `pattern` (needs a proc macro), `min > max`.

Relates to [[facet-validate-crate]], [[models-stay-plain]], [[facet-proxy-hook]].
