//! Codegen shared between `#[derive(Form)]` and `#[derive(FieldSet)]` — anything
//! that only needs the field list and what [`FieldContainerMeta`] exposes (ident,
//! vis, and the `model`/`title`/`validator`/`label_case` in `CommonMeta`).
//! Buttons, handlers, and the `<form onsubmit>` wrapper are `Form`-only and stay
//! in `form.rs`; the bare fields-only render is `FieldSet`-only and stays in
//! `field_set.rs`.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::{
    error::MacroError,
    field_container_meta::FieldContainerMeta,
    field_meta::{FieldMeta, FieldMetas},
};

/// The `Store`-derivable state struct: one `FormField<T>` per declared field,
/// plus the `errors: Vec<FormError>` every container gets.
pub(crate) fn state_struct(
    container: &impl FieldContainerMeta,
    field_metas: &[FieldMeta],
) -> Result<TokenStream2, MacroError> {
    let state_struct_name = container.state_struct_name();
    let vis = container.vis();

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
        #vis struct #state_struct_name {
            #( #field_entries, )*

            pub errors: Vec<::formoxus::error::FormError>
        }
    })
}

/// The generated `...Providers` struct and the type to bind `type Providers =
/// ...;` to. When no field needs a provided-data slot (no `#[form(component =
/// ..., provided)]` field, no embedded `#[form(field_set)]`), `providers_type`
/// is plain `()` and `struct_decl` is `None` — callers pass `()` at the render
/// call site, and the overwhelming majority of forms (no field sets, nothing
/// provided) never notice this mechanism exists.
pub(crate) struct ProvidersDecl {
    pub providers_type: TokenStream2,
    pub struct_decl: Option<TokenStream2>,
}

pub(crate) fn providers_decl(
    container: &impl FieldContainerMeta,
    field_metas: &[FieldMeta],
) -> ProvidersDecl {
    let slots: Vec<TokenStream2> = field_metas
        .iter()
        .filter_map(FieldMeta::providers_slot)
        .collect();

    if slots.is_empty() {
        return ProvidersDecl {
            providers_type: quote! { () },
            struct_decl: None,
        };
    }

    let providers_ident = container.providers_struct_name();
    let vis = container.vis();

    ProvidersDecl {
        providers_type: quote! { #providers_ident },
        // No `Debug`: a `Provider<C>` is an `Rc<dyn Fn() -> ...>`, and trait-object
        // `Fn`/`Future`s aren't `Debug` — same reason the generated `...Handlers`
        // struct derives nothing at all. `Clone` still works (`Rc` is `Clone`
        // regardless of what it points to), needed so a repeating group's
        // `render_call` can share one provider slot across every row.
        struct_decl: Some(quote! {
            #[derive(Clone)]
            #vis struct #providers_ident {
                #( #slots ),*
            }
        }),
    }
}

/// `FromModel<Model>` for the container's own struct as its Model (the
/// `#[form(model = ...)]`-absent case) — type-driven field seeding.
pub(crate) fn from_model_impl(
    container: &impl FieldContainerMeta,
    field_metas: &[FieldMeta],
) -> Result<TokenStream2, MacroError> {
    assert!(container.common().model.is_none());
    let model_ident = container.ident();
    let state_ident = container.state_struct_name();
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

/// `ValidateForm<Model>`, delegating to the container's `#[form(validator =
/// ...)]` (or an empty `Vec` when unset) — the one piece generated regardless
/// of whether the container declares a separate `model`.
pub(crate) fn validate_form_impl(
    container: &impl FieldContainerMeta,
) -> Result<TokenStream2, MacroError> {
    let model_ident = container
        .common()
        .model
        .as_ref()
        .unwrap_or(container.ident());
    let state_ident = container.state_struct_name();
    let func = match &container.common().validator {
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

/// The field-driven fragments both `FormState`'s and `FieldSetState`'s generated
/// `validate`/`render` assemble from — nothing here touches handlers or buttons,
/// so it's identical for a `Form` and a `FieldSet`.
pub(crate) struct StateFragments {
    pub clear_all: TokenStream2,
    pub assign_to_vars: TokenStream2,
    pub let_required_fields: TokenStream2,
    pub validate_model: TokenStream2,
    pub render_calls: Vec<TokenStream2>,
    pub has_errors: TokenStream2,
}

pub(crate) fn state_fragments(
    container: &impl FieldContainerMeta,
    field_metas: &[FieldMeta],
) -> Result<StateFragments, MacroError> {
    assert!(container.common().model.is_none());
    Ok(StateFragments {
        clear_all: field_metas.clear()?,
        assign_to_vars: field_metas.assign_to_vars()?,
        let_required_fields: field_metas.let_required_fields()?,
        validate_model: field_metas.validate_model(container.ident())?,
        render_calls: field_metas.render_calls(container),
        has_errors: field_metas.has_errors()?,
    })
}
