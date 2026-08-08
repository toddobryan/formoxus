use darling::{self, FromField};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Type;

use crate::error::MacroError;

#[derive(Debug, FromField)]
#[darling(attributes(form), forward_attrs(serde), and_then = "Self::validate")]
pub struct FieldMeta {
    pub ident: Option<syn::Ident>,
    pub ty: Type,
}

impl FieldMeta {
    pub fn validate(self) -> darling::Result<Self> {
        // TODO
        Ok(self)
    }

    pub fn to_decl(&self) -> TokenStream2 {
        let field_ident = self.field_ident();
        let inner_type = option_inner(&self.ty).unwrap_or(&self.ty);

        quote! {
            pub #field_ident: ::formoxus::FormField<#inner_type>
        }
    }

    pub fn field_ident(&self) -> &syn::Ident {
        self.ident.as_ref().expect("All fields should have idents")
    }
}

pub(crate) trait FieldMetas {
    fn clear(&self) -> Result<TokenStream2, MacroError>;
    fn assign_to_vars(&self) -> Result<TokenStream2, MacroError>;
    fn let_required_fields(&self) -> Result<TokenStream2, MacroError>;
    fn validate_model(&self, model_ident: &syn::Ident) -> Result<TokenStream2, MacroError>;
    fn field_initializers(&self) -> TokenStream2;
}

impl FieldMetas for [FieldMeta] {
    fn clear(&self) -> Result<TokenStream2, MacroError> {
        let clear_errors_for_fields = self.iter().map(|f| {
            let field_ident = f.field_ident();
            quote! {
                self.#field_ident.clear_errors();
            }
        });

        Ok(quote! {
            #( #clear_errors_for_fields )*
            self.errors.clear();
        })
    }

    fn assign_to_vars(&self) -> Result<TokenStream2, MacroError> {
        let assignments: Vec<TokenStream2> = self
            .iter()
            .map(|f| {
                let field_ident = f.field_ident();
                let optional_or_required = match option_inner(&f.ty) {
                    Some(_) => quote! { optional() },
                    None => quote! { required() },
                };
                quote! {
                    let #field_ident = self.#field_ident.#optional_or_required
                }
            })
            .collect();

        Ok(quote! {
            #( #assignments );*
        })
    }

    fn let_required_fields(&self) -> Result<TokenStream2, MacroError> {
        let reqs: Vec<&syn::Ident> = self
            .iter()
            .filter_map(|f| {
                let field_ident = f.field_ident();
                match option_inner(&f.ty) {
                    Some(_) => None,
                    None => Some(field_ident),
                }
            })
            .collect();
        let some_reqs = reqs.iter().map(|id| {
            quote! { Some(#id) }
        });

        Ok(quote! {
            let (#( #some_reqs ),* ) = ( #( #reqs ),* ) else {
                return None;
            };
        })
    }

    fn validate_model(&self, model_ident: &syn::Ident) -> Result<TokenStream2, MacroError> {
        let fields: Vec<&syn::Ident> = self.iter().map(|f| f.field_ident()).collect();
        Ok(quote! {
            let model = #model_ident { #( #fields ),* };
            let form_errors = self.validate_form(&model);
            if form_errors.is_empty() {
                Some(model)
            } else {
                self.errors = form_errors;
                None
            }
        })
    }

    fn field_initializers(&self) -> TokenStream2 {
        let inits: Vec<TokenStream2> = self
            .iter()
            .map(|f| {
                let field_ident = f.field_ident();
                let initializer = match option_inner(&f.ty) {
                    Some(_) => {
                        quote! { ::formoxus::FormField::with_optional(model.#field_ident.clone()) }
                    }
                    None => {
                        quote! { ::formoxus::FormField::with_value(model.#field_ident.clone()) }
                    }
                };
                quote! {
                    #field_ident: #initializer
                }
            })
            .collect();
        quote! { #( #inits ),* }
    }
}

fn option_inner(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    if type_path.qself.is_some() {
        return None; // e.g. `<X as Trait>::Option`
    }
    let segment = type_path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };
    match args.args.first() {
        Some(syn::GenericArgument::Type(inner)) if args.args.len() == 1 => Some(inner),
        _ => None,
    }
}
