---
name: facet-proxy-hook
description: "2026-11-23, PROVEN in a scratch probe on facet 0.46.5: `#[facet(formoxus::proxy = String)]` + TryFrom both ways gives a type its own text form, seen from Shape alone, no runtime; derive never fills vtable `parse` from FromStr; `formoxus::field` (own grammar, fn_ptr) also PROVEN, generics too; launch contexts do NOT reach server fns"
metadata:
  type: project
---

Answers issue #28's question B ("does facet 0.46 have a proxy/string hook?"). Yes.

**What works (probe, 2026-11-23, facet 0.46.5 + facet-reflect 0.46.5):**

```rust
#[derive(Facet)]
#[facet(formoxus::proxy = String)]
struct TimeLimit(u32);
impl TryFrom<String> for TimeLimit { … }        // parse
impl TryFrom<&TimeLimit> for String { … }       // format
```

1. **Namespaced, so only formoxus sees it.** `ns::proxy` lands in
   `Shape::format_proxies` as `FormatProxy { format: "formoxus", … }`; the
   namespace is just a string, nothing to register, and `shape.proxy` stays
   `None`, so facet-json and friends are untouched.
2. **Found from `Shape` alone**: `shape.format_proxy("formoxus")`, including on
   a struct field's shape. So `build.rs` meets it in list items and variants
   with no registry, and `Submission::accept` on the server needs no runtime.
3. **Format:** `Peek::new(&v).custom_serialization_with_proxy(p)` → `"1h15m"`.
4. **Parse:** `Partial::alloc::<T>()?.begin_custom_deserialization_from_shape_with_format(Some("formoxus"))?.0`,
   then `.set(String)?.end()?.build()`. NOT `begin_custom_deserialization_with_format`:
   that reads a FIELD-level proxy and fails at the root ("not currently processing a field").
5. **Error text is wrapped:** `Custom deserialization of shape 'String' into
   'TimeLimit' failed: "90": expected like 1h15m at <root>`. To show the dev's
   own message, call `ProxyDef::convert_in` directly or unwrap the kind.

**Dead ends:**
1. The derive never fills the vtable's `parse` from `FromStr`
   (`DeclaredTraits` has no from_str; auto-detect covers Display, Debug,
   PartialEq, …). A dev's `FromStr` alone is invisible to `parse_scalar`; only a
   hand-written `Facet` impl (`vtable_direct!` with `[parse = f]`) sets it.
2. **Dioxus 0.7.10 launch contexts do not reach server functions.**
   `LaunchBuilder::with_context` feeds the client root and the SSR vdom's root
   (`server.rs` `render_handler`); `register_server_functions_with_context` is
   named in a doc comment only and does not exist. So anything
   `Submission::accept` needs cannot live only in Dioxus context, which is
   #28's question A made concrete.

**`formoxus::field` PROVEN too (second probe, same day).** A lib crate named
`formoxus` with

```rust
pub type FieldFn = fn() -> &'static str;   // stand-in for the real constructor type
facet::define_attr_grammar! {
    ns "formoxus";
    crate_path ::formoxus;
    pub enum Attr { Field(fn_ptr FieldFn) }
}
// The grammar does NOT derive Facet for an enum holding a fn pointer, but
// `Attr::get_as` needs one; facet hand-writes the same for its builtin Attr:
unsafe impl facet::Facet<'_> for Attr {
    const SHAPE: &'static facet::Shape = &facet::Shape::builder_for_sized::<Self>("formoxus::Attr")
        .ty(facet::Type::User(facet::UserType::Opaque)).build();
}
```

and a model crate depending on it: `#[facet(formoxus::field = time_limit_field)]`
on a type is found by `shape.attributes` (ns `"formoxus"`, key `"field"`,
`get_as::<Attr>()`) on the type, as a struct field's shape, and as a
`Vec`'s item shape; it combines with `formoxus::proxy` in one `#[facet(…)]`;
on ONE struct field it lands in `field.get_attr(Some("formoxus"), "field")`;
`String` carries none. **Generic types work:** `#[facet(formoxus::field =
Ref::<T>::field)]` resolves per `T`. Costs: the model crate depends on
formoxus (the namespace resolves as the path `formoxus::__attr!`), and only
types the dev derives can carry it; foreign types still need `field:` in
`form!` or a registry.

**Todd, 2026-11-23:** formoxus initialization should follow Dioxus's app
creation (the `LaunchBuilder` style) and is where a dev sets default classes and
the type → field map. `Formoxus` + `provide_defaults` already exist; see
[[config-cascade]]. The server-reach gap above is what that design has to answer.

Probe sources: the 2026-11-23 session scratchpad (`proxyprobe/`, `fieldprobe/`), not kept.
Relates to [[facet-newtypes-and-custom-widgets]], [[django-parallel-design]].
