use darling::{FromDeriveInput, ast};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse2};

use crate::{
    error::MacroError,
    field_meta::{FieldMeta, FieldMetas},
    form_meta::{FormMeta, HandlerKind},
};

/// Entry point for `#[derive(Form)]`. SKELETON: parses the input (so malformed
/// input still yields a clean compiler error) and emits nothing. Replace the
/// empty output with the generated state struct / `from_model` / `validate` /
/// render component described in the crate docs.
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

    let state_struct = form_state_struct(&form_meta, field_metas)?;

    let handlers_struct = form_handlers_struct(&form_meta)?;

    let validate_form_impl = validate_form_impl(&form_meta)?;

    // no separate Model to derive
    let trait_impls: Option<TokenStream2> = if form_meta.model.is_none() {
        let from_model_impl = from_model_impl(&form_meta, field_metas)?;
        let form_state_impl = form_state_impl(&form_meta, field_metas)?;

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

        #validate_form_impl

        #trait_impls
    })
}

fn form_impl_for(form_meta: &FormMeta) -> Result<TokenStream2, MacroError> {
    let form_ident = &form_meta.ident;
    let state_ident = &form_meta.state_struct_name();

    Ok(quote! {
        impl ::formoxus::form::Form for #form_ident {
            type State = #state_ident;
        }
    })
}

fn form_state_struct(
    form_meta: &FormMeta,
    field_metas: &[FieldMeta],
) -> Result<TokenStream2, MacroError> {
    let state_struct_name = form_meta.state_struct_name();
    let vis = &form_meta.vis;
    let attrs: &[syn::Attribute] = &form_meta.attrs;

    let field_entries: Vec<TokenStream2> = field_metas.iter().map(|fm| fm.to_decl()).collect();

    Ok(quote! {
        // Bring `dioxus_stores` into scope for the `Store` derive's bare paths
        // (see `formoxus::__private::store_scope`). Glob so repeated derives in one
        // module don't collide; emitted in the caller's module so field types resolve.
        #[allow(unused_imports)]
        use ::formoxus::__private::store_scope::*;

        #[derive(
            Clone,
            Debug,
            Default,
            ::serde::Serialize,
            ::serde::Deserialize,
            ::formoxus::__private::dioxus::prelude::Store,
        )]
        #( #attrs )*
        #vis struct #state_struct_name {
            #( #field_entries, )*

            pub errors: Vec<::formoxus::error::FormError>
        }
    })
}

/// One field per `#[form(button(...))]`, named after the button, typed as
/// `Handler<Model>` or `UncheckedHandler` per that button's resolved
/// `HandlerKind`. Callers construct this by hand at the `store.render(...)`
/// call site — see `formoxus::form::{handler, unchecked_handler}`.
fn form_handlers_struct(form_meta: &FormMeta) -> Result<TokenStream2, MacroError> {
    let handlers_ident = form_meta.handlers_struct_name();
    let vis = &form_meta.vis;
    let model_ident = form_meta.model.as_ref().unwrap_or(&form_meta.ident);

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

fn from_model_impl(
    form_meta: &FormMeta,
    field_metas: &[FieldMeta],
) -> Result<TokenStream2, MacroError> {
    assert!(form_meta.model.is_none());
    let model_ident = &form_meta.ident;
    let state_ident = form_meta.state_struct_name();
    let field_initializers = field_metas.field_initializers();

    Ok(quote! {
        impl ::formoxus::form::FromModel<#model_ident> for #state_ident {
            fn from_model(model: &#model_ident) -> Self {
                #state_ident {
                    #field_initializers,
                    errors: Vec::new(),
                }
            }
        }
    })
}

fn validate_form_impl(form_meta: &FormMeta) -> Result<TokenStream2, MacroError> {
    let model_ident = form_meta.model.as_ref().unwrap_or(&form_meta.ident);
    let state_ident = form_meta.state_struct_name();
    let func = match &form_meta.form_validator {
        Some(fn_path) => quote! { #fn_path(model) },
        None => quote! { Vec::new() },
    };

    Ok(quote! {
        impl ::formoxus::form::ValidateForm<#model_ident> for #state_ident {
            fn validate_form(&self, model: &#model_ident) -> Vec<::formoxus::error::FormError> {
                #func
            }
        }
    })
}

fn form_state_impl(
    form_meta: &FormMeta,
    field_metas: &[FieldMeta],
) -> Result<TokenStream2, MacroError> {
    assert!(form_meta.model.is_none());
    let state_ident = form_meta.state_struct_name();
    let struct_ident = &form_meta.ident;
    let handlers_ident = form_meta.handlers_struct_name();
    let title = form_meta.title.clone().map(|t| {
        quote! {
            h2 { class: "form-title", #t },
        }
    });

    let clear_all = field_metas.clear()?;
    let assign_to_vars = field_metas.assign_to_vars()?;
    let let_required_fields = field_metas.let_required_fields()?;
    let validate_model = field_metas.validate_model(&form_meta.ident)?;
    let render_calls = field_metas.render_calls(form_meta);
    let has_errors = field_metas.has_errors()?;
    let destructure_handlers = form_meta.tokens_for_destructure_handlers();
    let onsubmit = form_meta.tokens_for_onsubmit();
    let button_tokens = form_meta.tokens_for_buttons();

    Ok(quote! {
        impl ::formoxus::form::FormState for #state_ident {
            type Model = #struct_ident;
            type Handlers = #handlers_ident;

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
