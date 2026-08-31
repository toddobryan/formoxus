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
    let (impl_generics, ty_generics, where_clause) = container.generics().split_for_impl();

    let field_entries: Vec<TokenStream2> = field_metas.iter().map(|fm| fm.to_decl()).collect();
    let field_idents: Vec<&syn::Ident> = field_metas.iter().map(|fm| fm.field_ident()).collect();

    Ok(quote! {
        // Bring `dioxus_stores` into scope for the `Store` derive's bare paths
        // (see `formoxus::__private::store_scope`). Glob so repeated derives in one
        // module don't collide; emitted in the caller's module so field types resolve.
        #[allow(unused_imports)]
        use ::formoxus::__private::store_scope::*;

        // No `#[derive(Default)]`: for a generic container, it blanket-adds a
        // `T: Default` bound to the whole impl regardless of whether `T` is
        // actually used directly (rust-lang/rust#26925 — the same limitation
        // `fields.rs` already works around for `FieldValue<T>`/`FormField<T>`).
        // A field_set-typed field here is `<T as FieldSet>::State`, whose own
        // `Default` is already guaranteed by `FieldSet::State`'s bound — no
        // `T: Default` needed at all, so hand-writing it instead of deriving
        // avoids demanding a bound nothing here actually requires. (`Clone`/
        // `Debug`/`Serialize`/`Deserialize` don't have this problem: their
        // derives bound the field's own syntactic type rather than blanket-
        // bounding every generic parameter, so `<T as FieldSet>::State: Trait`
        // — already guaranteed — is all they ask for.)
        #[derive(
            Clone,
            Debug,
            ::serde::Serialize,
            ::serde::Deserialize,
            ::formoxus::__private::dioxus::prelude::Store,
        )]
        #vis struct #state_struct_name #impl_generics #where_clause {
            #( #field_entries, )*

            pub errors: Vec<::formoxus::error::FormError>
        }

        impl #impl_generics ::std::default::Default for #state_struct_name #ty_generics #where_clause {
            fn default() -> Self {
                Self {
                    #( #field_idents: ::std::default::Default::default(), )*
                    errors: ::std::vec::Vec::new(),
                }
            }
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
    let (impl_generics, ty_generics, where_clause) = container.generics().split_for_impl();
    let vis = container.vis();

    ProvidersDecl {
        providers_type: quote! { #providers_ident #ty_generics },
        // No `Debug`: a `Provider<C>` is an `Rc<dyn Fn() -> ...>`, and trait-object
        // `Fn`/`Future`s aren't `Debug` — same reason the generated `...Handlers`
        // struct derives nothing at all. `Clone` still works (`Rc` is `Clone`
        // regardless of what it points to), needed so a repeating group's
        // `render_call` can share one provider slot across every row.
        struct_decl: Some(quote! {
            #[derive(Clone)]
            #vis struct #providers_ident #impl_generics #where_clause {
                #( #slots ),*
            }
        }),
    }
}

/// `FromModel<Model>` — type-driven field seeding. `field_initializers`
/// reads fields directly off a local binding named `model` shaped like the
/// container's own declaration struct (`Self`'s fields ARE these field
/// names). In the common case `Model` already *is* Self, so `model` needs no
/// adjustment. With a `#[form(model = ...)]` override, `Model`'s fields don't
/// necessarily line up at all — `model` is first reseated to a `Self`-shaped
/// value via `Self: From<Model>` (the app author's job to implement; a
/// missing impl surfaces as a compile error naming exactly that), requiring
/// `Model: Clone` to get an owned value out of the `&Model` this fn receives.
pub(crate) fn from_model_impl(
    container: &impl FieldContainerMeta,
    field_metas: &[FieldMeta],
) -> Result<TokenStream2, MacroError> {
    let self_ident = container.ident();
    let state_ident = container.state_struct_name();
    let (impl_generics, ty_generics, where_clause) = container.generics().split_for_impl();
    let field_initializers = field_metas.field_initializers();

    let model_ty = match &container.common().model {
        Some(model_path) => quote! { #model_path },
        None => quote! { #self_ident #ty_generics },
    };
    let reseat_as_self = if container.common().model.is_some() {
        quote! {
            let model: #self_ident #ty_generics =
                ::std::convert::From::from(::std::clone::Clone::clone(model));
        }
    } else {
        quote! {}
    };

    Ok(quote! {
        impl #impl_generics ::formoxus::form::FromModel<#model_ty> for #state_ident #ty_generics #where_clause {
            fn from_model(model: &#model_ty) -> Self {
                #reseat_as_self
                Self {
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
    let state_ident = container.state_struct_name();
    let (impl_generics, ty_generics, where_clause) = container.generics().split_for_impl();
    // Same reasoning as `form_handlers_struct`'s `model_ty`: a `#[form(model =
    // ...)]` override names a concrete, independently-declared type that isn't
    // parameterized by the container's own generics, so `#ty_generics` only
    // belongs on the container's own name when the model *is* the container itself.
    let model_ty = match &container.common().model {
        Some(model_path) => quote! { #model_path },
        None => {
            let container_ident = container.ident();
            quote! { #container_ident #ty_generics }
        }
    };
    let func = match &container.common().validator {
        Some(fn_path) => quote! { #fn_path(model) },
        None => quote! { Vec::new() },
    };

    Ok(quote! {
        impl #impl_generics ::formoxus::form::ValidateForm<#model_ty> for #state_ident #ty_generics #where_clause {
            fn validate_form(&self, model: &#model_ty) -> Vec<::formoxus::error::FormError> {
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
    Ok(StateFragments {
        clear_all: field_metas.clear()?,
        assign_to_vars: field_metas.assign_to_vars()?,
        let_required_fields: field_metas.let_required_fields()?,
        validate_model: field_metas.validate_model(container)?,
        render_calls: field_metas.render_calls(container),
        has_errors: field_metas.has_errors()?,
    })
}
