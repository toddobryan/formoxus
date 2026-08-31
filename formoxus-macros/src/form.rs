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

    let from_model_impl = container::from_model_impl(&form_meta, field_metas)?;
    let form_state_impl = form_state_impl(&form_meta, field_metas, &providers_decl.providers_type)?;

    Ok(quote! {
        #form_impl

        #state_struct

        #handlers_struct

        #providers_struct

        #validate_form_impl

        #from_model_impl

        #form_state_impl
    })
}

fn form_impl_for(form_meta: &impl FieldContainerMeta) -> Result<TokenStream2, MacroError> {
    let form_ident = &form_meta.ident();
    let state_ident = &form_meta.state_struct_name();
    let (impl_generics, ty_generics, where_clause) = form_meta.generics().split_for_impl();

    Ok(quote! {
        impl #impl_generics ::formoxus::form::Form for #form_ident #ty_generics #where_clause {
            type State = #state_ident #ty_generics;
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
    let (impl_generics, ty_generics, where_clause) = form_meta.generics.split_for_impl();
    // `Handler<Model>` needs `Model` spelled out generics and all only when the
    // model *is* the form itself (the common case, `form_meta.common.model ==
    // None`) — a `#[form(model = ...)]` override names a concrete,
    // independently-declared type that isn't parameterized by the form's own
    // generics, so splicing `#ty_generics` onto it would be wrong.
    let model_ty = match &form_meta.common.model {
        Some(model_path) => quote! { #model_path },
        None => {
            let form_ident = &form_meta.ident;
            quote! { #form_ident #ty_generics }
        }
    };

    let fields: Vec<TokenStream2> = form_meta
        .buttons
        .iter()
        .map(|b| {
            let name = &b.name;
            let field_ty = match b.handler_kind() {
                HandlerKind::Validated => quote! { ::formoxus::form::Handler<#model_ty> },
                HandlerKind::Unchecked => quote! { ::formoxus::form::UncheckedHandler },
            };
            quote! { pub #name: #field_ty }
        })
        .collect();

    // The struct only needs the container's own generics when a button's
    // `Handler<Model>` actually mentions them — i.e. when there's no
    // `#[form(model = ...)]` override, so `Model` is `Self` (generic if the
    // container is). A model override names a fixed, independently-declared
    // type none of these fields reference at all — declaring the struct
    // generic anyway would leave `T` unused (E0392), and there's no ergonomic
    // way to make callers fill in a `PhantomData` field they'd never expect.
    Ok(if form_meta.common.model.is_none() {
        quote! {
            #vis struct #handlers_ident #impl_generics #where_clause {
                #( #fields ),*
            }
        }
    } else {
        quote! {
            #vis struct #handlers_ident {
                #( #fields ),*
            }
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
    let (impl_generics, ty_generics, where_clause) = form_meta.generics.split_for_impl();
    let model_ty = match &form_meta.common.model {
        Some(model_path) => quote! { #model_path },
        None => quote! { #struct_ident #ty_generics },
    };
    // Mirrors `form_handlers_struct`'s own condition: the struct it declared
    // is only generic when there's no model override.
    let handlers_ty = if form_meta.common.model.is_none() {
        quote! { #handlers_ident #ty_generics }
    } else {
        quote! { #handlers_ident }
    };
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
        impl #impl_generics ::formoxus::form::FormState for #state_ident #ty_generics #where_clause {
            type Model = #model_ty;
            type Handlers = #handlers_ty;
            type Providers = #providers_type;

            fn validate(&mut self) -> Option<#model_ty> {
                #clear_all

                #assign_to_vars;

                #let_required_fields

                #validate_model
            }

            #has_errors

            fn render(
                data: ::formoxus::__private::dioxus::prelude::Store<Self>,
                handlers: #handlers_ty,
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
