//! `using_fns!` — the handlers for one `render`, keyed by button name.
//!
//! ```ignore
//! form.render(using_fns! {
//!     save:   |m| async move { save(m).await },
//!     cancel: || async move { nav.go_back() },
//! })
//! ```
//!
//! **Why a map and not a struct literal.** A struct literal would give the
//! completeness check for free — forget `cancel` and rustc says `missing field
//! 'cancel'`. The reflection path has no struct to name: `form!` is an
//! *expression* macro producing a runtime `FormSpec` value, so at the `render`
//! call site, usually in another function, there is no per-form type in scope.
//! `<T as FormState>::Handlers { … }` does not close the gap either — qualified
//! paths in struct-literal position are still unstable (rust#86935). So the
//! check moves to render time, in `Fns::reconcile`.
//!
//! **Why the macro reads arity instead of a trait doing it.** `IntoSlot<T>`
//! picks an impl from the *target field's* type, which a map does not have. And
//! one target type cannot carry impls for both `Fn(M)` and `Fn()`: coherence
//! cannot prove a type implements only one, so it is a hard `E0119` (verified).
//! A macro has what neither has — the closure's source text — so it just looks.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Expr, Ident, Result, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

pub fn impl_using_fns(input: TokenStream2) -> TokenStream2 {
    match syn::parse2::<FnSet>(input) {
        Ok(set) => set.expand(),
        Err(err) => err.to_compile_error(),
    }
}

struct FnSet {
    entries: Vec<FnEntry>,
}

struct FnEntry {
    name: Ident,
    body: FnBody,
}

enum FnBody {
    /// `|m| …` — takes the model, so the form validates first.
    Validated(Expr),
    /// `|| …` — takes nothing and runs regardless.
    Unchecked(Expr),
}

impl Parse for FnSet {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let entries: Vec<FnEntry> = Punctuated::<FnEntry, Token![,]>::parse_terminated(input)?
            .into_iter()
            .collect();
        for (i, e) in entries.iter().enumerate() {
            if let Some(prev) = entries[..i].iter().find(|p| p.name == e.name) {
                return Err(syn::Error::new_spanned(
                    &e.name,
                    format!("`{}` is given twice — one fn per button", prev.name),
                ));
            }
        }
        Ok(FnSet { entries })
    }
}

impl Parse for FnEntry {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let expr: Expr = input.parse()?;
        let body = match &expr {
            Expr::Closure(c) => match c.inputs.len() {
                0 => FnBody::Unchecked(expr.clone()),
                1 => FnBody::Validated(expr.clone()),
                n => {
                    return Err(syn::Error::new_spanned(
                        &c.inputs,
                        format!(
                            "a button fn takes the model or nothing, not {n} arguments"
                        ),
                    ));
                }
            },
            // Anything else hides its arity. A path could be either shape, and
            // guessing would pick the wrong `ButtonFn` variant with no error
            // until the button is clicked.
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "expected a closure: `|m| async move { … }` to validate the model \
                     first, or `|| async move { … }` to run unconditionally. A named \
                     function needs one too — `|m| save(m)`",
                ));
            }
        };
        Ok(FnEntry { name, body })
    }
}

impl FnSet {
    fn expand(self) -> TokenStream2 {
        let entries = self.entries.iter().map(|e| {
            let name = e.name.to_string();
            let slot = match &e.body {
                FnBody::Validated(f) => quote! {
                    ::formoxus::buttons::ButtonFn::Validated(
                        ::formoxus::form::handler(#f)
                    )
                },
                FnBody::Unchecked(f) => quote! {
                    ::formoxus::buttons::ButtonFn::Unchecked(
                        ::formoxus::form::unchecked_handler(#f)
                    )
                },
            };
            quote! { .with(#name, #slot) }
        });
        quote! {
            ::formoxus::buttons::Fns::new()
            #(#entries)*
        }
    }
}
