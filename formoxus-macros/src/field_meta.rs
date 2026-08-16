use darling::{self, FromField};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Type;

use crate::{error::MacroError, form_meta::FormMeta};

#[derive(Debug, FromField)]
#[darling(attributes(form), forward_attrs(serde), and_then = "Self::validate")]
pub struct FieldMeta {
    pub ident: Option<syn::Ident>,
    pub ty: Type,

    // attrs for all fields
    pub component: Option<syn::Path>,
    pub label: Option<String>,
    pub case: Option<syn::Path>,
}

impl FieldMeta {
    pub fn validate(self) -> darling::Result<Self> {
        // TODO
        // check for duplicate labels
        Ok(self)
    }

    pub fn to_decl(&self) -> TokenStream2 {
        let field_ident = self.field_ident();
        let inner_type = option_inner(&self.ty).unwrap_or(&self.ty);

        quote! {
            pub #field_ident: ::formoxus::fields::FormField<#inner_type>
        }
    }

    pub fn field_ident(&self) -> &syn::Ident {
        self.ident.as_ref().expect("All fields should have idents")
    }

    /// The `{ … }` rsx block that renders this field: the `#[form(component =
    /// …)]` override if given, else the value type's `DefaultWidget`.
    pub fn render_call(&self, form_meta: &FormMeta) -> TokenStream2 {
        let field_ident = self.field_ident();
        let inner_type = option_inner(&self.ty).unwrap_or(&self.ty);
        let required = option_inner(&self.ty).is_none();
        let label_tokens = match self.label.clone() {
            Some(label_name) => match self.case.clone() {
                Some(override_case) => quote! { ::formoxus::label_case::ToCase::to_case(#label_name, #override_case) },
                None => quote! { #label_name },
            },
            None => {
                let label_name = field_ident.to_string();
                let default_case = syn::parse_quote!(::formoxus::label_case::LabelCase::Title);
                let label_case = self
                    .case
                    .clone()
                    .unwrap_or_else(|| form_meta.label_case.clone().unwrap_or_else(|| default_case));
                quote! {
                    ::formoxus::label_case::ToCase::to_case(#label_name, #label_case)
                }
            },
        };

        let render_expr = match &self.component {
            Some(path) => quote! {
                <#path as ::formoxus::widgets::FieldWidget<#inner_type>>::render(
                    data.#field_ident().into(),
                    ::formoxus::widgets::FieldProps {
                        label: #label_tokens,
                        required: #required,
                        placeholder: None,
                    },
                )
            },
            None => quote! {
                ::formoxus::widgets::render_default(
                    data.#field_ident().into(),
                    ::formoxus::widgets::FieldProps {
                        label: #label_tokens,
                        required: #required,
                        placeholder: None,
                    },
                )
            },
        };

        quote! { { #render_expr } }
    }
}

pub(crate) trait FieldMetas {
    fn clear(&self) -> Result<TokenStream2, MacroError>;
    fn assign_to_vars(&self) -> Result<TokenStream2, MacroError>;
    fn let_required_fields(&self) -> Result<TokenStream2, MacroError>;
    fn validate_model(&self, model_ident: &syn::Ident) -> Result<TokenStream2, MacroError>;
    fn field_initializers(&self) -> TokenStream2;
    fn has_errors(&self) -> Result<TokenStream2, MacroError>;
    fn render_calls(&self, form_meta: &FormMeta) -> Vec<TokenStream2>;
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
            // Cross-field errors land on `self.errors` FIRST so the gate below sees
            // them; then one `has_errors()` gate covers form-level errors, a field-level
            // `Invalid` (the optional-invalid case, which still builds the model), and
            // validator errors on a `Valid` field.
            self.errors = ::formoxus::form::ValidateForm::validate_form(self, &model);
            if self.has_errors() {
                None
            } else {
                Some(model)
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
                        quote! { ::formoxus::fields::FormField::with_optional(model.#field_ident.clone()) }
                    }
                    None => {
                        quote! { ::formoxus::fields::FormField::with_value(model.#field_ident.clone()) }
                    }
                };
                quote! {
                    #field_ident: #initializer
                }
            })
            .collect();
        quote! { #( #inits ),* }
    }

    fn has_errors(&self) -> Result<TokenStream2, MacroError> {
        let has_error_calls: Vec<TokenStream2> = self
            .iter()
            .map(|f| {
                let field_ident = f.field_ident();
                quote! {
                    self.#field_ident.has_errors()
                }
            })
            .collect();
        Ok(quote! {
            fn has_errors(&self) -> bool {
                !self.errors.is_empty()
                    #( || #has_error_calls )*
            }
        })
    }

    fn render_calls(&self, form_meta: &FormMeta) -> Vec<TokenStream2> {
        self.iter()
            .map(|field| field.render_call(form_meta))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use darling::FromDeriveInput;
    use googletest::prelude::*;
    use syn::parse_quote;

    use super::FieldMeta;
    use crate::form_meta::FormMeta;

    /// Parses a whole `#[derive(Form)] struct ...` into a `FormMeta` — the
    /// same path `form.rs` uses for real, so this exercises darling's own
    /// parsing too. The `label_case`/`case` paths in the fixtures below don't
    /// need to resolve to anything real: `render_call` only splices them as
    /// tokens, it never resolves them, so any plausible-looking path works.
    fn parse_form(input: syn::DeriveInput) -> FormMeta {
        FormMeta::from_derive_input(&input).unwrap()
    }

    /// The first (only) field in a fixture parsed by `parse_form`.
    fn first_field(form_meta: &FormMeta) -> &FieldMeta {
        match &form_meta.data {
            darling::ast::Data::Struct(fields) => &fields.fields[0],
            darling::ast::Data::Enum(_) => unreachable!("fixtures are always structs"),
        }
    }

    #[gtest]
    fn field_level_case_wins_over_form_level_label_case() {
        let form_meta = parse_form(parse_quote! {
            #[form(label_case = test_case::FormLoses)]
            struct MyForm {
                #[form(case = test_case::FieldWins)]
                opt_flag: bool,
            }
        });

        let rendered = first_field(&form_meta).render_call(&form_meta).to_string();

        expect_that!(rendered, contains_substring("FieldWins"));
        expect_that!(rendered, not(contains_substring("FormLoses")));
    }

    #[gtest]
    fn custom_label_without_a_case_override_is_used_verbatim() {
        // Even though the form sets a `label_case`, an explicit `label` with
        // no `case` of its own is not run through it — the field author wrote
        // exactly what they want shown.
        let form_meta = parse_form(parse_quote! {
            #[form(label_case = test_case::Ignored)]
            struct MyForm {
                #[form(label = "Custom Label")]
                opt_flag: bool,
            }
        });

        let rendered = first_field(&form_meta).render_call(&form_meta).to_string();

        expect_that!(rendered, contains_substring("\"Custom Label\""));
        expect_that!(rendered, not(contains_substring("ToCase")));
        expect_that!(rendered, not(contains_substring("Ignored")));
    }

    #[gtest]
    fn custom_label_with_a_case_override_applies_it() {
        // Setting both on the same field is read as "yes, I do want the case
        // conversion applied to my custom label" — and it uses the field's
        // own `case`, not the form's `label_case`, even if both are set.
        let form_meta = parse_form(parse_quote! {
            #[form(label_case = test_case::FormLoses)]
            struct MyForm {
                #[form(label = "Custom Label", case = test_case::FieldWins)]
                opt_flag: bool,
            }
        });

        let rendered = first_field(&form_meta).render_call(&form_meta).to_string();

        expect_that!(rendered, contains_substring("ToCase :: to_case"));
        expect_that!(rendered, contains_substring("\"Custom Label\""));
        expect_that!(rendered, contains_substring("FieldWins"));
        expect_that!(rendered, not(contains_substring("FormLoses")));
    }

    #[gtest]
    fn form_level_label_case_applies_when_field_has_no_override() {
        let form_meta = parse_form(parse_quote! {
            #[form(label_case = test_case::FormWins)]
            struct MyForm {
                opt_flag: bool,
            }
        });

        let rendered = first_field(&form_meta).render_call(&form_meta).to_string();

        expect_that!(rendered, contains_substring("FormWins"));
    }

    #[gtest]
    fn defaults_to_title_case_when_neither_is_set() {
        let form_meta = parse_form(parse_quote! {
            struct MyForm {
                opt_flag: bool,
            }
        });

        let rendered = first_field(&form_meta).render_call(&form_meta).to_string();

        // Both the case applied AND the raw field ident it's applied to —
        // with no `#[form(label = …)]` override, the ident itself (not a
        // pre-title-cased version of it) is what gets passed to `to_case`.
        expect_that!(rendered, contains_substring("LabelCase :: Title"));
        expect_that!(rendered, contains_substring("\"opt_flag\""));
    }

    #[gtest]
    fn empty_string_label_override_is_used_verbatim_not_treated_as_absent() {
        // `label: Option<String>` — `Some("")` is a real (if odd) override, not
        // the same as no attribute at all. Guards against a future change that
        // treats an empty string as "unset" and falls through to the field
        // ident (which would render as `""` too here, quietly masking a
        // regression) or the case-conversion path (which an empty label with
        // no `case` override should never go through — see
        // `custom_label_without_a_case_override_is_used_verbatim`).
        let form_meta = parse_form(parse_quote! {
            struct MyForm {
                #[form(label = "")]
                opt_flag: bool,
            }
        });

        let rendered = first_field(&form_meta).render_call(&form_meta).to_string();

        expect_that!(rendered, contains_substring("\"\""));
        expect_that!(rendered, not(contains_substring("ToCase")));
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
