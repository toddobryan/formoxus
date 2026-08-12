use dioxus::prelude::{WritableExt, use_store};
use dioxus::stores::Store;

use crate::error::FormError;

pub trait FormStoreExt {
    type Model;
    fn validate(&self) -> Option<Self::Model>;
}

impl<S: FormState + 'static> FormStoreExt for Store<S> {
    type Model = S::Model;

    fn validate(&self) -> Option<S::Model> {
        // `Store` is a Copy handle to shared reactive state; copy it for a mutable
        // binding, then write through the copy (mutates the same underlying store).
        let mut store = *self;
        store.write().validate()
    }
}

pub trait Form: std::fmt::Debug {
    type State: FormState + Default + Clone + std::fmt::Debug + 'static;
}

pub trait FormState: FromModel<Self::Model> + ValidateForm<Self::Model> + std::fmt::Debug {
    type Model: std::fmt::Debug;
    fn validate(&mut self) -> Option<Self::Model>;
    fn has_errors(&self) -> bool;
}

pub trait FromModel<Model> {
    fn from_model(model: &Model) -> Self;
}

pub trait ValidateForm<Model> {
    fn validate_form(&self, model: &Model) -> Vec<FormError>;
}

/// Create an empty form (create mode): the reactive `Store` for `F`'s state,
/// seeded with defaults. Call it as `use_form::<LoginForm>()` — you name only
/// your form struct; the synthetic `…State` type stays hidden behind `F::State`.
pub fn use_form<F: Form>() -> Store<F::State> {
    use_store(|| F::State::default())
}

/// Create a form seeded from an existing model (edit mode): every field starts
/// with `initial == value`. The model type is `F`'s cleaned output — for a form
/// whose declaration *is* its model, that's the form struct itself.
pub fn use_form_from<F: Form>(model: <F::State as FormState>::Model) -> Store<F::State> {
    use_store(move || F::State::from_model(&model))
}
