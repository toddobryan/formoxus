use dioxus::prelude::WritableExt;
use dioxus::stores::Store;

use crate::FormError;

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
}

pub trait FromModel<Model> {
    fn from_model(model: &Model) -> Self;
}

pub trait ValidateForm<Model> {
    fn validate_form(&self, model: &Model) -> Vec<FormError>;
}
