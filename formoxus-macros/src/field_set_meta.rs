use darling::{self, FromDeriveInput, FromMeta, ast, util};

pub(crate) use crate::field_container_meta::FieldContainerMeta;
use crate::field_meta::FieldMeta;

#[derive(Debug, FromMeta)]
pub(crate) struct CommonMeta {
    // `syn::Path`, not `syn::Ident`: a `#[form(model = QuestionData::TrueFalse)]`
    // needs to name a specific enum variant, not just a bare type — every use
    // site below just splices this into `quote!` (`#model_path`), which works
    // identically for a `Path` as it did for an `Ident`.
    pub model: Option<syn::Path>,
    pub title: Option<String>,
    pub validator: Option<syn::Path>,
    pub label_case: Option<syn::Path>,
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(field_set), and_then = "Self::validate")]
pub(crate) struct FieldSetMeta {
    pub ident: syn::Ident,
    pub vis: syn::Visibility,
    pub data: ast::Data<util::Ignored, FieldMeta>,
    pub generics: syn::Generics,

    #[darling(flatten)]
    pub common: CommonMeta,
}

impl FieldSetMeta {
    pub fn validate(self) -> darling::Result<Self> {
        // TODO: add validation
        Ok(self)
    }
}

impl FieldContainerMeta for FieldSetMeta {
    fn ident(&self) -> &syn::Ident {
        &self.ident
    }
    fn vis(&self) -> &syn::Visibility {
        &self.vis
    }
    fn common(&self) -> &CommonMeta {
        &self.common
    }
    fn generics(&self) -> &syn::Generics {
        &self.generics
    }
}
