use quote::format_ident;

use crate::field_set_meta::CommonMeta;

pub(crate) trait FieldContainerMeta {
    fn ident(&self) -> &syn::Ident;
    fn vis(&self) -> &syn::Visibility;
    fn common(&self) -> &CommonMeta;

    fn state_struct_name(&self) -> syn::Ident {
        format_ident!("{}State", self.ident())
    }

    fn providers_struct_name(&self) -> syn::Ident {
        format_ident!("{}Providers", self.ident())
    }

    fn generics(&self) -> &syn::Generics;
}
