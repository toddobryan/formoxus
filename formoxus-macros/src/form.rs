use darling::{FromDeriveInput, ast};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse2};

use crate::{
    container,
    error::MacroError,
    field_container_meta::FieldContainerMeta,
    field_meta::FieldMeta,
    form_meta::{FormMeta, HandlerKind},
};

/// Entry point for `#[derive(Form)]`.
pub fn derive_form(tokens: TokenStream2) -> TokenStream2 {
    match derive_inner(tokens) {
        Ok(tokens) => tokens,
        Err(err) => err.into_tokens(),
    }
}

fn derive_inner(tokens: TokenStream2) -> Result<TokenStream2, MacroError> {
    let input: DeriveInput = parse2(tokens)?;

    let Data::Struct(data) = &input.data else {
        return Err(MacroError::Syn(syn::Error::new_spanned(
            input.ident,
            "Form can only be derived for structs",
        )));
    };

    let Fields::Named(_) = &data.fields else {
        return Err(MacroError::Syn(syn::Error::new_spanned(
            &data.fields,
            "Deriving Form requires a struct with named fields",
        )));
    };

    let form_meta: FormMeta = FormMeta::from_derive_input(&input)?;
    let field_metas: &[FieldMeta] = match &form_meta.data {
        ast::Data::Struct(fields) => &fields.fields,
        ast::Data::Enum(_) => unreachable!("shape checked above: input is a struct"),
    };

    let form_impl = form_impl_for(&form_meta)?;

    let state_struct = container::state_struct(&form_meta, field_metas)?;

    let handlers_struct = form_handlers_struct(&form_meta)?;

    let providers_decl = container::providers_decl(&form_meta, field_metas);
    let providers_struct = &providers_decl.struct_decl;

    let validate_form_impl = container::validate_form_impl(&form_meta)?;

    // no separate Model to derive
    let trait_impls: Option<TokenStream2> = if form_meta.common.model.is_none() {
        let from_model_impl = container::from_model_impl(&form_meta, field_metas)?;
        let form_state_impl = form_state_impl(&form_meta, field_metas, &providers_decl.providers_type)?;

        Some(quote! {
            #from_model_impl

            #form_state_impl
        })
    } else {
        None
    };

    Ok(quote! {
        #form_impl

        #state_struct

        #handlers_struct

        #providers_struct

        #validate_form_impl

        #trait_impls
    })
}

fn form_impl_for(form_meta: &impl FieldContainerMeta) -> Result<TokenStream2, MacroError> {
    let form_ident = &form_meta.ident();
    let state_ident = &form_meta.state_struct_name();

    Ok(quote! {
        impl ::formoxus::form::Form for #form_ident {
            type State = #state_ident;
        }
    })
}

/// One field per `#[form(button(...))]`, named after the button, typed as
/// `Handler<Model>` or `UncheckedHandler` per that button's resolved
/// `HandlerKind`. Callers construct this by hand at the `store.render(...)`
/// call site — see `formoxus::form::{handler, unchecked_handler}`. `FieldSet`
/// has no buttons at all, so this one stays Form-only.
fn form_handlers_struct(form_meta: &FormMeta) -> Result<TokenStream2, MacroError> {
    let handlers_ident = form_meta.handlers_struct_name();
    let vis = &form_meta.vis;
    let model_ident = form_meta.common.model.as_ref().unwrap_or(&form_meta.ident);

    let fields: Vec<TokenStream2> = form_meta
        .buttons
        .iter()
        .map(|b| {
            let name = &b.name;
            let field_ty = match b.handler_kind() {
                HandlerKind::Validated => quote! { ::formoxus::form::Handler<#model_ident> },
                HandlerKind::Unchecked => quote! { ::formoxus::form::UncheckedHandler },
            };
            quote! { pub #name: #field_ty }
        })
        .collect();

    Ok(quote! {
        #vis struct #handlers_ident {
            #( #fields ),*
        }
    })
}

/// The `FormState` impl: the one place a `Form` and a `FieldSet` genuinely
/// diverge. The field-driven pieces come from the shared
/// `container::state_fragments`; everything else here — `Handlers`, the
/// `<form onsubmit>` wrapper, the button row — is Form-only. `FieldSet`'s
/// analog (`field_set::field_set_state_impl`) assembles the same fragments
/// into a bare, button-less render instead.
fn form_state_impl(
    form_meta: &FormMeta,
    field_metas: &[FieldMeta],
    providers_type: &TokenStream2,
) -> Result<TokenStream2, MacroError> {
    let state_ident = form_meta.state_struct_name();
    let struct_ident = &form_meta.ident;
    let handlers_ident = form_meta.handlers_struct_name();
    let title = form_meta.common.title.clone().map(|t| {
        quote! {
            h2 { class: "form-title", #t },
        }
    });

    let container::StateFragments {
        clear_all,
        assign_to_vars,
        let_required_fields,
        validate_model,
        render_calls,
        has_errors,
    } = container::state_fragments(form_meta, field_metas)?;
    let destructure_handlers = form_meta.tokens_for_destructure_handlers();
    let onsubmit = form_meta.tokens_for_onsubmit();
    let button_tokens = form_meta.tokens_for_buttons();

    Ok(quote! {
        impl ::formoxus::form::FormState for #state_ident {
            type Model = #struct_ident;
            type Handlers = #handlers_ident;
            type Providers = #providers_type;

            fn validate(&mut self) -> Option<#struct_ident> {
                #clear_all

                #assign_to_vars;

                #let_required_fields

                #validate_model
            }

            #has_errors

            fn render(
                data: ::formoxus::__private::dioxus::prelude::Store<Self>,
                handlers: #handlers_ident,
                providers: #providers_type,
            ) -> ::formoxus::__private::dioxus::prelude::Element {
                #[allow(unused_imports)]
                use ::formoxus::__private::component_scope::*;
                use ::formoxus::__private::dioxus::prelude::ReadableExt;
                #destructure_handlers
                ::formoxus::__private::dioxus::prelude::rsx! {
                    #title
                    form {
                        onsubmit: #onsubmit,
                        #(#render_calls)*
                        ::formoxus::widgets::FormErrors { errors: data.errors().cloned() },
                        #button_tokens,
                    }
                }
            }
        }
    })
}
