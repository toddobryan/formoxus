//! `types! { pub struct AppTypes; include lib::LibTypes; T => newtype V, … }`:
//! one list of a crate's types, known at compile time and carried by each
//! form at run time (`CUSTOM_FIELDS_PLAN.md` decision AB, box 5.9.4).
//!
//! The grammar, after any outer attributes (a doc comment on the list):
//!
//! ```text
//! [vis] struct Name;
//! include krate::List;            // any number, each another crate's `pub` list
//! T => newtype V,                 // T read and written through the list's field for V
//! T => field F,                   // T through F, built with `F::default()`
//! replace krate::T => field F,    // an included entry, superseded (AF)
//! ```
//!
//! What it writes: the struct; one `FieldFor` impl per entry, per built-in and
//! for `Option<T>`; `registry()`, `overrides()` and `install()`; and, for a
//! `pub` list, the replay macro another crate's `include` calls (AI).
//!
//! **How `include` works: replay.** A proc macro cannot see another crate's
//! macro input, so a `pub` list also writes `__formoxus_replay_<Name>!`, a
//! `macro_rules!` holding its own entries. `include krate::List;` calls
//! `krate::__formoxus_replay_List!`, which adds its entries to an accumulator
//! (`@acc [...]`) and calls `types!` back with the rest of the input, until no
//! `include` is left. The replayed entries are written `$crate::…` by THIS
//! macro, so `$crate` carries the library's span and names the library: from
//! `macro_rules!` it would name formoxus (the probe hit that).

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{
    Attribute, GenericArgument, Ident, LitStr, Path, PathArguments, Token, Type, TypePath,
    Visibility, braced, bracketed, parenthesized,
};

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
fn built_ins() -> Vec<(&'static str, TokenStream, TokenStream)> {
    let fields = quote!(::formoxus::fields);
    let mut list = vec![
        (
            "String",
            quote!(::std::string::String),
            quote!(#fields::StringField),
        ),
        (
            "bool",
            quote!(::core::primitive::bool),
            quote!(#fields::BoolField),
        ),
    ];
    for n in [
        "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
    ] {
        let ident = format_ident!("{n}");
        list.push((
            n,
            quote!(::core::primitive::#ident),
            quote!(#fields::NumberField<::core::primitive::#ident>),
        ));
    }
    list
}

pub(crate) fn impl_types(input: TokenStream) -> TokenStream {
    match syn::parse2::<Input>(input).and_then(Input::expand) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

/// How an entry's field is made.
#[derive(Clone)]
enum How {
    /// `newtype V`: a `Newtype<T, _>` over the list's own field for `V`.
    Newtype(TokenStream),
    /// `field F`: `F::default()`.
    Field(TokenStream),
}

impl How {
    fn parse_after_arrow(input: ParseStream<'_>) -> syn::Result<Self> {
        let kw: Ident = input.parse()?;
        let arg: Type = input.parse()?;
        match kw.to_string().as_str() {
            "newtype" => Ok(How::Newtype(quote!(#arg))),
            "field" => Ok(How::Field(quote!(#arg))),
            _ => Err(syn::Error::new(
                kw.span(),
                "expected `newtype` or `field` after `=>`: `T => newtype String` or `T => field YourField`",
            )),
        }
    }

    fn keyword(&self) -> Ident {
        match self {
            How::Newtype(_) => format_ident!("newtype"),
            How::Field(_) => format_ident!("field"),
        }
    }

    fn arg(&self) -> &TokenStream {
        match self {
            How::Newtype(arg) | How::Field(arg) => arg,
        }
    }
}

/// An entry another crate's list replayed into this one: its tokens come
/// `$crate`-qualified, so they are carried as tokens and never re-parsed as
/// types (`$crate` is not a path syn takes apart).
struct Included {
    /// The defining crate's name, as `CARGO_CRATE_NAME` spells it.
    krate: String,
    /// The type's path from that crate's root, `Stamp` or `a::Stamp`, which a
    /// `replace krate::a::Stamp` is matched against.
    path: String,
    ty: TokenStream,
    how: How,
}

impl Parse for Included {
    /// `("krate" "path" { ty } newtype { V })`, as `replay_macro` writes it.
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let content;
        parenthesized!(content in input);
        let krate: LitStr = content.parse()?;
        let path: LitStr = content.parse()?;
        let ty;
        braced!(ty in content);
        let ty: TokenStream = ty.parse()?;
        let kw: Ident = content.parse()?;
        let arg;
        braced!(arg in content);
        let arg: TokenStream = arg.parse()?;
        let how = if kw == "newtype" {
            How::Newtype(arg)
        } else {
            How::Field(arg)
        };
        Ok(Included {
            krate: krate.value(),
            path: path.value(),
            ty,
            how,
        })
    }
}

/// One line of the dev's own list.
struct Entry {
    replace: bool,
    ty: Type,
    how: How,
}

impl Parse for Entry {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        // `replace` is a keyword only when a type follows it, not `=>`.
        let replace =
            input.fork().parse::<Ident>().is_ok_and(|i| i == "replace") && !input.peek2(Token![=>]);
        if replace {
            input.parse::<Ident>()?;
        }
        let ty: Type = input.parse()?;
        input.parse::<Token![=>]>()?;
        let how = How::parse_after_arrow(input)?;
        Ok(Entry { replace, ty, how })
    }
}

struct Input {
    included: Vec<Included>,
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    /// The `include`s not yet replayed; the first is the next to call.
    includes: Vec<Path>,
    /// Everything after the `include`s, unparsed while an `include` is left,
    /// since it is handed on to the next replay as it is.
    rest: TokenStream,
}

impl Parse for Input {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut included = Vec::new();
        if input.peek(Token![@]) {
            input.parse::<Token![@]>()?;
            let acc: Ident = input.parse()?;
            if acc != "acc" {
                return Err(syn::Error::new(acc.span(), "expected `@acc`"));
            }
            let content;
            bracketed!(content in input);
            while !content.is_empty() {
                included.push(content.parse::<Included>()?);
                content.parse::<Token![,]>()?;
            }
        }
        let attrs = input.call(Attribute::parse_outer)?;
        let vis: Visibility = input.parse()?;
        input.parse::<Token![struct]>()?;
        let name: Ident = input.parse()?;
        input.parse::<Token![;]>()?;
        let mut pending = Vec::new();
        while input.fork().parse::<Ident>().is_ok_and(|i| i == "include") {
            input.parse::<Ident>()?;
            pending.push(input.parse::<Path>()?);
            input.parse::<Token![;]>()?;
        }
        Ok(Input {
            included,
            attrs,
            vis,
            name,
            includes: pending,
            rest: input.parse()?,
        })
    }
}

/// A path's segments as idents, generic arguments ignored: what a `replace`
/// is matched by.
fn segments(path: &Path) -> Vec<String> {
    path.segments.iter().map(|s| s.ident.to_string()).collect()
}

fn type_path(ty: &Type) -> Option<&Path> {
    match ty {
        Type::Path(TypePath {
            qself: None, path, ..
        }) => Some(path),
        _ => None,
    }
}

/// The built-in a type names, if any: `String`, `std::string::String`, `u8`.
/// A dev's own `crate::String` is not one.
fn built_in_named(ty: &Type) -> Option<&'static str> {
    let path = type_path(ty)?;
    let last = path.segments.last()?;
    if !matches!(last.arguments, PathArguments::None) {
        return None;
    }
    let name = last.ident.to_string();
    let first = path.segments.first().map(|s| s.ident.to_string());
    if path.segments.len() > 1 && !matches!(first.as_deref(), Some("std" | "core" | "alloc")) {
        return None;
    }
    built_ins()
        .into_iter()
        .map(|(n, _, _)| n)
        .find(|n| *n == name)
}

/// `ty`, written so it resolves from another crate that replays it: a root
/// name or a `crate::…` path becomes `$crate::…`, an absolute `::krate::…`
/// path stays, and a built-in becomes its full path. Anything else is
/// refused: from inside the macro there is no telling whether `models::X`
/// names a module of this crate or another crate.
fn replayable(ty: &Type) -> syn::Result<TokenStream> {
    if let Some(name) = built_in_named(ty) {
        let (_, full, _) = built_ins()
            .into_iter()
            .find(|(n, _, _)| *n == name)
            .expect("just found");
        return Ok(full);
    }
    let Some(path) = type_path(ty) else {
        return Err(syn::Error::new_spanned(
            ty,
            "a `pub` list's entries are replayed into the crates that include it, so each must \
             be a named type",
        ));
    };
    let mut out = TokenStream::new();
    let mut segments = path.segments.iter().peekable();
    if path.leading_colon.is_some() {
        out.extend(quote!(::));
    } else if path.segments.len() == 1 {
        out.extend(quote!($crate::));
    } else if path.segments[0].ident == "crate" {
        segments.next();
        out.extend(quote!($crate::));
    } else {
        return Err(syn::Error::new_spanned(
            ty,
            "a `pub` list's entries are replayed into the crates that include it: write a name \
             at this crate's root (`pub use` it there), `crate::…`, or `::other_crate::…`",
        ));
    }
    let mut first = true;
    for segment in segments {
        if !first {
            out.extend(quote!(::));
        }
        first = false;
        let ident = &segment.ident;
        out.extend(quote!(#ident));
        if let PathArguments::AngleBracketed(args) = &segment.arguments {
            let mut inner = Vec::new();
            for arg in &args.args {
                match arg {
                    GenericArgument::Type(t) => inner.push(replayable(t)?),
                    other => inner.push(quote!(#other)),
                }
            }
            out.extend(quote!(<#(#inner),*>));
        }
    }
    Ok(out)
}

impl Input {
    fn expand(self) -> syn::Result<TokenStream> {
        if !self.includes.is_empty() {
            return Ok(self.call_next_replay());
        }
        let entries = Punctuated::<Entry, Token![,]>::parse_terminated.parse2(self.rest.clone())?;
        let entries: Vec<Entry> = entries.into_iter().collect();
        self.write_list(&entries)
    }

    /// Hands what has been gathered, and the rest of the input, to the next
    /// `include`'s replay macro.
    fn call_next_replay(&self) -> TokenStream {
        let next = &self.includes[0];
        if next.segments.len() < 2 {
            return syn::Error::new_spanned(
                next,
                "`include` names another crate's list by its crate: `include krate::LibTypes;`. \
                 A list in this crate cannot be included, since a crate cannot reach its own \
                 generated macros by path",
            )
            .to_compile_error();
        }
        let later = &self.includes[1..];
        let krate = &next.segments[0].ident;
        let list = &next.segments.last().expect("checked").ident;
        if krate == "formoxus" && list == "BuiltIns" {
            return syn::Error::new_spanned(
                next,
                "every `types!` list already holds the built-in types, so `BuiltIns` is never \
                 included",
            )
            .to_compile_error();
        }
        let replay = format_ident!("__formoxus_replay_{list}");
        let gathered = self.included.iter().map(|inc| {
            let (krate, path, ty) = (&inc.krate, &inc.path, &inc.ty);
            let (kw, arg) = (inc.how.keyword(), inc.how.arg());
            quote!((#krate #path { #ty } #kw { #arg }),)
        });
        let (attrs, vis, name, rest) = (&self.attrs, &self.vis, &self.name, &self.rest);
        quote! {
            ::#krate::#replay! {
                @acc [ #(#gathered)* ]
                #(#attrs)* #vis struct #name;
                #( include #later; )*
                #rest
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn write_list(&self, entries: &[Entry]) -> syn::Result<TokenStream> {
        let name = &self.name;
        let field_for = quote!(::formoxus::type_registry::FieldFor);
        let registry = quote!(::formoxus::type_registry::TypeRegistry);
        let field = quote!(::formoxus::field::Field);

        // AF: built-ins read the same everywhere.
        for entry in entries {
            if let Some(built_in) = built_in_named(&entry.ty) {
                return Err(syn::Error::new_spanned(
                    &entry.ty,
                    format!(
                        "`{built_in}` is built in and reads the same in every form, so no list \
                         can give it a field. For a `{built_in}` that reads differently, \
                         declare a type of your own (`struct Shouty({built_in});`) and list that"
                    ),
                ));
            }
        }

        // Each `replace`, matched to the included entry it supersedes.
        let mut superseded = vec![false; self.included.len()];
        let mut kind_checks = Vec::new();
        for entry in entries.iter().filter(|e| e.replace) {
            let segs = type_path(&entry.ty).map(segments).unwrap_or_default();
            let found = segs.split_first().and_then(|(krate, rest)| {
                let path = rest.join("::");
                self.included
                    .iter()
                    .position(|inc| inc.krate == *krate && inc.path == path)
            });
            let Some(at) = found else {
                return Err(syn::Error::new_spanned(
                    &entry.ty,
                    "`replace` supersedes an entry of an included list, named by its crate path \
                     (`replace their_crate::Badge => …`), and no included list has this type. \
                     To give a type of your own its field, list it without `replace`",
                ));
            };
            superseded[at] = true;
            // AB.8: the included list's forms render the type with a widget
            // for its kind; a replacement of another kind is a mistake.
            let ty = &entry.ty;
            let theirs = match &self.included[at].how {
                How::Field(f) => quote!(<#f as #field>::KIND),
                How::Newtype(v) => quote!(<<#name as #field_for<#v>>::Field as #field>::KIND),
            };
            kind_checks.push(quote! {
                const _: () = ::core::assert!(
                    <<#name as #field_for<#ty>>::Field as #field>::KIND as u8 == #theirs as u8,
                    "a `replace` must keep the kind of field the included list gave the type: \
                     that list's forms render it with a widget for that kind",
                );
            });
        }

        // AB.4.4: a type an included list has needs `replace`, not a second
        // entry. Caught here when the entry spells the crate path; an
        // imported name slips through to rustc's E0119.
        for entry in entries.iter().filter(|e| !e.replace) {
            let segs = type_path(&entry.ty).map(segments).unwrap_or_default();
            if let Some((krate, rest)) = segs.split_first() {
                let path = rest.join("::");
                if self
                    .included
                    .iter()
                    .any(|inc| inc.krate == *krate && inc.path == path)
                {
                    return Err(syn::Error::new_spanned(
                        &entry.ty,
                        "an included list already has this type: write `replace` to give it \
                         another field",
                    ));
                }
            }
        }

        let impl_for = |ty: &TokenStream, how: &How| match how {
            How::Field(f) => quote! {
                impl #field_for<#ty> for #name {
                    type Settings = #f;
                    type Field = #f;
                    fn build(settings: #f) -> #f {
                        settings
                    }
                }
            },
            How::Newtype(v) => quote! {
                impl #field_for<#ty> for #name {
                    type Settings = <#name as #field_for<#v>>::Settings;
                    type Field =
                        ::formoxus::fields::Newtype<#ty, <#name as #field_for<#v>>::Field>;
                    fn build(settings: Self::Settings) -> Self::Field {
                        ::formoxus::fields::Newtype::new(
                            <#name as #field_for<#v>>::build(settings),
                        )
                    }
                }
            },
        };

        let mut impls = Vec::new();
        let mut listed = Vec::new(); // every type the run-time registry holds
        for (_, ty, f) in built_ins() {
            impls.push(impl_for(&ty, &How::Field(f)));
            listed.push(ty);
        }
        let first_own = listed.len(); // `KINDS` leaves out the built-ins
        for (inc, gone) in self.included.iter().zip(&superseded) {
            if !gone {
                impls.push(impl_for(&inc.ty, &inc.how));
                listed.push(inc.ty.clone());
            }
        }
        let mut replaced = Vec::new();
        for entry in entries {
            let ty = &entry.ty;
            let ty = quote!(#ty);
            impls.push(impl_for(&ty, &entry.how));
            listed.push(ty.clone());
            if entry.replace {
                replaced.push(ty);
            }
        }

        let replay = self.replay_macro(entries)?;
        let (attrs, vis) = (&self.attrs, &self.vis);
        // QUESTION for Todd (5.9.4, recorded in the plan): built-ins go
        // through the public `field_for` until the hidden `__built_in` exists
        // (5.9.3.2.4); `field_for` does not refuse built-ins yet, so this works.
        let build = |ty: &TokenStream| quote!(.field_for(<#name as #field_for<#ty>>::build(::core::default::Default::default())));
        let registrations = listed.iter().map(build);
        // The built-ins are judged by shape, which already knows them.
        let kinds = listed[first_own..].iter().map(|ty| {
            quote! {
                (
                    <#ty as ::formoxus::facet::Facet<'static>>::SHAPE,
                    <<#name as #field_for<#ty>>::Field as #field>::KIND,
                )
            }
        });
        let overrides = replaced.iter().map(build);
        Ok(quote! {
            #(#attrs)*
            #vis struct #name;

            #(#impls)*

            impl<T> #field_for<::core::option::Option<T>> for #name
            where
                #name: #field_for<T>,
            {
                type Settings = <#name as #field_for<T>>::Settings;
                type Field = <#name as #field_for<T>>::Field;
                fn build(settings: Self::Settings) -> Self::Field {
                    <#name as #field_for<T>>::build(settings)
                }
            }

            #(#kind_checks)*

            impl ::formoxus::type_registry::TypeKinds for #name {
                const KINDS: &'static [(
                    &'static ::formoxus::facet::Shape,
                    ::formoxus::attrs::FieldType,
                )] = &[#(#kinds),*];
            }

            #[allow(dead_code)]
            impl #name {
                /// Every type this list holds, built-ins included: the
                /// run-time registry each of its forms carries.
                #[must_use]
                pub fn registry() -> &'static #registry {
                    static REGISTRY: ::std::sync::LazyLock<#registry> =
                        ::std::sync::LazyLock::new(|| #registry::new() #(#registrations)*);
                    &REGISTRY
                }

                /// This list's `replace` lines and nothing else: what
                /// `install` installs (decision AF).
                #[must_use]
                pub fn overrides() -> #registry {
                    #registry::new() #(#overrides)*
                }

                /// Installs [`overrides`](Self::overrides) for the process.
                pub fn install() {
                    #registry::install(Self::overrides());
                }
            }

            #replay
        })
    }

    /// For a `pub` list, the macro another crate's `include` calls: it adds
    /// this list's own entries (not its `replace`s or what it included,
    /// decision AF.2) to the accumulator and calls `types!` back.
    fn replay_macro(&self, entries: &[Entry]) -> syn::Result<TokenStream> {
        let krate = std::env::var("CARGO_CRATE_NAME").unwrap_or_default();
        // `BuiltIns` is `pub` for the prelude, but nothing includes it: every
        // list holds the built-ins already, and `call_next_replay` refuses it.
        let built_ins = krate == "formoxus" && self.name == "BuiltIns";
        if !matches!(self.vis, Visibility::Public(_)) || built_ins {
            return Ok(TokenStream::new());
        }
        let mut items = Vec::new();
        for entry in entries.iter().filter(|e| !e.replace) {
            let path = type_path(&entry.ty)
                .map(|p| {
                    let segs = segments(p);
                    let from = usize::from(segs.first().is_some_and(|s| s == "crate"));
                    segs[from..].join("::")
                })
                .unwrap_or_default();
            let ty = replayable(&entry.ty)?;
            let arg: Type = syn::parse2(entry.how.arg().clone())?;
            let arg = replayable(&arg)?;
            let kw = entry.how.keyword();
            items.push(quote!((#krate #path { #ty } #kw { #arg }),));
        }
        let replay = format_ident!("__formoxus_replay_{}", self.name);
        let acc = Ident::new("acc", Span::call_site());
        let rest = Ident::new("rest", Span::call_site());
        Ok(quote! {
            #[doc(hidden)]
            #[macro_export]
            macro_rules! #replay {
                (@acc [$($#acc:tt)*] $($#rest:tt)*) => {
                    ::formoxus::types! { @acc [$($#acc)* #(#items)*] $($#rest)* }
                };
            }
        })
    }
}
