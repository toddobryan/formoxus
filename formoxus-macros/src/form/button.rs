//! `buttons: { … }` — the button vocabulary, and what one button declares.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Ident, Result, Token, braced,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

use super::suggest::{edit_distance, to_snake};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ButtonInfo {
    pub name: Ident,
    pub ty: ButtonType,
    pub text: Option<String>,
    pub invocation: Option<Invocation>,
}

impl ButtonInfo {
    /// The `ButtonSpec` this declares, fully qualified.
    ///
    /// The name goes across as a string because it is a map key at render, not
    /// an identifier — the reflection path has no per-form type to hang a field
    /// on, which is the whole reason handlers are reconciled at runtime.
    pub(crate) fn spec_tokens(&self) -> TokenStream2 {
        let name = self.name.to_string();
        let ty = self.ty.tokens();
        let text = self.text.as_ref().map(|t| quote! { .with_text(#t) });
        let invocation = self.invocation.as_ref().map(|i| {
            let i = i.tokens();
            quote! { .with_invocation(#i) }
        });
        quote! {
            ::formoxus::buttons::ButtonSpec::new(#name, #ty) #text #invocation
        }
    }

    /// The braced body after `name:`. `type` is the only required key; `text`
    /// falls back to the name run through the form's label casing, and
    /// `invocation` to whatever the type implies.
    pub(crate) fn parse_body(name: Ident, input: ParseStream<'_>) -> Result<Self> {
        if !input.peek(syn::token::Brace) {
            return Err(input.error("expected a button body surrounded by braces"));
        }
        let body;
        let braces = braced!(body in input);

        let mut ty: Option<ButtonType> = None;
        let mut text: Option<String> = None;
        let mut invocation: Option<Invocation> = None;
        while !body.is_empty() {
            // `parse_any`, not the plain `Ident` parse: `type` is a reserved
            // word, so the ordinary parse rejects the one key that is required.
            // HTML spells the attribute `type` and so does this — `kind` would
            // be a second name for a thing the author already knows.
            let key = Ident::parse_any(&body)?;
            let _colon: Token![:] = body.parse()?;
            match key.to_string().as_str() {
                "type" if ty.is_some() => {
                    return Err(syn::Error::new_spanned(&key, "duplicate type"));
                }
                "type" => ty = Some(body.parse()?),
                "text" if text.is_some() => {
                    return Err(syn::Error::new_spanned(&key, "duplicate text"));
                }
                // A literal, not an `Expr`: the text is baked into the spec.
                "text" => text = Some(body.parse::<syn::LitStr>()?.value()),
                "invocation" if invocation.is_some() => {
                    return Err(syn::Error::new_spanned(&key, "duplicate invocation"));
                }
                "invocation" => invocation = Some(body.parse()?),
                other => {
                    return Err(syn::Error::new_spanned(
                        &key,
                        format!("unknown key {other}, expected type, text, or invocation"),
                    ));
                }
            }
            if body.peek(Token![,]) {
                body.parse::<Token![,]>()?;
            }
        }

        let Some(ty) = ty else {
            // Nothing sensible to default to: every other key describes a
            // button that already exists, and the type is what decides whether
            // it submits, resets, or merely runs a handler.
            return Err(syn::Error::new(
                braces.span.join(),
                format!(
                    "`{name}` needs a `type` — one of {}",
                    BUTTON_TYPE_NAMES.join(", ")
                ),
            ));
        };
        Ok(ButtonInfo {
            name,
            ty,
            text,
            invocation,
        })
    }
}

/// When the button's handler runs, overriding what its type implies.
///
/// Spelled `invocation` rather than `handler: validated | unchecked`, which was
/// the obvious alternative: `validated` names the *handler*, and what is
/// actually being chosen is whether the model has to pass validation before the
/// handler is reached at all.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Invocation {
    IfModelValidates,
    Unconditional,
}

impl Invocation {
    fn tokens(&self) -> TokenStream2 {
        let variant = match self {
            Invocation::IfModelValidates => quote!(IfModelValidates),
            Invocation::Unconditional => quote!(Unconditional),
        };
        quote! { ::formoxus::buttons::Invocation::#variant }
    }
}

impl Parse for Invocation {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let name: Ident = input.parse()?;
        match name.to_string().as_str() {
            "if_model_validates" => Ok(Invocation::IfModelValidates),
            "unconditional" => Ok(Invocation::Unconditional),
            other => Err(syn::Error::new(
                name.span(),
                format!(
                    "unknown invocation `{other}` — expected if_model_validates or unconditional"
                ),
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ButtonType {
    Destructive,
    Reset,
    Cancel,
    Button,
    Submit,
}

/// The author-facing button-type vocabulary, same shape and same reasoning as
/// [`widgets!`]: one table generates both the parse and the list the error
/// message reads from, so the two cannot drift apart.
///
/// Lowercase for the same reason widget names are — three of these five are
/// literally the HTML `type` attribute, and the author is writing HTML's word.
macro_rules! button_types {
    ( $( $name:ident => $variant:ident ),* $(,)? ) => {
        impl Parse for ButtonType {
            fn parse(input: ParseStream<'_>) -> Result<Self> {
                let name: Ident = input.parse()?;
                match name.to_string().as_str() {
                    $( stringify!($name) => Ok(ButtonType::$variant), )*
                    _ => Err(syn::Error::new(name.span(), unknown_button_type(&name))),
                }
            }
        }

        impl ButtonType {
            /// The `ButtonType` variant this names, fully qualified. Generated
            /// from the same table as the parse, so a new type cannot be
            /// accepted and then fail to expand.
            fn tokens(&self) -> TokenStream2 {
                match self {
                    $( ButtonType::$variant => quote! {
                        ::formoxus::buttons::ButtonType::$variant
                    }, )*
                }
            }
        }

        const BUTTON_TYPE_NAMES: &[&str] = &[ $( stringify!($name) ),* ];
    };
}

button_types! {
    submit      => Submit,
    reset       => Reset,
    cancel      => Cancel,
    destructive => Destructive,
    button      => Button,
}

/// The message for a button type that is not in the table — the same three
/// cases, in the same order, as [`unknown_widget`].
fn unknown_button_type(name: &Ident) -> String {
    let written = name.to_string();
    let lowered = to_snake(&written);
    if lowered != written && BUTTON_TYPE_NAMES.contains(&lowered.as_str()) {
        return format!(
            "unknown button type `{written}` — type names are lowercase, write `{lowered}`"
        );
    }
    match BUTTON_TYPE_NAMES
        .iter()
        .filter(|n| edit_distance(&lowered, n) <= 2)
        .min_by_key(|n| edit_distance(&lowered, n))
    {
        Some(near) => format!("unknown button type `{written}` — did you mean `{near}`?"),
        None => format!(
            "unknown button type `{written}` — expected one of {}",
            BUTTON_TYPE_NAMES.join(", ")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::form::tests::{err_of, kinds, parse};
    use crate::form::{Entry, FormSpecInput};
    use googletest::prelude::*;
    use quote::quote;

    /// Buttons as `("name", "Type", "text")`, in source order, with `""` for
    /// absent text. Flattened across entries, though the parser allows only one
    /// `buttons` block.
    fn buttons(spec: &FormSpecInput) -> Vec<(String, String, String)> {
        spec.entries
            .iter()
            .filter_map(|e| match e {
                Entry::Buttons(bs) => Some(bs),
                _ => None,
            })
            .flatten()
            .map(|b| {
                (
                    b.name.to_string(),
                    format!("{:?}", b.ty),
                    b.text.clone().unwrap_or_default(),
                )
            })
            .collect()
    }

    // ── Buttons ──────────────────────────────────────────────────────────

    #[gtest]
    fn a_buttons_block_parses() {
        // The shape `tests/consumer/buttons.rs` asks for, verbatim. Also the
        // regression test for dispatch order: `buttons` is a custom keyword and
        // therefore also an `Ident`, so if the field arm is tried first this
        // never reaches `parse_buttons` and dies on the missing `=>`.
        let spec = parse(quote! {
            FakeFormWithButtons {
                some_data => { widget: textarea },
                buttons: {
                    delete: { type: destructive, text: "Drop" },
                    reload: { type: reset },
                    update: { type: submit, text: "Save to Db" },
                }
            }
        })
        .expect("a buttons block should parse");

        expect_that!(kinds(&spec), elements_are![eq(&"field"), eq(&"buttons")]);
        expect_that!(
            buttons(&spec),
            elements_are![
                eq(&(
                    "delete".to_string(),
                    "Destructive".to_string(),
                    "Drop".to_string()
                )),
                eq(&("reload".to_string(), "Reset".to_string(), String::new())),
                eq(&(
                    "update".to_string(),
                    "Submit".to_string(),
                    "Save to Db".to_string()
                )),
            ]
        );
    }

    #[gtest]
    fn button_order_is_source_order() {
        // Display order is the only thing the author can say about layout, so
        // the parser must not sort or dedup into a map.
        let spec = parse(quote! {
            Source {
                buttons: {
                    zebra: { type: cancel },
                    apple: { type: submit },
                }
            }
        })
        .unwrap();
        expect_that!(
            buttons(&spec)
                .iter()
                .map(|b| b.0.clone())
                .collect::<Vec<_>>(),
            elements_are![eq("zebra"), eq("apple")]
        );
    }

    #[gtest]
    fn every_button_type_is_accepted() {
        let spec = parse(quote! {
            Source {
                buttons: {
                    a: { type: submit },
                    b: { type: reset },
                    c: { type: cancel },
                    d: { type: destructive },
                    e: { type: button },
                }
            }
        })
        .unwrap();
        expect_that!(
            buttons(&spec)
                .iter()
                .map(|b| b.1.clone())
                .collect::<Vec<_>>(),
            elements_are![
                eq("Submit"),
                eq("Reset"),
                eq("Cancel"),
                eq("Destructive"),
                eq("Button")
            ]
        );
    }

    #[gtest]
    fn an_invocation_overrides_what_the_type_implies() {
        let spec = parse(quote! {
            Source {
                buttons: { preview: { type: button, invocation: if_model_validates } }
            }
        })
        .unwrap();
        let Entry::Buttons(bs) = &spec.entries[0] else {
            panic!("expected a buttons entry");
        };
        expect_that!(bs[0].invocation, some(eq(&Invocation::IfModelValidates)));
    }

    #[gtest]
    fn an_absent_invocation_stays_none() {
        // `None` means "whatever the type implies", which is a different fact
        // from either variant — the renderer needs to be able to tell.
        let spec = parse(quote! { Source { buttons: { go: { type: submit } } } }).unwrap();
        let Entry::Buttons(bs) = &spec.entries[0] else {
            panic!("expected a buttons entry");
        };
        expect_that!(bs[0].invocation, none());
    }

    #[gtest]
    fn trailing_commas_are_allowed_in_both_button_braces() {
        parse(quote! {
            Source {
                buttons: {
                    go: { type: submit, text: "Go", },
                },
            }
        })
        .expect("trailing commas in a buttons block should parse");
    }

    // ── Button errors ────────────────────────────────────────────────────

    #[gtest]
    fn a_button_needs_a_type() {
        let msg = err_of(quote! { Source { buttons: { go: { text: "Go" } } } });
        expect_that!(msg, contains_substring("go"));
        expect_that!(msg, contains_substring("type"));
    }

    #[gtest]
    fn an_unknown_button_type_suggests_a_near_miss() {
        expect_that!(
            err_of(quote! { Source { buttons: { go: { type: submitt } } } }),
            contains_substring("did you mean `submit`")
        );
    }

    #[gtest]
    fn a_capitalized_button_type_is_told_to_lowercase() {
        // The spelling an author copies off the `ButtonType` enum.
        expect_that!(
            err_of(quote! { Source { buttons: { go: { type: Submit } } } }),
            contains_substring("write `submit`")
        );
    }

    #[gtest]
    fn an_unknown_button_key_names_itself() {
        let msg = err_of(quote! { Source { buttons: { go: { type: submit, clas: "x" } } } });
        expect_that!(msg, contains_substring("clas"));
        expect_that!(
            msg,
            contains_substring("expected type, text, or invocation")
        );
    }

    #[gtest]
    fn a_duplicate_button_key_is_rejected() {
        expect_that!(
            err_of(quote! { Source { buttons: { go: { type: submit, type: reset } } } }),
            contains_substring("duplicate")
        );
    }

    #[gtest]
    fn a_repeated_button_name_is_rejected() {
        // Two slots of the same name in the generated handler struct.
        let msg = err_of(quote! {
            Source {
                buttons: {
                    go: { type: submit },
                    go: { type: reset },
                }
            }
        });
        expect_that!(msg, contains_substring("go"));
        expect_that!(msg, contains_substring("twice"));
    }

    #[gtest]
    fn two_buttons_blocks_are_rejected() {
        expect_that!(
            err_of(quote! {
                Source {
                    buttons: { go: { type: submit } },
                    buttons: { stop: { type: cancel } },
                }
            }),
            contains_substring("`buttons` is given twice")
        );
    }

    #[gtest]
    fn an_empty_buttons_block_is_rejected() {
        expect_that!(
            err_of(quote! { Source { buttons: {} } }),
            contains_substring("empty buttons block")
        );
    }

    #[gtest]
    fn a_buttons_block_must_be_braced() {
        expect_that!(
            err_of(quote! { Source { buttons: submit } }),
            contains_substring("braces")
        );
    }

    #[gtest]
    fn a_button_body_must_be_braced() {
        expect_that!(
            err_of(quote! { Source { buttons: { go: submit } } }),
            contains_substring("braces")
        );
    }

    #[gtest]
    fn button_text_must_be_a_literal() {
        // An `Expr` would let a `const` through and make the spec non-constant.
        expect_that!(
            err_of(quote! { Source { buttons: { go: { type: submit, text: SAVE } } } }).len(),
            gt(0)
        );
    }
}
