use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use dioxus::core::Element;
use dioxus::prelude::{WritableExt, use_store};
use dioxus::stores::Store;

use crate::error::FormError;

/// A button handler that receives the form's validated `Model` — only called
/// once `FormStoreExt::validate` succeeds. `Rc`, not `Box`: the generated
/// `onclick`/`onsubmit` closures run on every click, and each run needs to move
/// an owned copy into a fresh `async move` block, so the handler itself must be
/// cheaply `Clone`. Single-threaded (WASM), so `Rc` over `Arc`.
pub type Handler<M> = Rc<dyn Fn(M) -> Pin<Box<dyn Future<Output = ()>>>>;

/// A button handler that runs unconditionally — no validation attempt, no
/// access to the model. For buttons like Cancel that must work even while the
/// form is invalid.
pub type UncheckedHandler = Rc<dyn Fn() -> Pin<Box<dyn Future<Output = ()>>>>;

/// Wrap a plain async closure as a [`Handler`], for a `#[form(button(...))]`
/// field on a generated `…Handlers` struct.
pub fn handler<M, F, Fut>(f: F) -> Handler<M>
where
    F: Fn(M) -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
{
    Rc::new(move |m| Box::pin(f(m)))
}

/// Wrap a plain async closure as an [`UncheckedHandler`].
pub fn unchecked_handler<F, Fut>(f: F) -> UncheckedHandler
where
    F: Fn() -> Fut + 'static,
    Fut: Future<Output = ()> + 'static,
{
    Rc::new(move || Box::pin(f()))
}

pub trait FormStoreExt {
    type Model;
    type Handlers;
    fn validate(&self) -> Option<Self::Model>;
    fn render(&self, handlers: Self::Handlers) -> Element;
}

impl<S: FormState + 'static> FormStoreExt for Store<S> {
    type Model = S::Model;
    type Handlers = S::Handlers;

    fn validate(&self) -> Option<S::Model> {
        // `Store` is a Copy handle to shared reactive state; copy it for a mutable
        // binding, then write through the copy (mutates the same underlying store).
        let mut store = *self;
        store.write().validate()
    }

    fn render(&self, handlers: Self::Handlers) -> Element {
        S::render(*self, handlers)
    }
}

pub trait Form: std::fmt::Debug {
    type State: FormState + Default + Clone + std::fmt::Debug + 'static;
}

pub trait FormState: FromModel<Self::Model> + ValidateForm<Self::Model> + std::fmt::Debug {
    type Model: std::fmt::Debug;
    /// The per-form `…Handlers` struct the derive generates: one field per
    /// `#[form(button(...))]`, each a [`Handler<Self::Model>`] or an
    /// [`UncheckedHandler`] depending on that button's resolved `HandlerKind`.
    type Handlers;
    fn validate(&mut self) -> Option<Self::Model>;
    fn has_errors(&self) -> bool;
    fn render(data: Store<Self>, handlers: Self::Handlers) -> Element
    where
        Self: Sized + 'static;
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
