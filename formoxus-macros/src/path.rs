//! `path!(Model.field)` — a compile-checked path into a model.
//!
//! The same trick `form!` uses for its field entries, lifted out so it works
//! anywhere. A proc macro cannot see a field's type — it receives tokens, not
//! a type table — so it cannot check a path itself. What it *can* do is emit an
//! expression that mentions the field and let rustc check that:
//!
//! ```ignore
//! path!(Signup.venue.city)
//! // {
//! //     #[allow(unused)]
//! //     fn __formoxus_path_witness(__s: &Signup) { let _ = &__s.venue.city; }
//! //     ::formoxus::path::Path::<Signup>::__new("venue.city")
//! // }
//! ```
//!
//! The witness is never called. It exists so that a misspelled segment is
//! `E0609: no field 'pasword' on type '&Signup'` at the call site rather than a
//! `no_such_path` panic on a server somewhere.
//!
//! **This is why `Path::__new` is hidden.** The guarantee is not that a
//! `Path<T>` wraps a plausible string — it is that every `Path<T>` in existence
//! came from an invocation that type-checked. A public constructor would let
//! `Path::new("pasword")` walk straight back through the hole.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Path as TypePath, Result, Token};

use crate::form::{SpecPath, probe};

pub fn impl_path(input: TokenStream2) -> TokenStream2 {
    match syn::parse2::<PathInput>(input) {
        Ok(parsed) => parsed.expand(),
        Err(e) => e.to_compile_error(),
    }
}

struct PathInput {
    /// The model type. A `syn::Path`, so `crate::models::Signup` works — `::`
    /// binds tighter than the `.` that starts the field path, so this stops in
    /// the right place on its own.
    model: TypePath,
    field: SpecPath,
}

impl Parse for PathInput {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let model: TypePath = input.parse()?;

        if input.is_empty() {
            return Err(syn::Error::new_spanned(
                &model,
                "expected a field path — write `path!(Model.field)`, not just the model",
            ));
        }
        input.parse::<Token![.]>()?;

        let field: SpecPath = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("unexpected token after the field path"));
        }
        Ok(PathInput { model, field })
    }
}

impl PathInput {
    fn expand(self) -> TokenStream2 {
        let model = &self.model;
        let key = self.field.key();
        let witness = probe(&self.field.segments, quote!(__s), 0);

        quote! {
            {
                #[allow(unused)]
                fn __formoxus_path_witness(__s: &#model) { #witness }

                ::formoxus::path::Path::<#model>::__new(#key)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use googletest::prelude::*;

    fn expand(src: TokenStream2) -> String {
        impl_path(src).to_string()
    }

    #[gtest]
    fn a_plain_field_witnesses_the_borrow_and_keys_on_its_name() {
        let out = expand(quote! { Signup.password });
        expect_that!(out, contains_substring("__s . password"));
        expect_that!(out, contains_substring("\"password\""));
        expect_that!(out, contains_substring("Path :: < Signup >"));
    }

    #[gtest]
    fn a_nested_path_walks_the_whole_chain() {
        let out = expand(quote! { Event.venue.city });
        expect_that!(out, contains_substring("__s . venue . city"));
        expect_that!(out, contains_substring("\"venue.city\""));
    }

    #[gtest]
    fn a_qualified_model_keeps_its_leading_segments() {
        let out = expand(quote! { crate::models::Signup.email });
        expect_that!(out, contains_substring("crate :: models :: Signup"));
        expect_that!(out, contains_substring("\"email\""));
    }

    /// `[]` reuses `form!`'s loop, so a row path is checked the same way — the
    /// element type is inferred, so nothing has to name it.
    #[gtest]
    fn a_row_path_witnesses_a_loop() {
        let out = expand(quote! { Quiz.answers[].text });
        expect_that!(out, contains_substring("for __row0 in"));
        expect_that!(out, contains_substring("\"answers[].text\""));
    }

    #[gtest]
    fn the_model_alone_is_rejected() {
        expect_that!(
            expand(quote! { Signup }),
            contains_substring("expected a field path")
        );
    }

    #[gtest]
    fn trailing_tokens_are_rejected() {
        expect_that!(
            expand(quote! { Signup.email, "extra" }),
            contains_substring("unexpected token")
        );
    }
}
