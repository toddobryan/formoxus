use darling::{self, FromDeriveInput, ast, util};
use quote::format_ident;

use crate::field_meta::FieldMeta;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(form), forward_attrs(serde), and_then = "Self::validate")]
pub(crate) struct FormMeta {
    pub attrs: Vec<syn::Attribute>,
    pub ident: syn::Ident,
    pub vis: syn::Visibility,
    pub data: ast::Data<util::Ignored, FieldMeta>,

    #[darling(default)]
    pub model: Option<syn::Ident>,
    #[darling(default)]
    pub form_validator: Option<syn::Path>,
}

impl FormMeta {
    pub fn validate(self) -> darling::Result<Self> {
        // TODO
        Ok(self)
    }

    pub fn state_struct_name(&self) -> syn::Ident {
        format_ident!("{}{}", self.ident.to_string(), "State")
    }
}
