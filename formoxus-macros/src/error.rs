use proc_macro2::TokenStream as TokenStream2;

#[derive(Debug)]
pub(crate) enum MacroError {
    Syn(syn::Error),
    Darling(darling::Error),
}

impl From<syn::Error> for MacroError {
    fn from(e: syn::Error) -> Self {
        Self::Syn(e)
    }
}

impl From<darling::Error> for MacroError {
    fn from(e: darling::Error) -> Self {
        Self::Darling(e)
    }
}

impl MacroError {
    pub(crate) fn into_tokens(self) -> TokenStream2 {
        match self {
            Self::Syn(e) => e.to_compile_error(),
            Self::Darling(e) => e.write_errors(),
        }
    }
}
