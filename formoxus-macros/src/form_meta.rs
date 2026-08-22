use std::collections::HashSet;

use darling::{
    self, FromDeriveInput, FromMeta, ast,
    util::{self, SpannedValue},
};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

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
    #[darling(multiple, rename = "button")]
    pub buttons: Vec<SpannedValue<ButtonInfo>>,
    pub form_validator: Option<syn::Path>,
    pub label_case: Option<syn::Path>,
}

impl FormMeta {
    pub fn validate(self) -> darling::Result<Self> {
        let submit_count = self
            .buttons
            .iter()
            .filter(|b| b.ty == ButtonType::Submit)
            .count();
        if submit_count != 1 {
            return Err(darling::Error::custom(format!(
                "a form must declare exactly one `#[form(button(type = submit, ...))]` \
                 (found {submit_count}) — every button now needs an explicit handler, \
                 so the submit button is no longer synthesized automatically"
            ))
            .with_span(&self.ident));
        }

        let mut seen_names = HashSet::new();
        for b in self.buttons.iter() {
            if !seen_names.insert(b.name.to_string()) {
                return Err(darling::Error::custom(format!(
                    "duplicate button name `{}` — button names must be unique, they become \
                     both the `…Handlers` struct field and the local variable bound to it",
                    b.name
                ))
                .with_span(&b.span()));
            }
        }

        Ok(self)
    }

    pub fn state_struct_name(&self) -> syn::Ident {
        format_ident!("{}{}", self.ident.to_string(), "State")
    }

    pub fn handlers_struct_name(&self) -> syn::Ident {
        format_ident!("{}Handlers", self.ident)
    }

    /// The `LabelCase` path buttons fall back to when a button has no explicit
    /// `text`: the form's `label_case`, or `Title` if that's unset too. Buttons
    /// don't get their own `case` override — same casing source as fields, by
    /// design (see formoxus-roadmap memory).
    pub fn button_label_case(&self) -> syn::Path {
        self.label_case
            .clone()
            .unwrap_or_else(|| syn::parse_quote!(::formoxus::label_case::LabelCase::Title))
    }

    /// The single button with `type = submit` — `validate` guarantees exactly one.
    pub fn submit_button(&self) -> &SpannedValue<ButtonInfo> {
        self.buttons
            .iter()
            .find(|b| b.ty == ButtonType::Submit)
            .expect("FormMeta::validate ensures exactly one submit button")
    }

    /// The `onsubmit: ...` closure for the generated `<form>`: routes native
    /// form submission (typing Enter, or clicking the submit button) through
    /// the submit button's own handler, per its resolved `HandlerKind`.
    pub fn tokens_for_onsubmit(&self) -> TokenStream2 {
        let submit = self.submit_button();
        let name = &submit.name;
        let call = submit.handler_kind().call_tokens(name);

        quote! {
            move |e| {
                let #name = #name.clone();
                async move {
                    e.prevent_default();
                    #call
                }
            }
        }
    }

    /// Pulls each button's handler out of the `handlers` param into a local
    /// variable named after the button — what `tokens_for_onsubmit`'s and each
    /// button's `onclick`'s `move` closures capture.
    pub fn tokens_for_destructure_handlers(&self) -> TokenStream2 {
        let handlers_ident = self.handlers_struct_name();
        let names: Vec<&syn::Ident> = self.buttons.iter().map(|b| &b.name).collect();

        quote! {
            let #handlers_ident { #( #names ),* } = handlers;
        }
    }

    pub fn tokens_for_buttons(&self) -> TokenStream2 {
        let label_case = self.button_label_case();
        let (destructive, mut rest): (
            Vec<&SpannedValue<ButtonInfo>>,
            Vec<&SpannedValue<ButtonInfo>>,
        ) = self
            .buttons
            .iter()
            .partition(|b| b.ty == ButtonType::Destructive);
        rest.sort_by_key(|b| b.ty.display_order());

        let tokens: Vec<TokenStream2> = destructive
            .iter()
            .chain(rest.iter())
            .map(|svb| svb.button_tokens(&label_case))
            .collect();

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
    pub name: syn::Ident,
    #[darling(rename = "type")]
    pub ty: ButtonType,
    pub text: Option<String>,
    pub class: Option<String>,
    pub handler: Option<HandlerKind>,
}

impl ButtonInfo {
    pub fn handler_kind(&self) -> HandlerKind {
        self.handler
            .unwrap_or_else(|| self.ty.default_handler_kind())
    }

    /// The `<button>` element for this button. The submit button renders with
    /// no `onclick` of its own — native `type="submit"` already routes both a
    /// click on it *and* an Enter-to-submit in the form through the `<form>`'s
    /// `onsubmit` (see `FormMeta::tokens_for_onsubmit`); every other button
    /// wires its own click straight to its handler.
    pub fn button_tokens(&self, label_case: &syn::Path) -> TokenStream2 {
        let html_type = self.ty.html_type();
        let class = self.class();
        let text = self.text_tokens(label_case);

        if self.ty == ButtonType::Submit {
            quote! {
                button { r#type: #html_type, class: #class, { #text } }
            }
        } else {
            let name = &self.name;
            let call = self.handler_kind().call_tokens(name);
            quote! {
                button {
                    r#type: #html_type,
                    class: #class,
                    onclick: move |_| {
                        let #name = #name.clone();
                        async move {
                            #call
                        }
                    },
                    { #text }
                }
            }
        }
    }

    /// Explicit `text` wins; otherwise the button's `name`, run through the
    /// form's label casing — same fallback shape as an unlabeled field.
    pub fn text_tokens(&self, label_case: &syn::Path) -> TokenStream2 {
        match &self.text {
            Some(t) => quote! { #t },
            None => {
                let name = self.name.to_string();
                quote! { ::formoxus::label_case::ToCase::to_case(#name, #label_case) }
            }
        }
    }

    pub fn class(&self) -> String {
        self.class
            .clone()
            .unwrap_or_else(|| self.ty.default_class())
    }
}

#[derive(Debug, FromMeta, PartialEq, Eq, Clone, Copy)]
#[darling(rename_all = "snake_case")]
pub(crate) enum HandlerKind {
    /// Only call the handler if `FormStoreExt::validate` succeeds; the handler
    /// receives the validated `Model`.
    Validated,
    /// Call the handler unconditionally, with no model — `validate` never runs.
    Unchecked,
}

impl HandlerKind {
    /// The call expression for a button's `onclick`/the form's `onsubmit`,
    /// assuming a `data: Store<Self>` binding is in scope (true inside a
    /// generated `FormState::render`).
    pub fn call_tokens(&self, name: &syn::Ident) -> TokenStream2 {
        match self {
            HandlerKind::Validated => quote! {
                if let Some(model) = ::formoxus::form::FormStoreExt::validate(&data) {
                    #name(model).await;
                }
            },
            HandlerKind::Unchecked => quote! {
                #name().await;
            },
        }
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
        }
        .to_string()
    }

    pub fn default_class(&self) -> String {
        match self {
            ButtonType::Destructive => "danger",
            ButtonType::Reset => "outline danger",
            ButtonType::Cancel => "outline secondary",
            ButtonType::Button => "secondary",
            ButtonType::Submit => "primary",
        }
        .to_string()
    }

    /// `Submit`/`Button` want the model (a save, a preview); `Cancel`/`Reset`/
    /// `Destructive` are escape hatches that must work on an invalid form, so
    /// they default to skipping validation. Either can be overridden per-button
    /// via `#[form(button(..., handler = validated | unchecked))]`.
    pub fn default_handler_kind(&self) -> HandlerKind {
        match self {
            ButtonType::Submit | ButtonType::Button => HandlerKind::Validated,
            ButtonType::Cancel | ButtonType::Reset | ButtonType::Destructive => {
                HandlerKind::Unchecked
            }
        }
    }
}
