use dioxus::prelude::*;
use formoxus::{FieldProps, FieldWidget, FormError, FormField, UnsetBooleanSelect, render_default};
use serde::{Deserialize, Serialize};

pub async fn submit_sample(model: SampleModel) {
    println!("{model:?}");
}

/// The cleaned, validated output — what a successful submit produces. Required
/// fields are their real types; genuinely optional ones stay `Option`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SampleModel {
    pub text: String,
    pub count: i32,
    pub max: Option<i32>,
    pub flag: bool,
    pub opt_flag: Option<bool>,
}

/// The reactive, in-progress form state: the `Store` content *and* the
/// serializable wire type (it round-trips whole). Every field is a `FormField`;
/// `errors` holds form-level (cross-field) errors. This is exactly what
/// `#[derive(Form)]` will generate from a plain declaration like:
///
/// ```ignore
/// #[derive(Form)]
/// #[form(model = SampleModel, onsubmit = submit_sample)]
/// struct SampleForm {
///     text: String,
///     count: i32,
///     max: Option<i32>,          // optional
///     flag: bool,
///     opt_flag: Option<bool>,
/// }
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize, Store)]
pub struct SampleFormState {
    pub text: FormField<String>,
    pub count: FormField<i32>,
    pub max: FormField<i32>, // optional field — no `required`
    pub flag: FormField<bool>,
    pub opt_flag: FormField<bool>,

    pub errors: Vec<FormError>,
}

impl SampleFormState {
    /// Seed a form from an existing model (edit mode): `initial == value`.
    pub fn from_model(model: &SampleModel) -> Self {
        SampleFormState {
            text: FormField::with_value(model.text.clone()),
            count: FormField::with_value(model.count),
            max: FormField::with_optional(model.max),
            flag: FormField::with_value(model.flag),
            opt_flag: FormField::with_optional(model.opt_flag),
            errors: Vec::new(),
        }
    }

    /// The cross-field validator — the one piece the macro can't generate, so the
    /// user hand-writes it. Runs against the tentatively-cleaned model.
    fn validate_form(&self, model: &SampleModel) -> Vec<FormError> {
        match model.max {
            Some(max) if model.count > max => vec![FormError(
                "if max is set, it must be at least count".to_string(),
            )],
            _ => Vec::new(),
        }
    }

    /// Clean the form: stamp field/form errors **in place**, and yield the model
    /// iff everything passed. Errors live on the fields/form, never in the return
    /// type — so there's nothing to reconstruct on failure. This is the entity
    /// `#[derive(Form)]` generates.
    pub fn validate(&mut self) -> Option<SampleModel> {
        // Re-validate from scratch so errors don't pile up across passes.
        self.text.clear_errors();
        self.count.clear_errors();
        self.max.clear_errors();
        self.flag.clear_errors();
        self.opt_flag.clear_errors();
        self.errors.clear();

        // Field-level: required fields must have a value; the optional one passes
        // through. Each `required()` stamps its own error on a miss.
        let text = self.text.required();
        let count = self.count.required();
        let max = self.max.optional();
        let flag = self.flag.required();
        let opt_flag = self.opt_flag.optional();

        // If any required field is missing, its error is already attached — bail.
        let (Some(text), Some(count), Some(flag)) = (text, count, flag) else {
            return None;
        };

        // Build the tentative model, then run the cross-field validator.
        let model = SampleModel { text, count, max, flag, opt_flag };
        let form_errors = self.validate_form(&model);
        if !form_errors.is_empty() {
            self.errors = form_errors;
            return None;
        }
        Some(model)
    }
}

#[component]
pub fn SampleForm(data: Store<SampleFormState>) -> Element {
    rsx! {
        form {
            class: "form",
            { render_default(
                data.text().into(),
                FieldProps { label: "Text".into(), required: true, placeholder: None },
            ) }
            { render_default(
                data.count().into(),
                FieldProps { label: "Count".into(), required: true, placeholder: None },
            ) }
            { render_default(
                data.max().into(),
                FieldProps { label: "Max".into(), required: false, placeholder: None },
            ) }
            { render_default(
                data.flag().into(),
                FieldProps { label: "Flag".into(), required: true, placeholder: None },
            ) }
            { UnsetBooleanSelect::render(
                data.opt_flag().into(),
                FieldProps { label: "OptFlag".into(), required: false, placeholder: None },
            ) }
        }
    }
}

fn main() {
    // The whole form is plain data — validation runs with no Dioxus runtime,
    // which is exactly what makes forms testable outside the web harness.

    // 1. Empty form → every required field flags itself; the optional one doesn't.
    let mut form = SampleFormState::default();
    assert!(form.validate().is_none());
    assert_eq!(form.text.errors.len(), 1);
    assert_eq!(form.count.errors.len(), 1);
    assert_eq!(form.flag.errors.len(), 1);
    assert_eq!(form.max.errors.len(), 0);
    assert_eq!(form.opt_flag.errors.len(), 0);
    println!("1. empty  → invalid; text error = {:?}", form.text.errors);

    // 2. Filled + valid → a clean model, no errors.
    let mut form = SampleFormState::from_model(&SampleModel {
        text: "hello".to_string(),
        count: 3,
        max: Some(5),
        flag: true,
        opt_flag: None,
    });
    let model = form.validate().expect("should be valid");
    println!("2. filled → valid;   model = {model:?}");

    // 3. Cross-field failure (count > max) → a form-level error, no model.
    form.count.value = Some(10);
    assert!(form.validate().is_none());
    println!("3. 10>5   → invalid; form errors = {:?}", form.errors);
}
