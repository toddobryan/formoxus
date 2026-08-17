use darling::{self, FromDeriveInput, FromMeta, ast, util::{self, SpannedValue}};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::Error;

use crate::field_meta::FieldMeta;

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(form), forward_attrs(serde), and_then = "Self::validate")]
pub(crate) struct FormMeta {
    pub attrs: Vec<syn::Attribute>,
    pub ident: syn::Ident,
    pub vis: syn::Visibility,
    pub data: ast::Data<util::Ignored, FieldMeta>,

    pub model: Option<syn::Ident>,
    pub title: Option<String>,
    pub submit_text: Option<String>,
    #[darling(multiple, rename = "button")]
    pub buttons: Vec<SpannedValue<ButtonInfo>>,
    pub form_validator: Option<syn::Path>,
    pub label_case: Option<syn::Path>,
}

impl FormMeta {
    pub fn validate(self) -> darling::Result<Self> {
        // TODO
        // check that buttons other than Reset, Submit, and Cancel have text
        Ok(self)
    }

    pub fn state_struct_name(&self) -> syn::Ident {
        format_ident!("{}{}", self.ident.to_string(), "State")
    }

    pub fn tokens_for_buttons(&self) -> TokenStream2 {
        let (destructive, mut rest): (Vec<&SpannedValue<ButtonInfo>>, Vec<&SpannedValue<ButtonInfo>>) =
            self.buttons.iter().partition(|b| b.ty == ButtonType::Destructive);
        rest .sort_by_key(|b| b.ty.display_order());
        // TODO: add the default submit button that every form has
        let tokens: Vec<TokenStream2> = destructive.iter().chain(rest.iter())
            .map(|svb| svb.button_tokens(svb.span())).collect();

        quote! {
            div {
                class: "formoxus-buttons",
                #( #tokens )*
            }
        }
    } 
}

#[derive(Debug, FromMeta, PartialEq, Eq)]
pub(crate) struct ButtonInfo {
    #[darling(rename = "type")]
    pub ty: ButtonType,
    pub text: Option<String>,
    pub class: Option<String>,
}

impl ButtonInfo {
    pub fn button_tokens(&self, span: Span) -> TokenStream2 {
        let html_type = self.ty.html_type();
        let class = self.class();
        let text = self.text(span);

        quote! {
            button { r#type: #html_type, class: #class, #text }
        }
    }

    pub fn text(&self, span: Span) -> TokenStream2 {
        match (self.text.clone(), self.ty.default_text()) {
            (Some(t), _) => quote! { #t },
            (None, Some(t)) => quote! { #t },
            (None, None) => Error::new(span, "did not provide text for button without default text").to_compile_error(),
        }
    }

    pub fn class(&self) -> String {
        self.class.clone().unwrap_or_else(|| self.ty.default_class())
    }
}

#[derive(Debug, FromMeta, PartialEq, Eq)]
#[darling(rename_all = "snake_case")]
pub(crate) enum ButtonType {
    Destructive,
    Reset,
    Cancel,
    Button,
    Submit,
}

impl ButtonType {
    pub fn display_order(&self) -> u8 {
        match self {
            ButtonType::Destructive => 0,
            ButtonType::Reset => 10,
            ButtonType::Cancel => 11,
            ButtonType::Button => 12,
            ButtonType::Submit => 13,
        }
    }

    pub fn html_type(&self) -> String {
        match self {
            ButtonType::Destructive => "button",
            ButtonType::Reset => "reset",
            ButtonType::Cancel => "button",
            ButtonType::Button => "button",
            ButtonType::Submit => "submit",
        }.to_string()
    }

    pub fn default_class(&self) -> String {
        match self {
            ButtonType::Destructive => "danger",
            ButtonType::Reset => "outline danger",
            ButtonType::Cancel => "outline secondary",
            ButtonType::Button => "secondary",
            ButtonType::Submit => "primary",
        }.to_string()
    }

    fn default_text(&self) -> Option<String> {
        match self {
            ButtonType::Destructive => None,
            ButtonType::Reset => Some("Reset"),
            ButtonType::Cancel => Some("Cancel"),
            ButtonType::Button => None,
            ButtonType::Submit => Some("Submit"),
        }.map(|str| str.to_string())
    }
}

