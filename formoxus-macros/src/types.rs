use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// The built-in types, each with its field. Every list gets a `FieldFor`
/// impl for each, and no list can replace one (decision AF).
/// `is_built_in` in `formoxus/src/type_registry.rs` is the run-time copy;
/// a test pins the two to each other.
///
/// **`usize` and `isize` are left out on purpose, not by oversight.** Their
/// width is target-dependent, 32 bits on the wasm client and 64 on the
/// server, so the same field would carry different bounds on each side of the
/// wire, and a value the server accepts could be unparseable in the browser.
/// A form model uses a fixed width (`u32`, `u64`) and converts at the edges.
/// Adding them would compile and look fine; the disagreement only shows up at
/// run time, on one target. (Moved here from `build.rs`' `scalar_node`, which
/// goes once these are the built-ins, 5.9.3.9.)
fn built_ins() -> Vec<(TokenStream, TokenStream)> {
    let fields = quote!(::formoxus::fields);
    let mut list = vec![
        (quote!(::std::string::String), quote!(#fields::StringField)),
        (quote!(::core::primitive::bool), quote!(#fields::BoolField)),
    ];
    for n in [
        "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
    ] {
        let n = format_ident!("{n}");
        list.push((
            quote!(::core::primitive::#n),
            quote!(#fields::NumberField<::core::primitive::#n>),
        ));
    }
    list
}
