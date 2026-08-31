use darling::{self, FromField, util::Flag};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Type;

use crate::{error::MacroError, field_container_meta::FieldContainerMeta};

#[derive(Debug, FromField)]
#[darling(attributes(form), and_then = "Self::validate")]
pub struct FieldMeta {
    pub ident: Option<syn::Ident>,
    pub ty: Type,

    // attrs for all fields
    pub field_set: Flag,
    pub component: Option<syn::Path>,
    pub label: Option<String>,
    pub case: Option<syn::Path>,
    pub provided: Flag,
}

impl FieldMeta {
    pub fn validate(self) -> darling::Result<Self> {
        // `errors` is the one field every generated state struct adds itself
        // (`form_state_struct`/`fieldset_state_struct`); a same-named user field
        // would silently collide with it on the wire (serde doesn't reject two
        // fields serializing to the same key — it only surfaces as a
        // `duplicate_field` error on deserialize, far from this mistake).
        if self.field_ident() == "errors" {
            return Err(darling::Error::custom(
                "a field can't be named `errors` — that name is reserved for the form-level \
                 errors formoxus adds to every generated state struct",
            )
            .with_span(self.field_ident()));
        }

        if self.provided.is_present() && self.component.is_none() {
            return Err(darling::Error::custom(
                "`provided` needs a `component` to provide to — add `#[form(component = \
                 SomeWidget, provided)]`",
            )
            .with_span(self.field_ident()));
        }

        // TODO
        // check for duplicate labels
        Ok(self)
    }

    pub fn to_decl(&self) -> TokenStream2 {
        let field_ident = self.field_ident();

        let field_type: TokenStream2 = if self.field_set.is_present() {
            let element_type = self.field_set_element_type();
            if self.is_field_set_list() {
                quote! {
                    ::std::vec::Vec<<#element_type as ::formoxus::form::FieldSet>::State>
                }
            } else {
                quote! {
                    <#element_type as ::formoxus::form::FieldSet>::State
                }
            }
        } else {
            let inner_type = option_inner(&self.ty).unwrap_or(&self.ty);
            quote! {
                ::formoxus::fields::FormField<#inner_type>
            }
        };

        quote! {
            pub #field_ident: #field_type
        }
    }

    pub fn field_ident(&self) -> &syn::Ident {
        self.ident.as_ref().expect("All fields should have idents")
    }

    /// The `FieldSet`-implementing type this `#[form(field_set)]` field wraps:
    /// for `common: Common` that's `Common` itself; for `options: Vec<Choice>`
    /// (a repeating group — formoxus's analog of a Django formset) that's
    /// `Choice`. Only meaningful when `field_set` is present.
    fn field_set_element_type(&self) -> &syn::Type {
        vec_inner(&self.ty).unwrap_or(&self.ty)
    }

    /// Whether a `#[form(field_set)]` field is a repeating group
    /// (`Vec<T>` where `T: FieldSet`) rather than a single nested `FieldSet`.
    fn is_field_set_list(&self) -> bool {
        self.field_set.is_present() && vec_inner(&self.ty).is_some()
    }

    /// A `#[form(component = ..., provided)]` field's `Provider<C>` payload
    /// type, projected from the widget's own `ProvidedWidget::Choices` rather
    /// than the field's value type — a `Ref<Source>` field's picker needs a
    /// `Vec<SourcePath>` of candidates, not a `Ref<Source>` itself. Only
    /// meaningful when `provided` is present (`validate()` ensures `component`
    /// is `Some` whenever it is).
    fn choices_type(&self) -> TokenStream2 {
        let inner_type = option_inner(&self.ty).unwrap_or(&self.ty);
        let component = self
            .component
            .as_ref()
            .expect("validate() ensures `provided` implies `component`");
        quote! { <#component as ::formoxus::widgets::ProvidedWidget<#inner_type>>::Choices }
    }

    /// The `pub field: Type` entry this field contributes to the generated
    /// `...Providers` struct, if any: present when the field itself needs
    /// externally-supplied data (`provided`), or nests a `FieldSet` (singular
    /// or repeating-group) whose own `Providers` needs a slot here — nested
    /// regardless of whether that field set happens to need any itself, since
    /// this macro invocation can't see the other derive's expansion to know.
    pub fn providers_slot(&self) -> Option<TokenStream2> {
        let field_ident = self.field_ident();
        if self.provided.is_present() {
            let choices_ty = self.choices_type();
            Some(quote! { pub #field_ident: ::formoxus::form::Provider<#choices_ty> })
        } else if self.field_set.is_present() {
            let element_type = self.field_set_element_type();
            Some(quote! {
                pub #field_ident: <<#element_type as ::formoxus::form::FieldSet>::State as ::formoxus::form::FieldSetState>::Providers
            })
        } else {
            None
        }
    }

    /// The `{ … }` rsx block that renders this field: the `#[form(component =
    /// …)]` override if given, else the value type's `DefaultWidget`.
    pub fn render_call(&self, container: &impl FieldContainerMeta) -> TokenStream2 {
        let field_ident = self.field_ident();

        // A field set has no single label/required-ness of its own — it renders
        // its own fields (and its own `FormErrors`) via its generated `render`,
        // spliced in bare (no `<fieldset>` wrapper) rather than routed through
        // `FieldWidget`/`render_default`.
        if self.is_field_set_list() {
            let element_type = self.field_set_element_type();
            quote! {
                div { class: "field-set-list",
                    // The row's own position is its only identity here — the
                    // underlying store addresses rows positionally (there's no
                    // separate stable id), so the `key` mirrors that exactly.
                    for (i , row) in data.#field_ident().iter().enumerate() {
                        div { key: "{i}", class: "field-set-list-row",
                            // The same provider(s) are shared across every row — a
                            // repeating group's rows source their choices from the
                            // same place (e.g. one shared list of sources), so this
                            // clones the slot rather than needing a per-row one.
                            { ::formoxus::form::FieldSetState::render(row, providers.#field_ident.clone()) }
                            button {
                                r#type: "button",
                                onclick: move |_| {
                                    let mut rows = data.#field_ident();
                                    rows.remove(i);
                                },
                                "Remove",
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        onclick: move |_| {
                            let mut rows = data.#field_ident();
                            rows.push(<#element_type as ::formoxus::form::FieldSet>::State::default());
                        },
                        "Add",
                    }
                }
            }
        } else if self.field_set.is_present() {
            quote! {
                { ::formoxus::form::FieldSetState::render(data.#field_ident(), providers.#field_ident) }
            }
        } else {
            let inner_type = option_inner(&self.ty).unwrap_or(&self.ty);
            let required = option_inner(&self.ty).is_none();
            let label_tokens = match self.label.clone() {
                Some(label_name) => match self.case.clone() {
                    Some(override_case) => {
                        quote! { ::formoxus::label_case::ToCase::to_case(#label_name, #override_case) }
                    }
                    None => quote! { #label_name },
                },
                None => {
                    let label_name = field_ident.to_string();
                    let default_case = syn::parse_quote!(::formoxus::label_case::LabelCase::Title);
                    let label_case = self.case.clone().unwrap_or_else(|| {
                        container
                            .common()
                            .label_case
                            .clone()
                            .unwrap_or_else(|| default_case)
                    });
                    quote! {
                        ::formoxus::label_case::ToCase::to_case(#label_name, #label_case)
                    }
                }
            };

            let render_expr = match &self.component {
                Some(path) if self.provided.is_present() => quote! {
                    <#path as ::formoxus::widgets::ProvidedWidget<#inner_type>>::render(
                        data.#field_ident().into(),
                        ::formoxus::widgets::FieldProps {
                            label: #label_tokens,
                            required: #required,
                            placeholder: None,
                        },
                        providers.#field_ident,
                    )
                },
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
}

pub(crate) trait FieldMetas {
    fn clear(&self) -> Result<TokenStream2, MacroError>;
    fn assign_to_vars(&self) -> Result<TokenStream2, MacroError>;
    fn let_required_fields(&self) -> Result<TokenStream2, MacroError>;
    fn validate_model(&self, container: &impl FieldContainerMeta) -> Result<TokenStream2, MacroError>;
    fn field_initializers(&self) -> TokenStream2;
    fn has_errors(&self) -> Result<TokenStream2, MacroError>;
    fn render_calls(&self, container: &impl FieldContainerMeta) -> Vec<TokenStream2>;
}

impl FieldMetas for [FieldMeta] {
    fn clear(&self) -> Result<TokenStream2, MacroError> {
        let clear_errors_for_fields = self.iter().filter_map(|f| {
            if f.field_set.is_present() {
                None
            } else {
                let field_ident = f.field_ident();
                Some(quote! {
                    self.#field_ident.clear_errors();
                })
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
                if f.is_field_set_list() {
                    quote! {
                        let #field_ident = {
                            // Eager `Vec<Option<_>>` first, THEN a second, pure
                            // `Option<Vec<_>>` pass — collecting straight from
                            // `.map(...)` into `Option<Vec<_>>` would short-circuit
                            // on the first `None`, skipping `validate()` (and the
                            // errors it stamps as a side effect) on every row after it.
                            let validated: ::std::vec::Vec<_> = self.#field_ident
                                .iter_mut()
                                .map(|row| ::formoxus::form::FieldSetState::validate(row))
                                .collect();
                            validated.into_iter().collect::<::std::option::Option<::std::vec::Vec<_>>>()
                        };
                    }
                } else if f.field_set.is_present() {
                    quote! {
                        let #field_ident = ::formoxus::form::FieldSetState::validate(&mut self.#field_ident);
                    }
                } else {
                    let optional_or_required = match option_inner(&f.ty) {
                        Some(_) => quote! { optional() },
                        None => quote! { required() },
                    };
                    quote! {
                        let #field_ident = self.#field_ident.#optional_or_required
                    }
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

    fn validate_model(&self, container: &impl FieldContainerMeta) -> Result<TokenStream2, MacroError> {
        let self_ident = container.ident();
        let fields: Vec<&syn::Ident> = self.iter().map(|f| f.field_ident()).collect();
        // `model` is always built as `Self` first, by name — that always
        // matches (Self's fields ARE these field names), override or not.
        // Only when `#[form(model = ...)]` names something else does it then
        // get converted via `.into()`, which requires the app to have
        // written `impl From<Self> for Model` — a compile error names the
        // missing conversion if they haven't.
        let convert_to_model = if container.common().model.is_some() {
            quote! { let model = ::std::convert::Into::into(model); }
        } else {
            quote! {}
        };
        Ok(quote! {
            let model = #self_ident { #( #fields ),* };
            #convert_to_model
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
                let initializer = if f.is_field_set_list() {
                    quote! {
                        model.#field_ident
                            .iter()
                            .map(|row| ::formoxus::form::FromModel::from_model(row))
                            .collect()
                    }
                } else if f.field_set.is_present() {
                    quote! { ::formoxus::form::FromModel::from_model(&model.#field_ident) }
                } else {
                    match option_inner(&f.ty) {
                        Some(_) => {
                            quote! { ::formoxus::fields::FormField::with_optional(model.#field_ident.clone()) }
                        }
                        None => {
                            quote! { ::formoxus::fields::FormField::with_value(model.#field_ident.clone()) }
                        }
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
                if f.is_field_set_list() {
                    quote! {
                        self.#field_ident
                            .iter()
                            .any(|row| ::formoxus::form::FieldSetState::has_errors(row))
                    }
                } else if f.field_set.is_present() {
                    quote! {
                        ::formoxus::form::FieldSetState::has_errors(&self.#field_ident)
                    }
                } else {
                    quote! {
                        self.#field_ident.has_errors()
                    }
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

    fn render_calls(&self, container: &impl FieldContainerMeta) -> Vec<TokenStream2> {
        self.iter()
            .map(|field| field.render_call(container))
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
            #[form(label_case = test_case::FormLoses, button(type = "submit", name = submit))]
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
            #[form(label_case = test_case::Ignored, button(type = "submit", name = submit))]
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
            #[form(label_case = test_case::FormLoses, button(type = "submit", name = submit))]
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
            #[form(label_case = test_case::FormWins, button(type = "submit", name = submit))]
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
            #[form(button(type = "submit", name = submit))]
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
            #[form(button(type = "submit", name = submit))]
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
    single_generic_arg(ty, "Option")
}

/// The `T` in `Vec<T>`, or `None` if `ty` isn't (syntactically) a bare `Vec<...>`.
/// A `#[form(field_set)]` field whose declared type has this shape is a
/// repeating group (formoxus's analog of a Django formset) rather than a
/// single nested `FieldSet`.
fn vec_inner(ty: &syn::Type) -> Option<&syn::Type> {
    single_generic_arg(ty, "Vec")
}

/// The sole generic argument of a bare `name<T>` path type (e.g. `Option<T>`,
/// `Vec<T>`), or `None` if `ty` doesn't syntactically match that shape (a
/// different type, more than one generic argument, or a qualified path like
/// `<X as Trait>::Option`).
fn single_generic_arg<'a>(ty: &'a syn::Type, name: &str) -> Option<&'a syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    if type_path.qself.is_some() {
        return None; // e.g. `<X as Trait>::Option`
    }
    let segment = type_path.path.segments.last()?;
    if segment.ident != name {
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
