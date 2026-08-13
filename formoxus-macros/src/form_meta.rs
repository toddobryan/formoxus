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

    /// The generated field-rendering component's name. Can't reuse the form
    /// struct's own name (`LoginForm`): once dioxus's `#[component]`/`Properties`
    /// codegen is in the mix, a `struct LoginForm` + `fn LoginForm` in the same
    /// module fails to compile (E0255) even though a plain struct/fn pair with
    /// matching names normally coexists fine (different namespaces) — so this
    /// gets its own name instead.
    pub fn fields_component_name(&self) -> syn::Ident {
        format_ident!("{}{}", self.ident.to_string(), "Fields")
    }
}
