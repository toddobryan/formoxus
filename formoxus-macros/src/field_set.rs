use darling::{FromDeriveInput, ast};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse2};

use crate::{
    container, error::MacroError, field_container_meta::FieldContainerMeta, field_meta::FieldMeta,
    field_set_meta::FieldSetMeta,
};

/// Entry point for `#[derive(FieldSet)]`.
pub fn derive_field_set(tokens: TokenStream2) -> TokenStream2 {
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
            "FieldSet can only be derived for structs",
        )));
    };

    let Fields::Named(_) = &data.fields else {
        return Err(MacroError::Syn(syn::Error::new_spanned(
            &data.fields,
            "Deriving FieldSet requires a struct with named fields",
        )));
    };

    let field_set_meta: FieldSetMeta = FieldSetMeta::from_derive_input(&input)?;
    let field_metas: &[FieldMeta] = match &field_set_meta.data {
        ast::Data::Struct(fields) => &fields.fields,
        ast::Data::Enum(_) => unreachable!("shape checked above: input is a struct"),
    };

    let field_set_impl = field_set_impl_for(&field_set_meta)?;

    let state_struct = container::state_struct(&field_set_meta, field_metas)?;

    let providers_decl = container::providers_decl(&field_set_meta, field_metas);
    let providers_struct = &providers_decl.struct_decl;

    let validate_form_impl = container::validate_form_impl(&field_set_meta)?;

    // no separate Model to derive
    let trait_impls: Option<TokenStream2> = if field_set_meta.common.model.is_none() {
        let from_model_impl = container::from_model_impl(&field_set_meta, field_metas)?;
        let field_set_state_impl =
            field_set_state_impl(&field_set_meta, field_metas, &providers_decl.providers_type)?;

        Some(quote! {
            #from_model_impl

            #field_set_state_impl
        })
    } else {
        None
    };

    Ok(quote! {
        #field_set_impl

        #state_struct

        #providers_struct

        #validate_form_impl

        #trait_impls
    })
}

fn field_set_impl_for(
    field_set_meta: &impl FieldContainerMeta,
) -> Result<TokenStream2, MacroError> {
    let field_set_ident = &field_set_meta.ident();
    let state_ident = &field_set_meta.state_struct_name();

    Ok(quote! {
        impl ::formoxus::form::FieldSet for #field_set_ident {
            type State = #state_ident;
        }
    })
}

/// The `FieldSetState` impl: same field-driven fragments a `Form` uses (via
/// the shared `container::state_fragments`), assembled with no `Handlers`, no
/// `<form>` wrapper, and no buttons — just the fields themselves, so an
/// embedding `Form` can splice this render bare or wrap it in its own
/// `fieldset { legend { ... } { ... } }`.
fn field_set_state_impl(
    field_set_meta: &FieldSetMeta,
    field_metas: &[FieldMeta],
    providers_type: &TokenStream2,
) -> Result<TokenStream2, MacroError> {
    let state_ident = field_set_meta.state_struct_name();
    let struct_ident = &field_set_meta.ident;
    let title = field_set_meta.common.title.clone().map(|t| {
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
    } = container::state_fragments(field_set_meta, field_metas)?;

    Ok(quote! {
        impl ::formoxus::form::FieldSetState for #state_ident {
            type Model = #struct_ident;
            type Providers = #providers_type;

            fn validate(&mut self) -> Option<#struct_ident> {
                #clear_all

                #assign_to_vars;

                #let_required_fields

                #validate_model
            }

            #has_errors

            fn render<L>(
                data: ::formoxus::__private::dioxus::prelude::Store<Self, L>,
                providers: #providers_type,
            ) -> ::formoxus::__private::dioxus::prelude::Element
            where
                L: ::formoxus::__private::dioxus::prelude::Writable<
                        Target = Self,
                        Storage = ::formoxus::__private::dioxus::prelude::UnsyncStorage,
                    > + Copy
                    + 'static,
            {
                #[allow(unused_imports)]
                use ::formoxus::__private::component_scope::*;
                use ::formoxus::__private::dioxus::prelude::ReadableExt;
                ::formoxus::__private::dioxus::prelude::rsx! {
                    #title
                    #(#render_calls)*
                    ::formoxus::widgets::FormErrors { errors: data.errors().cloned() },
                }
            }
        }
    })
}
