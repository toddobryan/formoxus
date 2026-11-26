//! A path whose type is in no list, in a module with no `FormoxusTypes` of
//! its own, so the prelude's built-ins are the list (decision AC). The error
//! names the type and says where to list it, from `FieldFor`'s
//! `on_unimplemented`.
use facet::Facet;
use formoxus::prelude::*;

#[derive(Facet, Clone, Debug, PartialEq)]
struct Shade(String);

#[derive(Facet, Clone, Debug, PartialEq)]
struct Palette {
    shade: Shade,
}

fn main() {
    let _ = empty_form(form! { Palette { shade => { label: "Shade" } } });
}
